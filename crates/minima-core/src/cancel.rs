//! Cancellation, latched. Producers set the flag; samplers read it or await it.
//!
//! The Esc watcher and a signal handler are both producers, so cancellation cannot live in the
//! terminal layer. `Cancel::cancelled()` is the future the agent loop passes to `select!`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

#[derive(Clone, Default)]
pub struct Cancel {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    flag: AtomicBool,
    notify: Notify,
}

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.inner.flag.store(true, Ordering::SeqCst);
        self.inner.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.flag.load(Ordering::SeqCst)
    }

    /// Clear the latch between user turns. One Esc cancels one user turn, not the session.
    pub fn reset(&self) {
        self.inner.flag.store(false, Ordering::SeqCst);
    }

    /// Resolves once cancelled. Registers the waiter before re-checking the flag, so a
    /// `cancel()` racing this call cannot be missed.
    pub async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            let waiting = self.inner.notify.notified();
            if self.is_cancelled() {
                return;
            }
            waiting.await;
        }
    }
}
