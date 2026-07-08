use crate::models::QualityMode;
use crate::state::AppState;

pub fn primary_model_for(state: &AppState, quality_mode: QualityMode) -> &str {
    match quality_mode {
        QualityMode::Default => state.config.ai_models.analyze.as_str(),
        QualityMode::Pro => state.config.ai_models.analyze_pro.as_str(),
    }
}

pub fn fallback_model_for(state: &AppState, quality_mode: QualityMode) -> &str {
    match quality_mode {
        QualityMode::Default => state.config.ai_models.fallback_fast.as_str(),
        QualityMode::Pro => state.config.ai_models.fallback_pro.as_str(),
    }
}
