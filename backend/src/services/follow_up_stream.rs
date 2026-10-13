use crate::{
    models::FollowUpRequest,
    services::{
        follow_up,
        stream_response::{sse_complete, sse_delta},
    },
    state::AppState,
};
use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

pub async fn stream_follow_up(
    state: AppState,
    request: FollowUpRequest,
    tx: UnboundedSender<String>,
) -> Result<()> {
    let response = follow_up::create_with_progress(&state, request, Some(&tx), None).await?;
    // Tool turns are private; only a validated complete answer is sent to the browser.
    let _ = tx.send(sse_delta(&response.answer));
    let _ = tx.send(sse_complete(&response));
    Ok(())
}
