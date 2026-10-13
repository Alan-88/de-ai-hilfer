use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use tokio::sync::{OwnedMutexGuard, OwnedSemaphorePermit, Semaphore};

// The lock covers the cache recheck through persistence, not just the AI request.
pub async fn lock_word(word: &str) -> OwnedMutexGuard<()> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>> = OnceLock::new();
    let lock = {
        let mut locks = LOCKS
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        locks.retain(|_, lock| lock.strong_count() > 0);
        let lock = locks
            .get(word)
            .and_then(Weak::upgrade)
            .unwrap_or_else(|| Arc::new(tokio::sync::Mutex::new(())));
        locks.insert(word.to_string(), Arc::downgrade(&lock));
        lock
    };
    lock.lock_owned().await
}

pub async fn generation_slot() -> OwnedSemaphorePermit {
    static SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
    SLOTS
        .get_or_init(|| Arc::new(Semaphore::new(3)))
        .clone()
        .acquire_owned()
        .await
        .expect("generation semaphore remains open")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn same_word_waits_while_another_word_runs() {
        let first = lock_word("same").await;
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), lock_word("same"))
                .await
                .is_err()
        );
        let other = tokio::time::timeout(std::time::Duration::from_millis(20), lock_word("other"))
            .await
            .unwrap();
        drop(first);
        drop(other);
        tokio::time::timeout(std::time::Duration::from_millis(20), lock_word("same"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn generation_stops_at_three_concurrent_words() {
        let one = generation_slot().await;
        let two = generation_slot().await;
        let three = generation_slot().await;
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), generation_slot())
                .await
                .is_err()
        );
        drop(one);
        let _slot = tokio::time::timeout(std::time::Duration::from_millis(20), generation_slot())
            .await
            .unwrap();
        drop(two);
        drop(three);
    }
}
