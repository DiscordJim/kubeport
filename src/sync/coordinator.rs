use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

pub struct AsyncCoordinator {
    decision: AtomicBool,
    notifier: Notify
}

impl AsyncCoordinator {
    pub fn new() -> Self {
        Self {
            decision: AtomicBool::default(),
            notifier: Notify::default()
        }
    }
    pub fn shutdown(&self) {
        self.change_state(true);
    }
    pub fn change_state(&self, new_state: bool) {
        let _ = self.decision.compare_exchange(!new_state, new_state, Ordering::Release, Ordering::Relaxed);
        self.notifier.notify_waiters();
    }
    pub async fn wait_on_change(&self) {
        self.notifier.notified().await;
    }
    pub async fn on_state_change(&self) -> bool {
        self.wait_on_change().await;
        self.decision.load(Ordering::Acquire)
    }
}