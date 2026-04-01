use crate::config::AiModelConfig;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct AiChatOptions {
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout: Duration,
}

impl Default for AiChatOptions {
    fn default() -> Self {
        Self {
            temperature: 0.2,
            max_tokens: 1200,
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

#[derive(Debug, Clone, Copy)]
pub enum AiScene {
    Analyze,
    FollowUp,
    IntelligentSearch,
    SpellCheck,
    Prototype,
    Embedding,
}

#[derive(Clone)]
pub struct AiClient {
    client: Client,
    api_key: String,
    base_url: String,
    models: AiModelConfig,
}

impl AiClient {
    pub fn new(api_key: String, base_url: String, models: AiModelConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(AiChatOptions::default().timeout)
                .build()
                .expect("failed to build reqwest client"),
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            models,
        }
    }

    pub async fn chat_with_options(
        &self,
        scene: AiScene,
        system_prompt: &str,
        user_message: &str,
        options: AiChatOptions,
    ) -> Result<String> {
        self.chat_model_with_options(self.model_for(scene), system_prompt, user_message, options)
            .await
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
                    ChatMessage {
                        role: "system".to_string(),
                        content: system_prompt.to_string(),
                    },
                    ChatMessage {
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
            .context("AI endpoint returned error status")?
            .json::<ChatCompletionResponse>()
            .await
            .context("failed to decode AI response")?;

        response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| anyhow!("AI response did not contain message content"))
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
                    ChatMessage {
                        role: "system".to_string(),
                        content: system_prompt.to_string(),
                    },
                    ChatMessage {
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

    pub async fn embed_with_options(
        &self,
        scene: AiScene,
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
                model: self.model_for(scene).to_string(),
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

    pub fn model_for(&self, scene: AiScene) -> &str {
        match scene {
            AiScene::Analyze => &self.models.analyze,
            AiScene::FollowUp => &self.models.follow_up,
            AiScene::IntelligentSearch => &self.models.intelligent_search,
            AiScene::SpellCheck => &self.models.spell_check,
            AiScene::Prototype => &self.models.prototype,
            AiScene::Embedding => &self.models.embedding,
        }
    }

    fn chat_max_tokens(&self, model: &str, options: AiChatOptions) -> Option<u32> {
        if should_omit_max_tokens(&self.base_url, model) {
            None
        } else {
            Some(options.max_tokens)
        }
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
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
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
