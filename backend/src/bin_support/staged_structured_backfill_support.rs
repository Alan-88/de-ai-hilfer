use anyhow::{Context, Result};
use de_ai_hilfer::services::analysis_grounded_backfill::GroundedAbCache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const PIPELINE_VERSION: &str = "abc-v1-strict-primary";

#[derive(Debug, Clone, Copy)]
pub enum Command {
    PrepareAb,
    RunC,
    ApplyReady,
}

impl Command {
    pub fn from_env() -> Self {
        match std::env::var("STAGED_BACKFILL_COMMAND")
            .unwrap_or_else(|_| "prepare_ab".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "run_c" | "c" => Self::RunC,
            "apply_ready" | "apply" => Self::ApplyReady,
            _ => Self::PrepareAb,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PrepareAb => "prepare_ab",
            Self::RunC => "run_c",
            Self::ApplyReady => "apply_ready",
        }
    }
}

#[derive(Clone)]
pub struct Options {
    pub command: Command,
    pub queue_dir: PathBuf,
    pub limit: i64,
    pub words: Vec<String>,
    pub request_spacing_secs: u64,
    pub prepare_concurrency: usize,
    pub c_concurrency: usize,
    pub c_retry_limit: usize,
    pub c_retry_initial_delay_secs: u64,
    pub c_retry_max_delay_secs: u64,
    pub deferred_cooldown_secs: u64,
    pub structure_model: Option<String>,
    pub ignore_db_existing: bool,
}

impl Options {
    pub fn from_env() -> Self {
        Self {
            command: Command::from_env(),
            queue_dir: std::env::var("STAGED_BACKFILL_QUEUE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("tmp/staged_structured_backfill")),
            limit: env_i64("STAGED_BACKFILL_LIMIT", 1).max(1),
            words: env_words("STAGED_BACKFILL_WORDS"),
            request_spacing_secs: env_u64("STAGED_BACKFILL_REQUEST_SPACING_SECS", 120),
            prepare_concurrency: env_usize("STAGED_BACKFILL_PREPARE_CONCURRENCY", 1).max(1),
            c_concurrency: env_usize("STAGED_BACKFILL_C_CONCURRENCY", 1).max(1),
            c_retry_limit: env_usize("STAGED_BACKFILL_C_RETRIES", 3),
            c_retry_initial_delay_secs: env_u64("STAGED_BACKFILL_C_RETRY_INITIAL_SECS", 180),
            c_retry_max_delay_secs: env_u64("STAGED_BACKFILL_C_RETRY_MAX_SECS", 900),
            deferred_cooldown_secs: env_u64("STAGED_BACKFILL_DEFERRED_COOLDOWN_SECS", 1800),
            structure_model: env_optional_string("STAGED_BACKFILL_STRUCTURE_MODEL"),
            ignore_db_existing: env_bool("STAGED_BACKFILL_IGNORE_DB_EXISTING", false),
        }
    }
}

pub struct QueuePaths {
    pub ab_path: PathBuf,
    pub ready_path: PathBuf,
    pub applied_path: PathBuf,
    pub ab_failed_path: PathBuf,
    pub ab_deferred_path: PathBuf,
    pub c_failed_path: PathBuf,
    pub c_deferred_path: PathBuf,
}

impl QueuePaths {
    pub fn new(root: &Path, word: &str) -> Self {
        let slug = slugify(word);
        Self {
            ab_path: root.join("ab").join(format!("{slug}.json")),
            ready_path: root.join("ready").join(format!("{slug}.json")),
            applied_path: root.join("applied").join(format!("{slug}.json")),
            ab_failed_path: root.join("failed").join(format!("{slug}.prepare_ab.json")),
            ab_deferred_path: root
                .join("deferred")
                .join(format!("{slug}.prepare_ab.json")),
            c_failed_path: root.join("failed").join(format!("{slug}.run_c.json")),
            c_deferred_path: root.join("deferred").join(format!("{slug}.run_c.json")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AbArtifact {
    pub pipeline_version: String,
    pub query_text: String,
    pub model_a_prompt_hash: String,
    pub stage2_prompt_hash: String,
    pub cache: GroundedAbCache,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadyArtifact {
    pub pipeline_version: String,
    pub query_text: String,
    pub stage2_model: String,
    pub analysis: Value,
}

#[derive(Debug, Serialize)]
pub struct StageReport {
    pub success_count: usize,
    pub failed_count: usize,
    pub skipped_count: usize,
    pub cases: Vec<StageCase>,
}

impl StageReport {
    pub fn from_cases(cases: Vec<StageCase>) -> Self {
        Self {
            success_count: cases.iter().filter(|case| case.status == "success").count(),
            failed_count: cases.iter().filter(|case| case.status == "failed").count(),
            skipped_count: cases.iter().filter(|case| case.status == "skipped").count(),
            cases,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StageCase {
    pub query_text: String,
    pub status: String,
    pub reason: Option<String>,
    pub error_kind: Option<String>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "is_zero")]
    pub transient_retries: usize,
}

impl StageCase {
    pub fn success(word: &str) -> Self {
        Self::success_with_retries(word, 0)
    }

    pub fn success_with_retries(word: &str, transient_retries: usize) -> Self {
        Self {
            query_text: word.to_string(),
            status: "success".to_string(),
            reason: None,
            error_kind: None,
            error: None,
            transient_retries,
        }
    }

    pub fn skipped(word: &str, reason: &str) -> Self {
        Self {
            query_text: word.to_string(),
            status: "skipped".to_string(),
            reason: Some(reason.to_string()),
            error_kind: None,
            error: None,
            transient_retries: 0,
        }
    }

    pub fn failed(word: &str, kind: &str, error: &anyhow::Error) -> Self {
        Self::failed_message_with_retries(word, kind, &format!("{error:#}"), 0)
    }

    pub fn failed_message(word: &str, kind: &str, message: &str) -> Self {
        Self::failed_message_with_retries(word, kind, message, 0)
    }

    pub fn failed_with_retries(
        word: &str,
        kind: &str,
        error: &anyhow::Error,
        transient_retries: usize,
    ) -> Self {
        Self::failed_message_with_retries(word, kind, &format!("{error:#}"), transient_retries)
    }

    pub fn failed_message_with_retries(
        word: &str,
        kind: &str,
        message: &str,
        transient_retries: usize,
    ) -> Self {
        Self {
            query_text: word.to_string(),
            status: "failed".to_string(),
            reason: None,
            error_kind: Some(kind.to_string()),
            error: Some(message.to_string()),
            transient_retries,
        }
    }

    pub fn deferred_with_retries(
        word: &str,
        kind: &str,
        error: &anyhow::Error,
        transient_retries: usize,
    ) -> Self {
        Self {
            query_text: word.to_string(),
            status: "deferred".to_string(),
            reason: Some("transient_upstream_failure".to_string()),
            error_kind: Some(kind.to_string()),
            error: Some(format!("{error:#}")),
            transient_retries,
        }
    }
}

pub async fn ensure_dirs(root: &Path) -> Result<()> {
    for name in ["ab", "ready", "applied", "failed", "deferred", "reports"] {
        tokio::fs::create_dir_all(root.join(name)).await?;
    }
    Ok(())
}

pub async fn list_artifacts(root: &Path, subdir: &str, limit: i64) -> Result<Vec<PathBuf>> {
    let mut entries = tokio::fs::read_dir(root.join(subdir)).await?;
    let mut paths = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    paths.sort();
    paths.truncate(limit as usize);
    Ok(paths)
}

pub async fn write_report(root: &Path, command: &Command, report: &StageReport) -> Result<PathBuf> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let path = root
        .join("reports")
        .join(format!("{}_{}.json", command.as_str(), timestamp));
    write_json(&path, report).await?;
    Ok(path)
}

pub async fn write_failure(
    path: &Path,
    word: &str,
    stage: &str,
    error: &anyhow::Error,
) -> Result<()> {
    write_failure_with_retries(path, word, stage, error, 0).await
}

pub async fn write_failure_with_retries(
    path: &Path,
    word: &str,
    stage: &str,
    error: &anyhow::Error,
    transient_retries: usize,
) -> Result<()> {
    let payload = serde_json::json!({
        "query_text": word,
        "stage": stage,
        "error_kind": classify_error(error),
        "error": format!("{error:#}"),
        "transient_retries": transient_retries,
    });
    write_json(path, &payload).await
}

pub async fn write_deferred_with_retries(
    path: &Path,
    word: &str,
    stage: &str,
    error: &anyhow::Error,
    transient_retries: usize,
) -> Result<()> {
    let payload = serde_json::json!({
        "query_text": word,
        "stage": stage,
        "error_kind": classify_error(error),
        "error": format!("{error:#}"),
        "transient_retries": transient_retries,
        "deferred_at": chrono::Utc::now().to_rfc3339(),
    });
    write_json(path, &payload).await
}

pub async fn write_json<T: Serialize>(path: &Path, payload: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, serde_json::to_vec_pretty(payload)?)
        .await
        .with_context(|| format!("failed to write {}", path.display()))
}

pub async fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = tokio::fs::read(path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn prompt_hash(prompt: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in prompt.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub fn classify_error(err: &anyhow::Error) -> String {
    let message = format!("{err:#}").to_lowercase();
    if message.contains("429")
        || message.contains("too many requests")
        || message.contains("rate limit")
        || message.contains("rate_limit")
    {
        "rate_limited".to_string()
    } else if message.contains("timed out") || message.contains("timeout") {
        "timeout".to_string()
    } else if message.contains("did not contain message content") {
        "empty_response".to_string()
    } else if message.contains("502")
        || message.contains("503")
        || message.contains("504")
        || message.contains("temporarily unavailable")
        || message.contains("server overloaded")
    {
        "upstream_unavailable".to_string()
    } else if message.contains("quality gate rejected") {
        "quality_gate".to_string()
    } else if message.contains("json") {
        "json_parse".to_string()
    } else {
        "other".to_string()
    }
}

pub fn is_retryable_run_c_error(kind: &str) -> bool {
    matches!(
        kind,
        "rate_limited" | "timeout" | "empty_response" | "upstream_unavailable" | "json_parse"
    )
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

fn slugify(word: &str) -> String {
    let mut slug = String::with_capacity(word.len() * 2 + 2);
    slug.push('q');
    slug.push('_');
    for byte in word.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(&mut slug, "{byte:02x}");
    }
    slug
}

fn env_i64(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
        .unwrap_or(default)
}

fn env_optional_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_words(name: &str) -> Vec<String> {
    std::env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(ToString::to_string)
        .collect()
}
