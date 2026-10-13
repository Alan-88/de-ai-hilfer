use crate::{batch_models::BatchJob, db::DbPool};
use serde_json::Value;
use sqlx::PgConnection;

pub async fn submit(
    pool: &DbPool,
    id: &str,
    key: &str,
    request: &Value,
    steps: &Value,
) -> anyhow::Result<String> {
    let mut tx = pool.begin().await?;
    let available: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_xact_lock_shared(hashtextextended('database-snapshot',0))",
    )
    .fetch_one(&mut *tx)
    .await?;
    anyhow::ensure!(available, "database restore is still active");
    let row: (String, Value) = sqlx::query_as(
        "INSERT INTO agent_batches (id, idempotency_key, request, steps) VALUES ($1,$2,$3,$4) ON CONFLICT(idempotency_key) DO UPDATE SET idempotency_key=EXCLUDED.idempotency_key RETURNING id, request"
    ).bind(id).bind(key).bind(request).bind(steps).fetch_one(&mut *tx).await?;
    anyhow::ensure!(
        &row.1 == request,
        "idempotency_key already belongs to a different request"
    );
    tx.commit().await?;
    Ok(row.0)
}

pub async fn get(pool: &DbPool, id: &str) -> anyhow::Result<BatchJob> {
    sqlx::query_as(
        "SELECT id,status,steps,error,created_at,updated_at FROM agent_batches WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("batch not found"))
}

pub async fn list(pool: &DbPool, limit: i64) -> Result<Vec<BatchJob>, sqlx::Error> {
    sqlx::query_as("SELECT id,status,steps,error,created_at,updated_at FROM agent_batches ORDER BY created_at DESC LIMIT $1")
        .bind(limit.clamp(1,100)).fetch_all(pool).await
}

pub async fn pending(pool: &DbPool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM agent_batches WHERE status IN ('queued','running') ORDER BY created_at LIMIT 100")
        .fetch_all(pool).await
}

pub async fn request(pool: &DbPool, id: &str) -> Result<Value, sqlx::Error> {
    sqlx::query_scalar("SELECT request FROM agent_batches WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn update(
    pool: &DbPool,
    id: &str,
    status: &str,
    steps: &Value,
    error: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE agent_batches SET status=$2,steps=$3,error=$4,updated_at=NOW() WHERE id=$1",
    )
    .bind(id)
    .bind(status)
    .bind(steps)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn receipt(
    connection: &mut PgConnection,
    key: &str,
) -> Result<Option<Value>, sqlx::Error> {
    sqlx::query_scalar("SELECT result FROM agent_operation_receipts WHERE operation_key=$1")
        .bind(key)
        .fetch_optional(connection)
        .await
}

pub async fn save_receipt(
    connection: &mut PgConnection,
    key: &str,
    result: &Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO agent_operation_receipts(operation_key,result) VALUES($1,$2)")
        .bind(key)
        .bind(result)
        .execute(connection)
        .await?;
    Ok(())
}

pub async fn lock_operation(connection: &mut PgConnection, key: &str) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(key)
        .execute(connection)
        .await?;
    Ok(())
}

pub async fn step_result(
    pool: &DbPool,
    job: &str,
    step: usize,
    value: &Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE agent_batches SET steps=jsonb_set(steps,$2,$3),updated_at=NOW() WHERE id=$1",
    )
    .bind(job)
    .bind(vec![step.to_string(), "result".into()])
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn item_result(
    pool: &DbPool,
    job: &str,
    step: usize,
    item: usize,
    value: &Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE agent_batches SET steps=jsonb_set(steps,$2,$3),updated_at=NOW() WHERE id=$1",
    )
    .bind(job)
    .bind(vec![
        step.to_string(),
        "result".into(),
        "results".into(),
        item.to_string(),
    ])
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}
