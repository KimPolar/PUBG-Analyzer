use std::{sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::Instant};

#[derive(Clone)]
pub struct RpmLimiter {
    interval: Duration,
    next_allowed: Arc<Mutex<Instant>>,
}

impl RpmLimiter {
    #[must_use]
    pub fn new(rpm: u32) -> Self {
        let rpm = rpm.max(1);
        Self {
            interval: Duration::from_secs_f64(60.0 / f64::from(rpm)),
            next_allowed: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub async fn acquire(&self) {
        let wait = {
            let mut next = self.next_allowed.lock().await;
            let now = Instant::now();
            let scheduled = (*next).max(now);
            *next = scheduled + self.interval;
            scheduled.saturating_duration_since(now)
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}
