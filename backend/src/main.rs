mod ai;
mod config;
mod db;
mod dictionary_lexeme_models;
mod handlers;
mod models;
mod prompts;
mod repositories;
mod services;
mod state;

use axum::{
    routing::{get, post},
    Router,
};
use state::AppState;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "de_ai_hilfer=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    let config = config::Config::from_env()?;
    tracing::info!("Configuration loaded");
    tracing::info!("Database URL: {}", config.redacted_database_url());

    // 创建数据库连接池
    let pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Database pool created");

    // 运行迁移
    db::run_migrations(&pool).await?;
    tracing::info!("Database migrations completed");

    // 配置 CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let prompts = prompts::PromptConfig::load(&config.prompt_config_path)?;
    let ai_client = ai::AiClient::new(
        config.openai_api_key.clone().unwrap_or_default(),
        config
            .openai_base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        config.ai_models.clone(),
    );
    tracing::info!(
        "AI routing loaded: default={}, analyze={}, analyze_pro={}, follow_up={}, follow_up_pro={}, intelligent_search={}, spell_check={}, prototype={}, fallback_fast={}, fallback_pro={}",
        config.ai_models.default,
        config.ai_models.analyze,
        config.ai_models.analyze_pro,
        config.ai_models.follow_up,
        config.ai_models.follow_up_pro,
        config.ai_models.intelligent_search,
        config.ai_models.spell_check,
        config.ai_models.prototype,
        config.ai_models.fallback_fast,
        config.ai_models.fallback_pro
    );
    let app_state = AppState {
        pool,
        config,
        prompts,
        ai_client,
        recent_searches: Arc::new(Mutex::new(VecDeque::with_capacity(20))),
    };

    // 构建路由
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/analyze", post(handlers::analyze::analyze_word))
        .route(
            "/api/v1/phrases/attach",
            post(handlers::analyze::attach_phrase_to_host),
        )
        .route(
            "/api/v1/phrases/detach",
            post(handlers::analyze::detach_phrase_from_host),
        )
        .route(
            "/api/v1/analyze/stream",
            post(handlers::analyze::stream_analyze_word),
        )
        .route(
            "/api/v1/follow-ups",
            post(handlers::follow_up::create_follow_up),
        )
        .route(
            "/api/v1/follow-ups/stream",
            post(handlers::follow_up::stream_follow_up),
        )
        .route(
            "/api/v1/entries/recent",
            get(handlers::query::get_recent_entries),
        )
        .route("/api/v1/entries", get(handlers::query::get_library_entries_page))
        .route("/api/v1/entries/all", get(handlers::query::get_all_entries))
        .route(
            "/api/v1/entries/:entry_id",
            get(handlers::query::get_entry_detail).delete(handlers::query::delete_entry),
        )
        .route("/api/v1/suggestions", get(handlers::query::get_suggestions))
        .route(
            "/api/v1/intelligent_search",
            post(handlers::query::intelligent_search),
        )
        .route("/api/v1/status", get(handlers::query::get_status))
        .route(
            "/api/v1/database/export",
            get(handlers::management::export_database),
        )
        .route(
            "/api/v1/database/import",
            post(handlers::management::import_database),
        )
        .route(
            "/api/v1/learning/session/v2",
            get(handlers::learning::get_session),
        )
        .route(
            "/api/v1/learning/add/:entry_id",
            post(handlers::learning::add_word),
        )
        .route(
            "/api/v1/learning/review/v2/:entry_id",
            post(handlers::learning::review_word),
        )
        .route(
            "/api/v1/learning/progress",
            get(handlers::learning::get_progress),
        )
        .route("/api/v1/learning/stats", get(handlers::learning::get_stats))
        .layer(cors)
        .with_state(app_state.clone());

    // 启动服务器
    let addr = app_state.config.server_address();
    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
