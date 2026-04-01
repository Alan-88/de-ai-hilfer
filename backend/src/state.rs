use crate::ai::AiClient;
use crate::config::Config;
use crate::db::DbPool;
use crate::prompts::PromptConfig;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
    pub prompts: PromptConfig,
    pub ai_client: AiClient,
    pub recent_searches: Arc<Mutex<VecDeque<String>>>,
}
