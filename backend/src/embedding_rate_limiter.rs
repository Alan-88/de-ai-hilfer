use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug)]
pub(crate) struct SmoothRateLimiter {
    permits_per_minute: usize,
    label: &'static str,
    next_allowed_at: Mutex<Instant>,
}

impl SmoothRateLimiter {
    pub(crate) fn new(label: &'static str, permits_per_minute: usize) -> Self {
        Self {
            permits_per_minute,
            label,
            next_allowed_at: Mutex::new(Instant::now()),
        }
    }

    pub(crate) async fn acquire(&self, permits: usize) {
        if permits == 0 {
            return;
        }

        let reserved_span =
            Duration::from_secs_f64((60.0 * permits as f64) / self.permits_per_minute as f64);

        let wait = {
            let mut next_allowed_at = self.next_allowed_at.lock().await;
            let now = Instant::now();
            let start_at = (*next_allowed_at).max(now);
            let wait = start_at.saturating_duration_since(now);
            *next_allowed_at = start_at + reserved_span;
            wait
        };

        if !wait.is_zero() {
            tracing::info!(
                "embedding rate limiter waiting: label={}, wait_ms={}, reserved_permits={}, permits_per_minute={}",
                self.label,
                wait.as_millis(),
                permits,
                self.permits_per_minute
            );
            tokio::time::sleep(wait).await;
        }
    }
}
