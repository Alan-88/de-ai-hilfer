use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub openai_api_key: Option<String>,
    pub openai_base_url: Option<String>,
    pub ai_models: AiModelConfig,
    pub prompt_config_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiModelConfig {
    pub default: String,
    pub analyze: String,
    pub analyze_pro: String,
    pub follow_up: String,
    pub follow_up_pro: String,
    pub intelligent_search: String,
    pub spell_check: String,
    pub prototype: String,
    pub embedding: String,
    pub fallback_fast: String,
    pub fallback_pro: String,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenvy::dotenv().ok();

        let ai_models = ai_models_from_env();

        Ok(Config {
            database_url: database_url_from_env(),
            server_host: expanded_env_var("SERVER_HOST").unwrap_or_else(|| "127.0.0.1".to_string()),
            server_port: expanded_env_var("SERVER_PORT")
                .unwrap_or_else(|| "8000".to_string())
                .parse()
                .unwrap_or(8000),
            openai_api_key: expanded_env_var("OPENAI_API_KEY"),
            openai_base_url: expanded_env_var("OPENAI_BASE_URL"),
            ai_models,
            prompt_config_path: expanded_env_var("PROMPT_CONFIG_PATH")
                .unwrap_or_else(|| "./config/prompts/default.yaml".to_string()),
        })
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }

    pub fn redacted_database_url(&self) -> String {
        redact_database_url(&self.database_url)
    }
}

fn ai_models_from_env() -> AiModelConfig {
    let default = expanded_env_var("AI_MODEL_DEFAULT")
        .or_else(|| expanded_env_var("OPENAI_MODEL"))
        .unwrap_or_else(|| "gemini-3-flash-preview".to_string());

    AiModelConfig {
        analyze: expanded_env_var("AI_MODEL_ANALYZE").unwrap_or_else(|| default.clone()),
        analyze_pro: expanded_env_var("AI_MODEL_ANALYZE_PRO")
            .or_else(|| expanded_env_var("AI_MODEL_PRO"))
            .unwrap_or_else(|| "gemini-3.1-pro-preview".to_string()),
        follow_up: expanded_env_var("AI_MODEL_FOLLOW_UP").unwrap_or_else(|| default.clone()),
        follow_up_pro: expanded_env_var("AI_MODEL_FOLLOW_UP_PRO")
            .or_else(|| expanded_env_var("AI_MODEL_PRO"))
            .or_else(|| expanded_env_var("AI_MODEL_ANALYZE_PRO"))
            .unwrap_or_else(|| "gemini-3.1-pro-preview".to_string()),
        intelligent_search: expanded_env_var("AI_MODEL_INTELLIGENT_SEARCH")
            .unwrap_or_else(|| default.clone()),
        spell_check: expanded_env_var("AI_MODEL_SPELL_CHECK").unwrap_or_else(|| default.clone()),
        prototype: expanded_env_var("AI_MODEL_PROTOTYPE").unwrap_or_else(|| default.clone()),
        embedding: expanded_env_var("AI_MODEL_EMBEDDING")
            .or_else(|| expanded_env_var("OPENAI_EMBEDDING_MODEL"))
            .unwrap_or_else(|| "gemini-embedding-2-preview".to_string()),
        fallback_fast: expanded_env_var("AI_MODEL_FALLBACK_FAST")
            .or_else(|| expanded_env_var("AI_MODEL_FALLBACK"))
            .unwrap_or_else(|| "glm-4.7-flash".to_string()),
        fallback_pro: expanded_env_var("AI_MODEL_FALLBACK_PRO")
            .or_else(|| expanded_env_var("AI_MODEL_FALLBACK"))
            .unwrap_or_else(|| "glm-5".to_string()),
        default,
    }
}

fn database_url_from_env() -> String {
    if let Some(database_url) = expanded_env_var("DATABASE_URL") {
        return database_url;
    }

    let host = expanded_env_var("DB_HOST").unwrap_or_else(|| "localhost".to_string());
    let port = expanded_env_var("DB_PORT").unwrap_or_else(|| "5432".to_string());
    let name = expanded_env_var("DB_NAME").unwrap_or_else(|| "de_ai_hilfer".to_string());
    let user = expanded_env_var("DB_USER").unwrap_or_else(|| "server".to_string());
    let password = expanded_env_var("DB_PASSWORD").unwrap_or_default();

    if password.is_empty() {
        format!(
            "postgres://{}@{}:{}/{}",
            encode_url_component(&user),
            host,
            port,
            name
        )
    } else {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            encode_url_component(&user),
            encode_url_component(&password),
            host,
            port,
            name
        )
    }
}

fn expanded_env_var(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| expand_env_placeholders(&value, 4))
        .filter(|value| !value.trim().is_empty())
}

fn expand_env_placeholders(value: &str, depth: usize) -> String {
    if depth == 0 {
        return value.to_string();
    }

    let chars = value.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut expanded = String::with_capacity(value.len());

    while index < chars.len() {
        if chars[index] != '$' {
            expanded.push(chars[index]);
            index += 1;
            continue;
        }

        if index + 1 < chars.len() && chars[index + 1] == '{' {
            if let Some(end) = chars[index + 2..].iter().position(|ch| *ch == '}') {
                let name = chars[index + 2..index + 2 + end].iter().collect::<String>();
                if let Ok(replacement) = env::var(&name) {
                    expanded.push_str(&expand_env_placeholders(&replacement, depth - 1));
                } else {
                    expanded.push_str(&format!("${{{name}}}"));
                }
                index += end + 3;
                continue;
            }
        }

        let mut end = index + 1;
        while end < chars.len() && matches!(chars[end], 'A'..='Z' | 'a'..='z' | '0'..='9' | '_') {
            end += 1;
        }

        if end == index + 1 {
            expanded.push('$');
            index += 1;
            continue;
        }

        let name = chars[index + 1..end].iter().collect::<String>();
        if let Ok(replacement) = env::var(&name) {
            expanded.push_str(&expand_env_placeholders(&replacement, depth - 1));
        } else {
            expanded.push('$');
            expanded.push_str(&name);
        }
        index = end;
    }

    expanded
}

fn encode_url_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{:02X}", byte).chars().collect(),
        })
        .collect()
}

fn redact_database_url(database_url: &str) -> String {
    if let Some((scheme, rest)) = database_url.split_once("://") {
        if let Some((userinfo, tail)) = rest.split_once('@') {
            let redacted_userinfo = match userinfo.split_once(':') {
                Some((user, _)) => format!("{user}:***"),
                None => "***".to_string(),
            };
            return format!("{scheme}://{redacted_userinfo}@{tail}");
        }
    }

    "<redacted>".to_string()
}
