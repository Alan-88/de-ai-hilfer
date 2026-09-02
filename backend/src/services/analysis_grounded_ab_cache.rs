use crate::models::{AiModelOverride, QualityMode};
use crate::services::analysis_grounded_model_a::ModelAOutput;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundedAbCache {
    pub query_text: String,
    pub dictionary_facts: Option<String>,
    pub model_a_output: ModelAOutput,
    pub stage2_markdown: String,
    pub stage2_model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroundedAbCacheKey {
    query_text: String,
    quality_mode: &'static str,
    generation_hint: String,
    analysis_model: String,
    provider_name: String,
    allow_stage2_fallback: bool,
}

impl GroundedAbCacheKey {
    pub fn new(
        query_text: &str,
        quality_mode: QualityMode,
        generation_hint: Option<&str>,
        analysis_model: &str,
        model_override: Option<&AiModelOverride>,
        allow_stage2_fallback: bool,
    ) -> Self {
        Self {
            query_text: query_text.to_string(),
            quality_mode: match quality_mode {
                QualityMode::Default => "default",
                QualityMode::Pro => "pro",
            },
            generation_hint: generation_hint.unwrap_or_default().trim().to_string(),
            analysis_model: analysis_model.to_string(),
            provider_name: model_override
                .map(|model| model.provider_name.trim().to_string())
                .unwrap_or_default(),
            allow_stage2_fallback,
        }
    }
}

struct CachedGroundedAb {
    value: GroundedAbCache,
    cached_at: Instant,
}

#[derive(Default)]
struct GroundedAbCacheStore {
    entries: HashMap<GroundedAbCacheKey, CachedGroundedAb>,
}

impl GroundedAbCacheStore {
    fn get(&mut self, key: &GroundedAbCacheKey, now: Instant) -> Option<GroundedAbCache> {
        self.remove_expired(now);
        self.entries.get(key).map(|entry| entry.value.clone())
    }

    fn put(&mut self, key: GroundedAbCacheKey, value: GroundedAbCache, now: Instant) {
        self.remove_expired(now);
        self.entries.insert(
            key,
            CachedGroundedAb {
                value,
                cached_at: now,
            },
        );
    }

    fn remove(&mut self, key: &GroundedAbCacheKey) {
        self.entries.remove(key);
    }

    fn remove_expired(&mut self, now: Instant) {
        self.entries
            .retain(|_, entry| now.saturating_duration_since(entry.cached_at) < CACHE_TTL);
    }
}

fn cache_store() -> &'static Mutex<GroundedAbCacheStore> {
    static CACHE: OnceLock<Mutex<GroundedAbCacheStore>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(GroundedAbCacheStore::default()))
}

pub fn get_grounded_ab_cache(key: &GroundedAbCacheKey) -> Option<GroundedAbCache> {
    cache_store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(key, Instant::now())
}

pub fn put_grounded_ab_cache(key: GroundedAbCacheKey, value: GroundedAbCache) {
    cache_store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .put(key, value, Instant::now());
}

pub fn remove_grounded_ab_cache(key: &GroundedAbCacheKey) {
    cache_store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(key);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache_value() -> GroundedAbCache {
        GroundedAbCache {
            query_text: "Wahl".to_string(),
            dictionary_facts: None,
            model_a_output: ModelAOutput::default(),
            stage2_markdown: "complete".to_string(),
            stage2_model: "model-b".to_string(),
        }
    }

    #[test]
    fn keeps_ab_for_retry_until_success_removes_it() {
        let now = Instant::now();
        let key =
            GroundedAbCacheKey::new("Wahl", QualityMode::Default, None, "model-b", None, false);
        let fallback_key =
            GroundedAbCacheKey::new("Wahl", QualityMode::Default, None, "model-b", None, true);
        let mut store = GroundedAbCacheStore::default();

        assert_ne!(key, fallback_key);
        store.put(key.clone(), cache_value(), now);
        assert!(store.get(&key, now).is_some());

        store.remove(&key);
        assert!(store.get(&key, now).is_none());
    }
}
