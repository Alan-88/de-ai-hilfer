use crate::ai::is_hard_failure;
use crate::models::{FollowUp, FollowUpRequest, FollowUpResponse, StreamMetaPayload};
use crate::repositories::{agent_batches, follow_up, knowledge};
use crate::services::{
    agent_api,
    ai_model_resolver::{resolve_task_model_with_override, AiModelTask},
    follow_up_prompt::{build_follow_up_prompt, excerpt},
    stream_response::sse_meta,
};
use crate::state::AppState;
use anyhow::{anyhow, ensure, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

pub fn validate_text(question: &str, answer: Option<&str>) -> Result<()> {
    ensure!(
        !question.trim().is_empty() && question.chars().count() <= 4000,
        "question must contain 1–4000 characters"
    );
    if let Some(answer) = answer {
        ensure!(
            !answer.trim().is_empty() && answer.chars().count() <= 100000,
            "answer must contain 1–100000 characters"
        );
    }
    Ok(())
}

pub async fn get(state: &AppState, id: i64) -> Result<FollowUp> {
    follow_up::get(&state.pool, id)
        .await?
        .ok_or_else(|| anyhow!("follow-up not found"))
}

pub async fn save(
    state: &AppState,
    entry_id: i64,
    question: &str,
    answer: &str,
    idempotency_key: &str,
) -> Result<FollowUp> {
    validate_text(question, Some(answer))?;
    ensure!(
        !idempotency_key.trim().is_empty() && idempotency_key.len() <= 128,
        "idempotency_key must contain 1–128 bytes"
    );
    let key = format!("followup-save:{idempotency_key}");
    let request = json!({"entry_id":entry_id,"question":question.trim(),"answer":answer.trim()});
    let mut tx = state.pool.begin().await?;
    agent_batches::lock_operation(&mut tx, &key).await?;
    if let Some(receipt) = agent_batches::receipt(&mut tx, &key).await? {
        ensure!(
            receipt["request"] == request,
            "idempotency_key already belongs to a different request"
        );
        return Ok(serde_json::from_value(receipt["record"].clone())?);
    }
    ensure!(
        knowledge::find_by_id(&state.pool, entry_id)
            .await?
            .is_some(),
        "knowledge entry not found"
    );
    let saved = follow_up::insert(&mut tx, entry_id, question.trim(), answer.trim()).await?;
    agent_batches::save_receipt(&mut tx, &key, &json!({"request":request,"record":saved})).await?;
    tx.commit().await?;
    Ok(saved)
}

pub async fn edit(
    state: &AppState,
    id: i64,
    request: crate::models::FollowUpEditRequest,
) -> Result<FollowUp> {
    validate_text(&request.question, Some(&request.answer))?;
    let mut original = get(state, id).await?;
    original.question = request.expected_question;
    original.answer = request.expected_answer;
    let mut tx = state.pool.begin().await?;
    let saved = follow_up::replace(
        &mut tx,
        &original,
        request.question.trim(),
        request.answer.trim(),
    )
    .await?
    .ok_or_else(|| anyhow!("follow-up changed or deleted; refresh before editing"))?;
    tx.commit().await?;
    Ok(saved)
}

pub async fn create(state: &AppState, request: FollowUpRequest) -> Result<FollowUpResponse> {
    create_with_progress(state, request, None, None).await
}

pub async fn create_with_progress(
    state: &AppState,
    request: FollowUpRequest,
    progress: Option<&UnboundedSender<String>>,
    operation_key: Option<&str>,
) -> Result<FollowUpResponse> {
    validate_text(&request.question, None)?;
    ensure!(
        request.entry_id > 0 && request.retry_id.is_none_or(|id| id > 0),
        "IDs must be positive"
    );
    // A committed batch result is reused even after the record is subsequently edited or deleted.
    if let Some(key) = operation_key {
        let mut connection = state.pool.acquire().await?;
        if let Some(saved) = agent_batches::receipt(&mut connection, key).await? {
            return Ok(serde_json::from_value(saved)?);
        }
    }
    let entry = knowledge::find_by_id(&state.pool, request.entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;
    let original = if let Some(id) = request.retry_id {
        let original = get(state, id).await?;
        ensure!(
            original.entry_id == request.entry_id,
            "retry target belongs to another entry"
        );
        Some(original)
    } else {
        None
    };
    let history = follow_up::list_by_entry_id(&state.pool, request.entry_id)
        .await?
        .into_iter()
        .filter(|item| Some(item.id) != request.retry_id)
        .collect::<Vec<_>>();
    let prompt = build_follow_up_prompt(
        &state.prompts.follow_up_prompt,
        &request.question,
        &entry.analysis,
        &history,
    );
    let mut primary = resolve_task_model_with_override(
        state,
        AiModelTask::Analyze,
        request.model_override.as_ref(),
    )
    .await?;
    if !primary.persisted {
        primary.model = match request.quality_mode {
            crate::models::QualityMode::Default => state.config.ai_models.follow_up.clone(),
            crate::models::QualityMode::Pro => state.config.ai_models.follow_up_pro.clone(),
        };
    }
    let model = primary.model.clone();
    if let Some(tx) = progress {
        let _ = tx.send(sse_meta(&StreamMetaPayload {
            kind: "follow_up".into(),
            model: model.clone(),
            quality_mode: request.quality_mode,
            source: "只读关联查询".into(),
            fallback: false,
        }));
    }
    let answer = tokio::time::timeout(Duration::from_secs(180), async {
        let mut messages = vec![json!({"role":"system","content":prompt}),json!({"role":"user","content":request.question})];
        let tools = json!([{"type":"function","function":{"name":"lexeme_query","description":"Read-only dictionary lookup. Resolve a related word first, then inspect selected lexeme IDs for dictionary facts and saved analysis. Never initializes or writes entries.","parameters":{"type":"object","properties":{"action":{"type":"string","enum":["resolve","inspect"]},"surfaces":{"type":"array","items":{"type":"string"},"minItems":1,"maxItems":3},"lexeme_ids":{"type":"array","items":{"type":"integer","minimum":1},"minItems":1,"maxItems":3}},"required":["action"],"additionalProperties":false}}}]);
        let mut calls = 0;
        for round in 1..=5 {
            let started = std::time::Instant::now();
            tracing::info!(entry_id=request.entry_id, round, model=%model, "follow-up model turn started");
            let mut turn = primary.client.chat_with_tools(&model,&messages,&tools).await;
            if turn.as_ref().err().is_some_and(is_hard_failure) {
                tokio::time::sleep(Duration::from_millis(500)).await;
                turn = primary.client.chat_with_tools(&model,&messages,&tools).await;
            }
            let turn = turn.context("追问生成失败，未保存回答，请重试")?;
            tracing::info!(entry_id=request.entry_id, round, tool_calls=turn.tool_calls.len(), elapsed_ms=started.elapsed().as_millis(), "follow-up model turn completed");
            if turn.tool_calls.is_empty() {
                let answer = turn.content.unwrap_or_default().trim().to_string();
                validate_text(&request.question,Some(&answer))?;
                return Ok(answer);
            }
            calls += turn.tool_calls.len();
            ensure!(calls <= 6, "追问关联查询超过上限，未保存回答，请缩小问题后重试");
            // Preserve provider continuation metadata, including Gemini thought signatures.
            let mut assistant = serde_json::to_value(&turn)?;
            assistant["role"] = "assistant".into();
            messages.push(assistant);
            for call in turn.tool_calls {
                ensure!(call.kind == "function" && call.function.name == "lexeme_query", "追问请求了未开放的工具，未保存回答");
                let result = query_tool(state,&call.function.arguments).await;
                let content = match result {Ok(result)=>result,Err(error)=>json!({"error":error.to_string()})};
                messages.push(json!({"role":"tool","tool_call_id":call.id,"content":content.to_string()}));
            }
        }
        Err(anyhow!("追问关联查询未在限定轮数内完成，未保存回答，请重试"))
    }).await.map_err(|_|anyhow!("追问生成超时，未保存回答，请重试"))??;

    let mut tx = state.pool.begin().await?;
    if let Some(key) = operation_key {
        agent_batches::lock_operation(&mut tx, key).await?;
        if let Some(saved) = agent_batches::receipt(&mut tx, key).await? {
            return Ok(serde_json::from_value(saved)?);
        }
    }
    let saved = if let Some(original) = original {
        follow_up::replace(&mut tx, &original, request.question.trim(), &answer)
            .await?
            .ok_or_else(|| {
                anyhow!("follow-up changed or deleted during retry; original was not overwritten")
            })?
    } else {
        follow_up::insert(&mut tx, request.entry_id, request.question.trim(), &answer).await?
    };
    let response = FollowUpResponse {
        answer,
        follow_up: saved,
        model: Some(model),
        quality_mode: Some(request.quality_mode),
    };
    if let Some(key) = operation_key {
        agent_batches::save_receipt(&mut tx, key, &serde_json::to_value(&response)?).await?;
    }
    tx.commit().await?;
    Ok(response)
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum QueryTool {
    Resolve { surfaces: Vec<String> },
    Inspect { lexeme_ids: Vec<i64> },
}

async fn query_tool(state: &AppState, arguments: &str) -> Result<Value> {
    let query: QueryTool =
        serde_json::from_str(arguments).context("invalid lexeme_query arguments")?;
    let mut results = Vec::new();
    match query {
        QueryTool::Resolve { surfaces } => {
            ensure!(
                !surfaces.is_empty()
                    && surfaces.len() <= 3
                    && surfaces
                        .iter()
                        .all(|word| !word.trim().is_empty() && word.chars().count() <= 240),
                "resolve requires 1–3 words of up to 240 characters"
            );
            for surface in surfaces {
                results.push(serde_json::to_value(
                    agent_api::resolve_lexeme(state, &surface, 3).await?,
                )?);
            }
        }
        QueryTool::Inspect { lexeme_ids } => {
            ensure!(
                !lexeme_ids.is_empty()
                    && lexeme_ids.len() <= 3
                    && lexeme_ids.iter().all(|id| *id > 0),
                "inspect requires 1–3 positive IDs"
            );
            for id in lexeme_ids {
                let inspected = agent_api::inspect_lexeme(state, id).await?;
                let analysis = if let Some(id) = inspected.knowledge.entry_id {
                    knowledge::find_by_id(&state.pool, id)
                        .await?
                        .map(|entry| excerpt(&entry.analysis.to_string(), 16000))
                } else {
                    None
                };
                results.push(json!({"state":inspected,"saved_analysis":analysis}));
            }
        }
    }
    Ok(json!({"results":results}))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn validates_external_writes_and_limits_tool_actions() {
        assert!(validate_text(" ", Some("answer")).is_err());
        assert!(validate_text("question", Some(" ")).is_err());
        assert!(validate_text(&"字".repeat(4001), None).is_err());
        assert!(
            serde_json::from_str::<QueryTool>(r#"{"action":"initialize","lexeme_ids":[1]}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<QueryTool>(
            r#"{"action":"inspect","lexeme_ids":[1],"write":true}"#
        )
        .is_err());
    }

    pub async fn verify_agent_and_mutations(state: &AppState, entry: i64, lexeme: i64) {
        use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        assert_eq!(database, "deai_test");
        sqlx::query("INSERT INTO dictionary_surface_forms(surface,normalized_surface,lexeme_id,source) VALUES('Wahl','wahl',$1,'headword')").bind(lexeme).execute(&state.pool).await.unwrap();
        let mode = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let mock_mode = mode.clone();
        let mock_calls = calls.clone();
        let pool = state.pool.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let mock = Router::new().route("/chat/completions",post(move |Json(body):Json<Value>| {
            let mode=mock_mode.clone();let calls=mock_calls.clone();let pool=pool.clone();
            async move {
                let count = calls.fetch_add(1,Ordering::SeqCst);
                let mode=mode.load(Ordering::SeqCst);
                if mode==2 {return StatusCode::UNAUTHORIZED.into_response();}
                if mode==3 && count==0 {return StatusCode::SERVICE_UNAVAILABLE.into_response();}
                assert_eq!(body["tools"][0]["function"]["name"],"lexeme_query");
                assert_eq!(body["max_tokens"],2048);
                let messages=body["messages"].as_array().unwrap();
                let tool_count=messages.iter().filter(|message|message["role"]=="tool").count();
                if mode==5 || tool_count<2 {
                    let args=if mode==5 || tool_count==0 {json!({"action":"resolve","surfaces":["Wahl"]})} else {
                        let result:Value=serde_json::from_str(messages.last().unwrap()["content"].as_str().unwrap()).unwrap();
                        assert_eq!(result["results"][0]["candidates"][0]["lexeme_id"],lexeme);
                        json!({"action":"inspect","lexeme_ids":[lexeme]})
                    };
                    return Json(json!({"choices":[{"finish_reason":"tool_calls","message":{"content":null,"tool_calls":[{"id":format!("call-{tool_count}"),"type":"function","function":{"name":"lexeme_query","arguments":args.to_string()}}]}}]})).into_response();
                }
                let result:Value=serde_json::from_str(messages.last().unwrap()["content"].as_str().unwrap()).unwrap();
                assert_eq!(result["results"][0]["state"]["knowledge"]["entry_id"],entry);
                assert!(result["results"][0]["saved_analysis"].is_string());
                if mode==4 {sqlx::query("UPDATE follow_ups SET answer='用户期间编辑' WHERE entry_id=$1").bind(entry).execute(&pool).await.unwrap();}
                Json(json!({"choices":[{"finish_reason":if mode==1 {"length"} else {"stop"},"message":{"content":"**区别**\n\n- 已付款凭证\n- 应付账单\n\n例句保留。"}}]})).into_response()
            }
        }));
        let server = tokio::spawn(async move {
            axum::serve(listener, mock).await.unwrap();
        });
        let mut state = state.clone();
        state.ai_client = crate::ai::AiClient::new("isolated".into(), url);
        state.config.ai_models.analyze = "mock".into();
        let request = |id| FollowUpRequest {
            entry_id: entry,
            question: "这两个词有什么区别？".into(),
            retry_id: id,
            quality_mode: Default::default(),
            model_override: None,
        };
        let original = save(&state, entry, "旧问题", "旧回答...", "followup-test-save")
            .await
            .unwrap();
        let cached_entry = knowledge::find_by_id(&state.pool, entry)
            .await
            .unwrap()
            .unwrap();
        let analyze_request = || {
            serde_json::from_value::<crate::models::AnalyzeRequest>(
                json!({"query_text":cached_entry.query_text}),
            )
            .unwrap()
        };
        for response in [
            crate::services::analyze::analyze(&state, analyze_request())
                .await
                .unwrap(),
            crate::services::analyze::analyze_compact(&state, analyze_request())
                .await
                .unwrap(),
            crate::services::query::intelligent_search(
                &state,
                crate::models::IntelligentSearchRequest {
                    term: cached_entry.query_text.clone(),
                    hint: String::new(),
                },
            )
            .await
            .unwrap(),
        ] {
            assert_eq!(response.follow_ups.len(), 1);
            assert_eq!(response.follow_ups[0].id, original.id);
            assert_eq!(response.follow_ups[0].answer, original.answer);
        }
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        crate::services::analyze_stream::stream_analyze(state.clone(), analyze_request(), tx)
            .await
            .unwrap();
        let mut complete = None;
        while let Some(event) = rx.recv().await {
            if event.starts_with("event: complete") {
                complete = Some(
                    serde_json::from_str::<crate::models::AnalyzeResponse>(
                        event
                            .lines()
                            .find_map(|line| line.strip_prefix("data: "))
                            .unwrap(),
                    )
                    .unwrap(),
                );
            }
        }
        assert_eq!(complete.unwrap().follow_ups[0].id, original.id);
        assert_eq!(
            crate::services::library::get_entry_detail(&state, entry)
                .await
                .unwrap()
                .follow_ups[0]
                .id,
            original.id
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "loading saved follow-ups must not call the model"
        );
        assert_eq!(
            save(&state, entry, "旧问题", "旧回答...", "followup-test-save")
                .await
                .unwrap()
                .id,
            original.id
        );
        assert!(
            save(&state, entry, "不同问题", "旧回答...", "followup-test-save")
                .await
                .is_err()
        );
        mode.store(1, Ordering::SeqCst);
        assert!(create(&state, request(Some(original.id))).await.is_err());
        assert_eq!(get(&state, original.id).await.unwrap().answer, "旧回答...");
        mode.store(2, Ordering::SeqCst);
        assert!(create(&state, request(None)).await.is_err());
        assert_eq!(
            follow_up::list_by_entry_id(&state.pool, entry)
                .await
                .unwrap()
                .len(),
            1
        );
        mode.store(3, Ordering::SeqCst);
        calls.store(0, Ordering::SeqCst);
        let replaced = create(&state, request(Some(original.id))).await.unwrap();
        assert_eq!(replaced.follow_up.id, original.id);
        assert!(replaced.answer.contains("\n\n-"));
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        let generated = create_with_progress(
            &state,
            request(None),
            None,
            Some("followup-generation-test"),
        )
        .await
        .unwrap();
        let before = calls.load(Ordering::SeqCst);
        assert_eq!(
            create_with_progress(
                &state,
                request(None),
                None,
                Some("followup-generation-test")
            )
            .await
            .unwrap()
            .follow_up
            .id,
            generated.follow_up.id
        );
        assert_eq!(calls.load(Ordering::SeqCst), before);
        mode.store(4, Ordering::SeqCst);
        assert!(create(&state, request(Some(original.id)))
            .await
            .unwrap_err()
            .to_string()
            .contains("changed or deleted"));
        assert_eq!(
            get(&state, original.id).await.unwrap().answer,
            "用户期间编辑"
        );
        assert!(edit(
            &state,
            original.id,
            crate::models::FollowUpEditRequest {
                question: "编辑".into(),
                answer: "新版".into(),
                expected_question: "旧问题".into(),
                expected_answer: "旧回答...".into()
            }
        )
        .await
        .is_err());
        let current = get(&state, original.id).await.unwrap();
        let edited = edit(
            &state,
            current.id,
            crate::models::FollowUpEditRequest {
                question: "编辑".into(),
                answer: "外部回答\n\n- 保留排版".into(),
                expected_question: current.question,
                expected_answer: current.answer,
            },
        )
        .await
        .unwrap();
        assert!(edited.answer.contains("\n\n-"));
        mode.store(5, Ordering::SeqCst);
        let before = calls.load(Ordering::SeqCst);
        assert!(create(&state, request(None)).await.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst) - before,
            5,
            "tool loops stop at the turn budget"
        );
        assert!(follow_up::delete(&state.pool, original.id).await.unwrap());
        assert!(get(&state, original.id).await.is_err());
        server.abort();
    }
}
