use crate::db::DbPool;
use serde::{de::DeserializeOwned, Serialize};

pub async fn get<T: DeserializeOwned>(pool: &DbPool, key: &str) -> anyhow::Result<Option<T>> {
    let value: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT value FROM analysis_checkpoints WHERE cache_key=$1 AND expires_at>NOW()",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;
    value
        .map(serde_json::from_value)
        .transpose()
        .map_err(Into::into)
}

pub async fn put<T: Serialize>(
    pool: &DbPool,
    key: &str,
    stage: &str,
    value: &T,
) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO analysis_checkpoints(cache_key,stage,value,expires_at) VALUES($1,$2,$3,NOW()+INTERVAL '24 hours') ON CONFLICT(cache_key) DO UPDATE SET stage=EXCLUDED.stage,value=EXCLUDED.value,expires_at=EXCLUDED.expires_at")
        .bind(key).bind(stage).bind(serde_json::to_value(value)?).execute(pool).await?;
    Ok(())
}

pub async fn cleanup(pool: &DbPool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM analysis_checkpoints WHERE expires_at<=NOW()")
        .execute(pool)
        .await?;
    Ok(())
}
