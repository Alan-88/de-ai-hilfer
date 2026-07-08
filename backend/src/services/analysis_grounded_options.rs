use crate::ai::AiChatOptions;
use crate::models::QualityMode;
use std::time::Duration;

pub(crate) fn model_a_chat_options() -> AiChatOptions {
    AiChatOptions {
        temperature: 0.1,
        max_tokens: Some(2200),
        timeout: Duration::from_secs(90),
    }
}

pub(crate) fn stage2_chat_options(_quality_mode: QualityMode) -> AiChatOptions {
    AiChatOptions {
        temperature: 0.2,
        max_tokens: Some(1800),
        timeout: Duration::from_secs(90),
    }
}

pub(crate) fn structure_chat_options() -> AiChatOptions {
    AiChatOptions {
        temperature: 0.0,
        max_tokens: None,
        timeout: Duration::from_secs(90),
    }
}
