use crate::batch_models::{
    BatchId, BatchJob, BatchOperation, BatchRequest, BatchStepState, FailurePolicy, TargetType,
};
use crate::repositories::agent_batches as repository;
use crate::{
    services::{agent_api, reading},
    state::AppState,
};
use anyhow::{anyhow, ensure, Result};
use futures_util::{stream, StreamExt};
use serde_json::{json, Value};
use sqlx::Connection;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::task::JoinSet;

pub fn validate(request: &BatchRequest) -> Result<()> {
    ensure!(
        !request.idempotency_key.trim().is_empty() && request.idempotency_key.len() <= 128,
        "idempotency_key must contain 1–128 bytes"
    );
    ensure!(
        !request.operations.is_empty() && request.operations.len() <= 100,
        "operations must contain 1–100 steps"
    );
    ensure!(
        serde_json::to_vec(request)?.len() <= 1_000_000,
        "batch payload is too large"
    );
    let mut prior = HashSet::new();
    for step in &request.operations {
        ensure!(
            !step.id.is_empty()
                && step.id.len() <= 64
                && step
                    .id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
            "invalid step id"
        );
        ensure!(!prior.contains(&step.id), "duplicate step id: {}", step.id);
        for dependency in &step.depends_on {
            ensure!(
                prior.contains(dependency),
                "dependencies must refer to earlier steps: {dependency}"
            );
        }
        let check_id = |id: &BatchId| -> Result<()> {
            match id {
                BatchId::Id(id) => ensure!(*id > 0, "IDs must be positive"),
                BatchId::Reference { step, path } => {
                    ensure!(
                        prior.contains(step),
                        "references must refer to earlier steps: {step}"
                    );
                    ensure!(
                        path.starts_with('/') && path.len() <= 256,
                        "reference path must be a JSON pointer"
                    );
                }
            }
            Ok(())
        };
        match &step.operation {
            BatchOperation::LexemeInitialize {
                lexeme_ids,
                model_override,
            } => {
                ensure!(
                    !lexeme_ids.is_empty()
                        && lexeme_ids.len() <= 100
                        && lexeme_ids.iter().all(|id| *id > 0),
                    "lexeme_ids must contain 1–100 positive IDs"
                );
                if let Some(model) = model_override {
                    ensure!(
                        !model.provider_name.trim().is_empty() && !model.model_id.trim().is_empty(),
                        "model override requires provider_name and model_id"
                    );
                }
            }
            BatchOperation::LearningMembership { ids, .. } => {
                ensure!(
                    !ids.is_empty() && ids.len() <= 100,
                    "ids must contain 1–100 IDs"
                );
                for id in ids {
                    check_id(id)?;
                }
            }
            BatchOperation::FolderCreate { name, parent_id } => {
                ensure!(
                    !name.trim().is_empty() && name.chars().count() <= 240,
                    "folder name must contain 1–240 characters"
                );
                if let Some(id) = parent_id {
                    check_id(id)?;
                }
            }
            BatchOperation::ArticleImport {
                title,
                body,
                folder_id,
            } => {
                ensure!(
                    !title.trim().is_empty()
                        && title.chars().count() <= 240
                        && !body.trim().is_empty(),
                    "article title and body are required (title up to 240 characters)"
                );
                if let Some(id) = folder_id {
                    check_id(id)?;
                }
            }
        }
        prior.insert(step.id.clone());
    }
    Ok(())
}

pub async fn submit(state: &AppState, request: BatchRequest) -> Result<BatchJob> {
    validate(&request)?;
    let steps = request
        .operations
        .iter()
        .map(|step| BatchStepState {
            id: step.id.clone(),
            status: "pending".into(),
            attempts: 0,
            result: None,
            error: None,
        })
        .collect::<Vec<_>>();
    let id = repository::submit(
        &state.pool,
        &uuid::Uuid::new_v4().to_string(),
        &request.idempotency_key,
        &serde_json::to_value(&request)?,
        &serde_json::to_value(steps)?,
    )
    .await?;
    repository::get(&state.pool, &id).await
}

pub async fn retry(state: &AppState, id: &str) -> Result<BatchJob> {
    let mut tx = state.pool.begin().await?;
    let row: (String, Value) =
        sqlx::query_as("SELECT status,steps FROM agent_batches WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("batch not found"))?;
    ensure!(
        row.0 != "queued" && row.0 != "running",
        "batch is still active"
    );
    let mut steps: Vec<BatchStepState> = serde_json::from_value(row.1)?;
    for step in &mut steps {
        if step.status != "succeeded" {
            step.status = "pending".into();
            step.error = None;
        }
    }
    sqlx::query(
        "UPDATE agent_batches SET status='queued',steps=$2,error=NULL,updated_at=NOW() WHERE id=$1",
    )
    .bind(id)
    .bind(serde_json::to_value(steps)?)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    repository::get(&state.pool, id).await
}

pub fn start_worker(state: AppState) {
    tokio::spawn(async move {
        let mut tasks = JoinSet::new();
        let mut active = HashSet::new();
        let mut cleanup = tokio::time::interval(Duration::from_secs(60));
        let mut poll = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = poll.tick() => match repository::pending(&state.pool).await {
                    Ok(ids) => for id in ids {
                        if active.len() >= 3 { break; }
                        if !active.insert(id.clone()) { continue; }
                        let state = state.clone();
                        tasks.spawn(async move { let result = execute_job(&state, &id).await; (id,result) });
                    },
                    Err(error) => tracing::error!("batch worker poll failed: {error}"),
                },
                _ = cleanup.tick() => {
                    if let Err(error) = crate::repositories::analysis_checkpoints::cleanup(&state.pool).await {
                        tracing::warn!("checkpoint cleanup failed: {error}");
                    }
                },
                result = tasks.join_next(), if !tasks.is_empty() => match result {
                    Some(Ok((id, result))) => { active.remove(&id); if let Err(error) = result { tracing::error!("batch {id} interrupted: {error:#}"); } },
                    Some(Err(error)) => { tasks.abort_all(); active.clear(); tracing::error!("batch worker panicked: {error}"); },
                    None => {},
                },
            }
        }
    });
}

async fn execute_job(state: &AppState, id: &str) -> Result<()> {
    // A dedicated connection releases the session lock on drop, including cancellation.
    let mut lock = sqlx::PgConnection::connect(&state.config.database_url).await?;
    let available: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_lock_shared(hashtextextended('database-snapshot',0))",
    )
    .fetch_one(&mut lock)
    .await?;
    if !available {
        return Ok(());
    }
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtextextended($1,0))")
        .bind(format!("batch:{id}"))
        .fetch_one(&mut lock)
        .await?;
    if !acquired {
        return Ok(());
    }
    let job = repository::get(&state.pool, id).await?;
    if job.status != "queued" && job.status != "running" {
        return Ok(());
    }
    let request: BatchRequest =
        serde_json::from_value(repository::request(&state.pool, id).await?)?;
    let mut steps: Vec<BatchStepState> = serde_json::from_value(job.steps)?;
    repository::update(
        &state.pool,
        id,
        "running",
        &serde_json::to_value(&steps)?,
        None,
    )
    .await?;
    for (index, step) in request.operations.iter().enumerate() {
        if steps[index].status == "succeeded" {
            continue;
        }
        let prior = steps[..index]
            .iter()
            .map(|step| (step.id.clone(), step.clone()))
            .collect::<HashMap<_, _>>();
        let stopped = request.on_error == FailurePolicy::Stop
            && steps[..index].iter().any(|s| s.status == "failed");
        let missing = step
            .depends_on
            .iter()
            .any(|dependency| prior[dependency].status != "succeeded");
        let references = match &step.operation {
            BatchOperation::LearningMembership { ids, .. } => ids.iter().collect::<Vec<_>>(),
            BatchOperation::FolderCreate { parent_id, .. } => parent_id.iter().collect(),
            BatchOperation::ArticleImport { folder_id, .. } => folder_id.iter().collect(),
            _ => Vec::new(),
        };
        let unavailable_reference = references
            .iter()
            .any(|id| matches!(id, BatchId::Reference { .. }) && resolve_id(id, &prior).is_err());
        if stopped || missing || unavailable_reference {
            steps[index].status = "skipped".into();
            steps[index].error = Some(
                if stopped {
                    "stopped after earlier failure"
                } else if missing {
                    "dependency did not succeed"
                } else {
                    "referenced result is unavailable"
                }
                .into(),
            );
        } else {
            steps[index].status = "running".into();
            steps[index].attempts += 1;
            repository::update(
                &state.pool,
                id,
                "running",
                &serde_json::to_value(&steps)?,
                None,
            )
            .await?;
            let previous = steps[index].result.clone();
            match execute_operation(
                state,
                &format!("{id}:{}", step.id),
                &step.operation,
                &prior,
                previous,
                (id, index),
            )
            .await
            {
                Ok(value) => {
                    let failed = value
                        .get("results")
                        .and_then(Value::as_array)
                        .is_some_and(|items| items.iter().any(|item| item["ok"] == false));
                    steps[index].status = if failed { "failed" } else { "succeeded" }.into();
                    steps[index].error =
                        failed.then(|| "one or more items failed; inspect results".into());
                    steps[index].result = Some(value);
                }
                Err(error) => {
                    steps[index].status = "failed".into();
                    steps[index].error = Some(format!("{error:#}"));
                }
            }
        }
        repository::update(
            &state.pool,
            id,
            "running",
            &serde_json::to_value(&steps)?,
            None,
        )
        .await?;
    }
    let status = if steps.iter().all(|s| s.status == "succeeded") {
        "succeeded"
    } else {
        "completed_with_errors"
    };
    repository::update(&state.pool, id, status, &serde_json::to_value(steps)?, None).await?;
    Ok(())
}

fn resolve_id(id: &BatchId, prior: &HashMap<String, BatchStepState>) -> Result<i64> {
    let value = match id {
        BatchId::Id(id) => Some(*id),
        BatchId::Reference { step, path } => prior
            .get(step)
            .and_then(|step| step.result.as_ref())
            .and_then(|result| result.pointer(path))
            .and_then(Value::as_i64),
    };
    value
        .filter(|id| *id > 0)
        .ok_or_else(|| anyhow!("reference does not resolve to a successful positive ID"))
}

async fn execute_operation(
    state: &AppState,
    key: &str,
    operation: &BatchOperation,
    prior: &HashMap<String, BatchStepState>,
    previous: Option<Value>,
    progress: (&str, usize),
) -> Result<Value> {
    match operation {
        BatchOperation::LexemeInitialize {
            lexeme_ids,
            model_override,
        } => {
            let saved = previous.as_ref().and_then(|v| v["results"].as_array());
            let initial = lexeme_ids
                .iter()
                .enumerate()
                .map(|(index, id)| {
                    saved
                        .and_then(|items| items.get(index))
                        .filter(|item| item["input"] == *id && item["ok"] == true)
                        .cloned()
                        .unwrap_or_else(|| json!({"input":id,"status":"pending"}))
                })
                .collect::<Vec<_>>();
            repository::step_result(
                &state.pool,
                progress.0,
                progress.1,
                &json!({"results":initial}),
            )
            .await?;
            let results = stream::iter(lexeme_ids.iter().copied().enumerate().map(|(index,id)| {
                let model = model_override.clone();
                let completed = saved.and_then(|items| items.get(index)).filter(|item| item["ok"] == true && item["input"] == id).cloned();
                async move {
                    if let Some(value) = completed { return Ok((index, value)); }
                    repository::item_result(&state.pool,progress.0,progress.1,index,&json!({"input":id,"status":"running"})).await?;
                    let result = agent_api::initialize_lexeme(state, id, model).await;
                    let value = match result {
                        Ok(response) => json!({"input":id,"ok":true,"value":{"entry_id":response.entry_id,"query_text":response.query_text,"source":response.source}}),
                        Err(error) => json!({"input":id,"ok":false,"error":format!("{error:#}")}),
                    };
                    repository::item_result(&state.pool,progress.0,progress.1,index,&value).await?;
                    Ok::<_,anyhow::Error>((index, value))
                }
            })).buffer_unordered(3).collect::<Vec<_>>().await;
            let mut results = results.into_iter().collect::<Result<Vec<_>>>()?;
            results.sort_by_key(|(index, _)| *index);
            Ok(json!({"results": results.into_iter().map(|(_,value)| value).collect::<Vec<_>>()}))
        }
        BatchOperation::LearningMembership {
            mutation,
            target_type,
            ids,
        } => {
            let mut results = Vec::new();
            for (index, id) in ids.iter().enumerate() {
                let result = async {
                    let mut id = resolve_id(id, prior)?;
                    if matches!(target_type, TargetType::Lexeme) {
                        id = agent_api::inspect_lexeme(state, id)
                            .await?
                            .knowledge
                            .entry_id
                            .ok_or_else(|| anyhow!("lexeme is not initialized"))?;
                    }
                    agent_api::membership_once(state, &format!("{key}:{index}"), *mutation, id)
                        .await
                }
                .await;
                results.push(match result {
                    Ok(value) => json!({"input":id,"ok":true,"value":value}),
                    Err(error) => json!({"input":id,"ok":false,"error":format!("{error:#}")}),
                });
            }
            Ok(json!({"results":results}))
        }
        BatchOperation::FolderCreate { name, parent_id } => {
            let parent = parent_id
                .as_ref()
                .map(|id| resolve_id(id, prior))
                .transpose()?;
            let folder = reading::create_folder_once(state, name, parent, Some(key)).await?;
            Ok(json!({"folder_id":folder.id,"name":folder.name,"parent_id":folder.parent_id}))
        }
        BatchOperation::ArticleImport {
            title,
            body,
            folder_id,
        } => {
            let folder = folder_id
                .as_ref()
                .map(|id| resolve_id(id, prior))
                .transpose()?;
            let id = reading::create_article_once(state, title, body, folder, Some(key)).await?;
            Ok(json!({"article_id":id,"title":title,"folder_id":folder}))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_dependencies_and_reference_ids() {
        let mut request: BatchRequest = serde_json::from_value(json!({"idempotency_key":"test", "operations":[
            {"id":"folder","operation":{"action":"folder_create","name":"German"}},
            {"id":"article","operation":{"action":"article_import","title":"Text","body":"Hallo", "folder_id":{"step":"folder","path":"/folder_id"}}}
        ]})).unwrap();
        validate(&request).unwrap();
        let prior = HashMap::from([(
            "folder".into(),
            BatchStepState {
                id: "folder".into(),
                status: "succeeded".into(),
                attempts: 1,
                result: Some(json!({"folder_id":12})),
                error: None,
            },
        )]);
        assert_eq!(
            resolve_id(
                &BatchId::Reference {
                    step: "folder".into(),
                    path: "/folder_id".into()
                },
                &prior
            )
            .unwrap(),
            12
        );
        assert!(resolve_id(
            &BatchId::Reference {
                step: "folder".into(),
                path: "/missing".into()
            },
            &prior
        )
        .is_err());
        request.operations.swap(0, 1);
        assert!(validate(&request).is_err());
    }

    #[tokio::test]
    #[ignore = "requires isolated DEAI_BATCH_TEST_DATABASE_URL (deai_test database)"]
    async fn durable_batches_and_generation_checkpoints() {
        use crate::{
            ai::AiClient,
            config::{AiModelConfig, Config},
            prompts::PromptConfig,
        };
        use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        };
        let url =
            std::env::var("DEAI_BATCH_TEST_DATABASE_URL").expect("isolated test URL required");
        let pool = crate::db::create_pool(&url).await.unwrap();
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            database, "deai_test",
            "refuse to mutate a non-test database"
        );
        crate::db::run_migrations(&pool).await.unwrap();
        sqlx::query("TRUNCATE agent_batches,agent_operation_receipts,analysis_checkpoints,reading_article_bookmarks,reading_articles,reading_folders,learning_progress,follow_ups,knowledge_entries,dictionary_lexemes,dictionary_raw_entries,dictionary_raw CASCADE").execute(&pool).await.unwrap();
        let counts = Arc::new(Mutex::new([0usize; 3]));
        let mode = Arc::new(AtomicUsize::new(0));
        let mock_counts = counts.clone();
        let mock_mode = mode.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let route = format!("http://{}", listener.local_addr().unwrap());
        let mock = Router::new().route("/chat/completions",post(move |Json(body): Json<Value>| {
            let counts = mock_counts.clone(); let mode = mock_mode.clone();
            async move {
                let prompt = body["messages"][0]["content"].as_str().unwrap();
                let stage = if prompt.contains("TEST_MODEL_A") {0} else if prompt.contains("TEST_STRUCTURE") {2} else {1};
                counts.lock().unwrap()[stage] += 1;
                if stage==2 || mode.load(Ordering::SeqCst)==3 { return StatusCode::UNAUTHORIZED.into_response(); }
                if stage==1 && mode.load(Ordering::SeqCst)==0 { return StatusCode::SERVICE_UNAVAILABLE.into_response(); }
                let content = if stage==0 {
                    json!({"word":"Wahl","entries":[{"pos":"noun","meanings":[{"zh":"选择","en":"choice"}]}]}).to_string()
                } else {
                    format!("#### 应用与例句 (Anwendung & Beispiele)\n{}\n#### 词汇网络 (Wortnetz)\n{}\n#### 深度解析与避坑 (Einblicke)\n{}。", "完整例句和解释。".repeat(35),"词汇关联。".repeat(35),"完整解释。".repeat(35))
                };
                Json(json!({"choices":[{"message":{"role":"assistant","content":content}}]})).into_response()
            }
        }));
        let server = tokio::spawn(async move {
            axum::serve(listener, mock).await.unwrap();
        });
        let config = Config {
            database_url: url,
            server_host: "127.0.0.1".into(),
            server_port: 0,
            agent_api_token: Some("test".into()),
            openai_api_key: Some("test".into()),
            openai_base_url: Some(route.clone()),
            prompt_config_path: String::new(),
            ai_models: AiModelConfig {
                default: "test".into(),
                analyze: "test".into(),
                analyze_pro: "test".into(),
                follow_up: "test".into(),
                follow_up_pro: "test".into(),
                intelligent_search: "test".into(),
                embedding: "test".into(),
                fallback_fast: String::new(),
                fallback_pro: String::new(),
            },
        };
        let state = AppState {
            pool: pool.clone(),
            config,
            prompts: PromptConfig {
                analysis_prompt: String::new(),
                model_a_prompt: "TEST_MODEL_A".into(),
                stage2_prompt: "TEST_MODEL_B".into(),
                structure_prompt: "TEST_STRUCTURE".into(),
                phrase_module_prompt: String::new(),
                follow_up_prompt: String::new(),
                intelligent_search_prompt: String::new(),
            },
            ai_client: AiClient::new("test".into(), route),
            recent_searches: Default::default(),
            learning_sessions: Default::default(),
        };
        let request: BatchRequest = serde_json::from_value(json!({"idempotency_key":"durable-test","operations":[
            {"id":"folder","operation":{"action":"folder_create","name":"Test folder"}},
            {"id":"bad","operation":{"action":"folder_create","name":"Invalid parent","parent_id":999999}},
            {"id":"article","operation":{"action":"article_import","title":"Test article","body":"Hallo Welt.","folder_id":{"step":"folder","path":"/folder_id"}}},
            {"id":"dependent","depends_on":["bad"],"operation":{"action":"article_import","title":"Skipped","body":"Hallo."}},
            {"id":"reference","operation":{"action":"article_import","title":"Skipped ref","body":"Hallo.","folder_id":{"step":"bad","path":"/folder_id"}}}
        ]})).unwrap();
        let job = submit(&state, request.clone()).await.unwrap();
        assert_eq!(job.status, "queued");
        assert_eq!(submit(&state, request.clone()).await.unwrap().id, job.id);
        let mut different = request.clone();
        different.on_error = FailurePolicy::Stop;
        assert!(submit(&state, different).await.is_err());
        execute_job(&state, &job.id).await.unwrap();
        let finished = repository::get(&pool, &job.id).await.unwrap();
        assert_eq!(finished.status, "completed_with_errors");
        assert_eq!(finished.steps[0]["status"], "succeeded");
        assert_eq!(finished.steps[1]["status"], "failed");
        assert_eq!(finished.steps[2]["status"], "succeeded");
        assert_eq!(finished.steps[3]["status"], "skipped");
        assert_eq!(finished.steps[4]["status"], "skipped");
        assert_eq!(
            finished.steps[2]["result"]["folder_id"],
            finished.steps[0]["result"]["folder_id"]
        );
        retry(&state, &job.id).await.unwrap();
        assert!(retry(&state, &job.id).await.is_err());
        execute_job(&state, &job.id).await.unwrap();
        let after_retry = repository::get(&pool, &job.id).await.unwrap();
        assert_eq!(after_retry.steps[0]["attempts"], 1);
        assert_eq!(after_retry.steps[1]["attempts"], 2);
        assert_eq!(after_retry.steps[2]["attempts"], 1);
        // Simulate death after a committed creation but before its step result was saved.
        let mut interrupted: Vec<BatchStepState> =
            serde_json::from_value(after_retry.steps).unwrap();
        interrupted[2].status = "running".into();
        interrupted[2].result = None;
        repository::update(&pool, &job.id, "running", &json!(interrupted), None)
            .await
            .unwrap();
        execute_job(&state, &job.id).await.unwrap();
        let articles: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM reading_articles WHERE title='Test article'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(articles, 1, "receipt prevents duplicate article creation");

        let stops: BatchRequest = serde_json::from_value(json!({"idempotency_key":"stop-test","on_error":"stop","operations":[
            {"id":"bad","operation":{"action":"folder_create","name":"Bad","parent_id":999999}},
            {"id":"independent","operation":{"action":"folder_create","name":"Should not exist"}}
        ]})).unwrap();
        let stopped = submit(&state, stops).await.unwrap();
        execute_job(&state, &stopped.id).await.unwrap();
        assert_eq!(
            repository::get(&pool, &stopped.id).await.unwrap().steps[1]["status"],
            "skipped"
        );

        // Prove A survives a B failure and A+B survive a C failure across fresh calls.
        use crate::models::{AnalysisDocument, QualityMode};
        use crate::services::analysis_grounded_runtime::{
            generate_grounded_analysis, stream_grounded_analysis, GroundedAnalysis,
        };
        assert!(generate_grounded_analysis(
            &state,
            "Wahl",
            None,
            QualityMode::Default,
            None,
            None,
            true
        )
        .await
        .is_err());
        assert_eq!(counts.lock().unwrap()[0], 1);
        let first: Value =
            sqlx::query_scalar("SELECT value FROM analysis_checkpoints WHERE stage='A'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(first["stage1"].is_object());
        mode.store(1, Ordering::SeqCst);
        assert!(generate_grounded_analysis(
            &state,
            "Wahl",
            None,
            QualityMode::Default,
            None,
            None,
            true
        )
        .await
        .is_err());
        assert_eq!(counts.lock().unwrap()[0], 1, "B retry must not repeat A");
        let before_c = *counts.lock().unwrap();
        assert!(generate_grounded_analysis(
            &state,
            "Wahl",
            None,
            QualityMode::Default,
            None,
            None,
            true
        )
        .await
        .is_err());
        let after_c = *counts.lock().unwrap();
        assert_eq!(
            before_c[0..2],
            after_c[0..2],
            "C retry must not repeat A or B"
        );
        assert_eq!(after_c[2], before_c[2] + 1);
        let (key, value): (String, Value) =
            sqlx::query_as("SELECT cache_key,value FROM analysis_checkpoints WHERE stage='B'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let complete = GroundedAnalysis {
            model: "test".into(),
            analysis: AnalysisDocument {
                markdown: value["ab"]["stage2_markdown"].as_str().unwrap().into(),
                structured: None,
                tags: vec![],
                aliases: vec![],
                prototype: None,
                attached_phrase_modules: vec![],
                dictionary_excerpt: None,
                model: Some("test".into()),
                quality_mode: Some(QualityMode::Default),
            },
        };
        let mut cached = value;
        cached["complete"] = json!(complete);
        crate::repositories::analysis_checkpoints::put(&pool, &key, "C", &cached)
            .await
            .unwrap();
        generate_grounded_analysis(&state, "Wahl", None, QualityMode::Default, None, None, true)
            .await
            .unwrap();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        stream_grounded_analysis(
            &state,
            &sender,
            "Wahl",
            None,
            QualityMode::Default,
            None,
            None,
            true,
        )
        .await
        .unwrap();
        assert!(receiver.try_recv().is_ok());
        assert_eq!(
            *counts.lock().unwrap(),
            after_c,
            "complete checkpoint avoids all model calls"
        );
        mode.store(3, Ordering::SeqCst);
        assert!(generate_grounded_analysis(
            &state,
            "Wahl",
            None,
            QualityMode::Default,
            None,
            None,
            false
        )
        .await
        .is_err());
        assert_eq!(
            counts.lock().unwrap()[0],
            2,
            "force refresh must bypass complete checkpoint"
        );

        sqlx::query("INSERT INTO dictionary_raw(headword,raw_data) VALUES('Wahl',$1)")
            .bind(json!({"pos":"noun","senses":[{"glosses":["choice"]}]}))
            .execute(&pool)
            .await
            .unwrap();
        let lexeme: i64 = sqlx::query_scalar("INSERT INTO dictionary_lexemes(bundle_key,surface,normalized_surface) VALUES('test-Wahl','Wahl','wahl') RETURNING id").fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO dictionary_raw_entries(source_key,headword,normalized_headword,lang_code,pos,raw_data) VALUES('test-Wahl','Wahl','wahl','de','noun',$1)").bind(json!({"word":"Wahl","pos":"noun","senses":[{"glosses":["choice"]}]})).execute(&pool).await.unwrap();
        let dictionary = crate::repositories::dictionary::find_by_headword(&pool, "Wahl")
            .await
            .unwrap()
            .unwrap();
        let key = crate::services::analysis_grounded_ab_cache::checkpoint_key(
            &state,
            "Wahl",
            Some(&dictionary),
            QualityMode::Default,
            None,
            None,
            true,
        )
        .await
        .unwrap();
        crate::repositories::analysis_checkpoints::put(&pool, &key, "C", &cached)
            .await
            .unwrap();
        let (first, second) = tokio::join!(
            agent_api::initialize_lexeme(&state, lexeme, None),
            agent_api::initialize_lexeme(&state, lexeme, None)
        );
        let entry = first.unwrap().entry_id;
        assert_eq!(second.unwrap().entry_id, entry);
        let entry_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_entries WHERE query_text='Wahl'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            entry_count, 1,
            "concurrent identical words persist only one entry"
        );
        let mixed: BatchRequest = serde_json::from_value(json!({"idempotency_key":"mixed-words","operations":[
            {"id":"words","operation":{"action":"lexeme_initialize","lexeme_ids":[lexeme,999999]}},
            {"id":"learn","operation":{"action":"learning_membership","mutation":"add","ids":[{"step":"words","path":"/results/0/value/entry_id"}]}},
            {"id":"all","depends_on":["words"],"operation":{"action":"folder_create","name":"All words succeeded"}},
            {"id":"failed-ref","operation":{"action":"learning_membership","mutation":"add","ids":[{"step":"words","path":"/results/1/value/entry_id"}]}}
        ]})).unwrap();
        let mixed_job = submit(&state, mixed).await.unwrap();
        execute_job(&state, &mixed_job.id).await.unwrap();
        let mixed_job = repository::get(&pool, &mixed_job.id).await.unwrap();
        assert_eq!(mixed_job.steps[0]["status"], "failed");
        assert_eq!(mixed_job.steps[0]["result"]["results"][0]["ok"], true);
        assert_eq!(mixed_job.steps[0]["result"]["results"][1]["ok"], false);
        assert_eq!(
            mixed_job.steps[1]["status"], "succeeded",
            "successful item of a partial step remains usable"
        );
        assert_eq!(mixed_job.steps[2]["status"], "skipped");
        assert_eq!(mixed_job.steps[3]["status"], "skipped");
        let reset: BatchRequest = serde_json::from_value(json!({"idempotency_key":"reset-once","operations":[
            {"id":"reset","operation":{"action":"learning_membership","mutation":"reset","ids":[entry]}}
        ]})).unwrap();
        let reset = submit(&state, reset).await.unwrap();
        execute_job(&state, &reset.id).await.unwrap();
        sqlx::query("UPDATE learning_progress SET review_count=2,state=2 WHERE entry_id=$1")
            .bind(entry)
            .execute(&pool)
            .await
            .unwrap();
        let mut steps: Vec<BatchStepState> =
            serde_json::from_value(repository::get(&pool, &reset.id).await.unwrap().steps).unwrap();
        steps[0].status = "running".into();
        steps[0].result = None;
        repository::update(&pool, &reset.id, "running", &json!(steps), None)
            .await
            .unwrap();
        execute_job(&state, &reset.id).await.unwrap();
        let reviews: i32 =
            sqlx::query_scalar("SELECT review_count FROM learning_progress WHERE entry_id=$1")
                .bind(entry)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            reviews, 2,
            "recovery must not reset a card again after new reviews"
        );

        let backup: crate::models::DatabaseBackupPayload = serde_json::from_str(
            &crate::services::management::export_database(&state)
                .await
                .unwrap(),
        )
        .unwrap();
        let queued: BatchRequest = serde_json::from_value(json!({"idempotency_key":"restore-guard","operations":[{"id":"folder","operation":{"action":"folder_create","name":"Pending folder"}}]})).unwrap();
        let queued = submit(&state, queued).await.unwrap();
        assert!(
            crate::repositories::management::replace_database_snapshot(&pool, &backup)
                .await
                .is_err(),
            "restore cannot race with pending work"
        );
        execute_job(&state, &queued.id).await.unwrap();
        crate::repositories::management::replace_database_snapshot(&pool, &backup)
            .await
            .unwrap();
        let tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_batches")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            tasks, 0,
            "restore clears receipts/tasks referring to old IDs"
        );
        server.abort();
    }
}
