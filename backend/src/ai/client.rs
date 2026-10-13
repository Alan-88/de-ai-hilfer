use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct AiChatOptions {
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub timeout: Duration,
}

impl Default for AiChatOptions {
    fn default() -> Self {
        Self {
            temperature: 0.2,
            max_tokens: Some(1200),
            timeout: Duration::from_secs(45),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AiEmbeddingOptions {
    pub timeout: Duration,
    pub dimensions: Option<u32>,
}

impl Default for AiEmbeddingOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(90),
            dimensions: None,
        }
    }
}

#[derive(Clone)]
pub struct AiClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AiClient {
    pub async fn chat_with_tools(
        &self,
        model: &str,
        messages: &[serde_json::Value],
        tools: &serde_json::Value,
    ) -> Result<ToolTurn> {
        let mut body = serde_json::json!({"model":model,"messages":messages,"tools":tools,"tool_choice":"auto","temperature":0.1});
        if let Some(tokens) = self.chat_max_tokens(
            model,
            AiChatOptions {
                max_tokens: Some(2048),
                ..Default::default()
            },
        ) {
            body["max_tokens"] = tokens.into();
        }
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .timeout(Duration::from_secs(90))
            .json(&body)
            .send()
            .await
            .context("failed to call follow-up AI endpoint")?
            .error_for_status()
            .context("follow-up AI endpoint returned error")?;
        parse_tool_turn(
            &response
                .text()
                .await
                .context("failed to read follow-up AI response")?,
        )
    }
    pub(crate) fn routing_identity(&self) -> &str {
        &self.base_url
    }

    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(AiChatOptions::default().timeout)
                .build()
                .expect("failed to build reqwest client"),
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn chat_model_with_options(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        options: AiChatOptions,
    ) -> Result<String> {
        let max_tokens = self.chat_max_tokens(model, options);
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .timeout(options.timeout)
            .json(&ChatCompletionRequest {
                model: model.to_string(),
                messages: vec![
                    ChatRequestMessage {
                        role: "system".to_string(),
                        content: system_prompt.to_string(),
                    },
                    ChatRequestMessage {
                        role: "user".to_string(),
                        content: user_message.to_string(),
                    },
                ],
                temperature: options.temperature,
                max_tokens,
                stream: None,
            })
            .send()
            .await
            .context("failed to call AI endpoint")?
            .error_for_status()
            .context("AI endpoint returned error status")?;

        let body = response
            .text()
            .await
            .context("failed to read AI response body")?;
        let response = parse_chat_completion_response(&body)?;

        response
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.extract_text())
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| anyhow!("AI response did not contain message content"))
    }

    pub async fn chat_model_connectivity_check(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        options: AiChatOptions,
    ) -> Result<()> {
        let max_tokens = self.chat_max_tokens(model, options);
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .timeout(options.timeout)
            .json(&ChatCompletionRequest {
                model: model.to_string(),
                messages: vec![
                    ChatRequestMessage {
                        role: "system".to_string(),
                        content: system_prompt.to_string(),
                    },
                    ChatRequestMessage {
                        role: "user".to_string(),
                        content: user_message.to_string(),
                    },
                ],
                temperature: options.temperature,
                max_tokens,
                stream: None,
            })
            .send()
            .await
            .context("failed to call AI endpoint")?
            .error_for_status()
            .context("AI endpoint returned error status")?;

        let body = response
            .text()
            .await
            .context("failed to read AI response body")?;
        let response = parse_chat_completion_response(&body)?;
        if response.choices.is_empty() {
            return Err(anyhow!("AI response did not contain choices"));
        }

        Ok(())
    }

    pub async fn chat_model_stream_with_options(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        options: AiChatOptions,
    ) -> Result<reqwest::Response> {
        let max_tokens = self.chat_max_tokens(model, options);
        self.client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .timeout(options.timeout)
            .json(&ChatCompletionRequest {
                model: model.to_string(),
                messages: vec![
                    ChatRequestMessage {
                        role: "system".to_string(),
                        content: system_prompt.to_string(),
                    },
                    ChatRequestMessage {
                        role: "user".to_string(),
                        content: user_message.to_string(),
                    },
                ],
                temperature: options.temperature,
                max_tokens,
                stream: Some(true),
            })
            .send()
            .await
            .context("failed to call AI streaming endpoint")?
            .error_for_status()
            .context("AI streaming endpoint returned error status")
    }

    pub async fn embed_model_with_options(
        &self,
        model: &str,
        inputs: &[String],
        options: AiEmbeddingOptions,
    ) -> Result<Vec<Vec<f32>>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .bearer_auth(&self.api_key)
            .timeout(options.timeout)
            .json(&EmbeddingRequest {
                model: model.to_string(),
                input: inputs.to_vec(),
                dimensions: options.dimensions,
                encoding_format: Some("float"),
            })
            .send()
            .await
            .context("failed to call embedding endpoint")?
            .error_for_status()
            .context("embedding endpoint returned error status")?
            .json::<EmbeddingResponse>()
            .await
            .context("failed to decode embedding response")?;

        let mut data = response.data;
        data.sort_by_key(|item| item.index);

        if data.len() != inputs.len() {
            return Err(anyhow!(
                "embedding response length mismatch: expected {}, got {}",
                inputs.len(),
                data.len()
            ));
        }

        Ok(data.into_iter().map(|item| item.embedding).collect())
    }

    fn chat_max_tokens(&self, model: &str, options: AiChatOptions) -> Option<u32> {
        if options.max_tokens.is_none() || should_omit_max_tokens(&self.base_url, model) {
            None
        } else {
            options.max_tokens
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolTurn {
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: ToolFunction,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

fn parse_tool_turn(body: &str) -> Result<ToolTurn> {
    let response: serde_json::Value =
        serde_json::from_str(body).context("invalid follow-up AI response")?;
    let choice = response["choices"]
        .as_array()
        .and_then(|items| items.first())
        .ok_or_else(|| anyhow!("follow-up AI returned no choice"))?;
    match choice["finish_reason"].as_str() {
        Some("stop" | "tool_calls") => {}
        Some("length") => return Err(anyhow!("追问输出达到长度上限，未保存不完整回答，请重试")),
        _ => return Err(anyhow!("追问未正常完成，未保存回答，请重试")),
    }
    let turn: ToolTurn = serde_json::from_value(choice["message"].clone())
        .context("invalid follow-up assistant message")?;
    anyhow::ensure!(
        turn.content
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
            || !turn.tool_calls.is_empty(),
        "追问返回空回答，请重试"
    );
    Ok(turn)
}

#[cfg(test)]
mod tool_tests {
    use super::*;
    #[test]
    fn rejects_incomplete_answers_and_preserves_markdown() {
        assert!(parse_tool_turn(
            r#"{"choices":[{"finish_reason":"length","message":{"content":"半句"}}]}"#
        )
        .is_err());
        assert!(parse_tool_turn(r#"{"choices":[{"message":{"content":"半句"}}]}"#).is_err());
        assert!(parse_tool_turn(
            r#"{"choices":[{"finish_reason":"stop","message":{"content":" "}}]}"#
        )
        .is_err());
        let answer = parse_tool_turn(
            r#"{"choices":[{"finish_reason":"stop","message":{"content":"**结论**\n\n- 例句"}}]}"#,
        )
        .unwrap();
        assert_eq!(answer.content.unwrap(), "**结论**\n\n- 例句");
    }

    #[test]
    fn preserves_provider_metadata_for_tool_continuation() {
        let body = serde_json::json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","content":null,"reasoning_content":"provider continuation","tool_calls":[{"id":"call-1","type":"function","function":{"name":"lexeme_query","arguments":"{}"},"extra_content":{"google":{"thought_signature":"opaque-signature"}}}]}}]});
        let turn = parse_tool_turn(&body.to_string()).unwrap();
        assert_eq!(
            serde_json::to_value(turn).unwrap(),
            body["choices"][0]["message"]
        );
    }
}

fn should_omit_max_tokens(base_url: &str, model: &str) -> bool {
    let model = model.to_ascii_lowercase();
    let base_url = base_url.to_ascii_lowercase();

    model.starts_with("gemini-") && !base_url.contains("api.openai.com")
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatRequestMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct ChatRequestMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    #[serde(default)]
    content: Option<ChatResponseContent>,
}

impl ChatResponseMessage {
    fn extract_text(self) -> Option<String> {
        self.content.and_then(ChatResponseContent::into_text)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ChatResponseContent {
    Text(String),
    Parts(Vec<ChatResponseContentPart>),
}

impl ChatResponseContent {
    fn into_text(self) -> Option<String> {
        match self {
            Self::Text(text) => Some(text),
            Self::Parts(parts) => {
                let combined = parts
                    .into_iter()
                    .filter_map(ChatResponseContentPart::into_text)
                    .collect::<String>();

                if combined.trim().is_empty() {
                    None
                } else {
                    Some(combined)
                }
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ChatResponseContentPart {
    Text(String),
    Object { text: Option<String> },
}

impl ChatResponseContentPart {
    fn into_text(self) -> Option<String> {
        match self {
            Self::Text(text) => Some(text),
            Self::Object { text } => text,
        }
    }
}

fn parse_chat_completion_response(body: &str) -> Result<ChatCompletionResponse> {
    serde_json::from_str(body).with_context(|| {
        let preview: String = body.chars().take(600).collect();
        format!("failed to decode AI response body: preview={preview}")
    })
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<&'static str>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Deserialize)]
struct EmbeddingItem {
    index: usize,
    embedding: Vec<f32>,
}
