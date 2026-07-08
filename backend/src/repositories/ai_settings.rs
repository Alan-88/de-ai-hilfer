use crate::db::DbPool;
use crate::models::{AiProviderProfileInput, AiTaskModelSettingInput};
use crate::models::{AiProviderProfileRecord, AiTaskModelSettingRecord};
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};

pub async fn list_provider_profiles(pool: &DbPool) -> Result<Vec<AiProviderProfileRecord>> {
    sqlx::query_as::<_, AiProviderProfileRecord>(
        r#"
        SELECT id, name, base_url, api_key, model_ids, is_default, created_at, updated_at
        FROM ai_provider_profiles
        ORDER BY is_default DESC, lower(name) ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn list_task_model_settings(pool: &DbPool) -> Result<Vec<AiTaskModelSettingRecord>> {
    sqlx::query_as::<_, AiTaskModelSettingRecord>(
        r#"
        SELECT task_key, provider_id, model_id, inherit_task_key, updated_at
        FROM ai_task_model_settings
        ORDER BY task_key ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn replace_ai_settings(
    pool: &DbPool,
    profiles: &[AiProviderProfileInput],
    task_settings: &[AiTaskModelSettingInput],
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE ai_provider_profiles SET is_default = FALSE")
        .execute(&mut *tx)
        .await?;

    let mut saved_ids = Vec::new();
    let mut provider_ids_by_name = HashMap::new();
    let mut seen_names = HashSet::new();

    for profile in profiles {
        let name = profile.name.trim();
        let base_url = profile.base_url.trim();
        if name.is_empty() {
            return Err(anyhow!("provider name cannot be empty"));
        }
        if base_url.is_empty() {
            return Err(anyhow!("provider base_url cannot be empty"));
        }
        if !seen_names.insert(name.to_ascii_lowercase()) {
            return Err(anyhow!("duplicate provider name: {name}"));
        }

        let model_ids = profile
            .model_ids
            .iter()
            .map(|model| model.trim().to_string())
            .filter(|model| !model.is_empty())
            .collect::<Vec<_>>();

        let id = if let Some(id) = profile.id {
            sqlx::query_scalar::<_, i64>(
                r#"
                UPDATE ai_provider_profiles
                SET name = $2,
                    base_url = $3,
                    api_key = COALESCE($4, api_key),
                    model_ids = $5,
                    is_default = $6
                WHERE id = $1
                RETURNING id
                "#,
            )
            .bind(id)
            .bind(name)
            .bind(base_url)
            .bind(profile.api_key.as_deref())
            .bind(&model_ids)
            .bind(profile.is_default)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("provider profile not found: {id}"))?
        } else {
            sqlx::query_scalar::<_, i64>(
                r#"
                INSERT INTO ai_provider_profiles (name, base_url, api_key, model_ids, is_default)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id
                "#,
            )
            .bind(name)
            .bind(base_url)
            .bind(profile.api_key.as_deref().unwrap_or_default())
            .bind(&model_ids)
            .bind(profile.is_default)
            .fetch_one(&mut *tx)
            .await?
        };

        saved_ids.push(id);
        provider_ids_by_name.insert(name.to_string(), id);
    }

    if saved_ids.is_empty() {
        sqlx::query("DELETE FROM ai_task_model_settings")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM ai_provider_profiles")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(());
    }

    sqlx::query("DELETE FROM ai_provider_profiles WHERE NOT (id = ANY($1))")
        .bind(&saved_ids)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM ai_task_model_settings")
        .execute(&mut *tx)
        .await?;

    for setting in task_settings {
        let provider_id = match setting.inherit_task_key.as_deref() {
            Some(_) => None,
            None => {
                let provider_name = setting
                    .provider_name
                    .as_deref()
                    .ok_or_else(|| anyhow!("provider_name is required for {}", setting.task_key))?;
                Some(
                    *provider_ids_by_name
                        .get(provider_name)
                        .ok_or_else(|| anyhow!("unknown provider profile: {provider_name}"))?,
                )
            }
        };

        sqlx::query(
            r#"
            INSERT INTO ai_task_model_settings (
                task_key, provider_id, model_id, inherit_task_key
            )
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(setting.task_key.trim())
        .bind(provider_id)
        .bind(setting.model_id.as_deref().map(str::trim))
        .bind(setting.inherit_task_key.as_deref().map(str::trim))
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
