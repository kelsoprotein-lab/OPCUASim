use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial_delay_ms: 1000,
            max_delay_ms: 60_000,
            backoff_factor: 2.0,
            max_attempts: None,
        }
    }
}

impl ReconnectPolicy {
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay_ms as f64 * self.backoff_factor.powi(attempt as i32);
        let clamped = delay.min(self.max_delay_ms as f64) as u64;
        Duration::from_millis(clamped)
    }

    pub fn should_retry(&self, attempt: u32) -> bool {
        match self.max_attempts {
            Some(max) => attempt < max,
            None => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReconnectState {
    Idle,
    Reconnecting { attempt: u32 },
    GaveUp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let policy = ReconnectPolicy::default();
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(1000));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(2000));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(4000));
    }

    #[test]
    fn test_max_delay_cap() {
        let policy = ReconnectPolicy::default();
        assert_eq!(policy.delay_for_attempt(10), Duration::from_millis(60_000));
    }

    #[test]
    fn test_should_retry_unlimited() {
        let policy = ReconnectPolicy::default();
        assert!(policy.should_retry(100));
    }

    #[test]
    fn test_should_retry_limited() {
        let policy = ReconnectPolicy {
            max_attempts: Some(3),
            ..Default::default()
        };
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
    }
}
