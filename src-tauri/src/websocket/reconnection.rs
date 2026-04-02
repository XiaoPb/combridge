#[derive(Debug, Clone)]
pub struct ReconnectionStrategy {
    pub base_interval_ms: u64,
    pub max_interval_ms: u64,
    pub max_attempts: u32,
    pub multiplier: f64,
}

impl ReconnectionStrategy {
    pub fn new(base_interval_ms: u64, max_attempts: u32) -> Self {
        Self {
            base_interval_ms,
            max_interval_ms: 60000,
            max_attempts,
            multiplier: 1.5,
        }
    }

    pub fn get_delay(&self, attempt: u32) -> u64 {
        if attempt == 0 {
            return 0;
        }

        let delay = self.base_interval_ms as f64
            * self
                .multiplier
                .powi(attempt as i32 - 1);

        delay.min(self.max_interval_ms as f64) as u64
    }

    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }

    pub fn reset(&mut self) {}
}

impl Default for ReconnectionStrategy {
    fn default() -> Self {
        Self::new(5000, 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconnection_delay() {
        let strategy = ReconnectionStrategy::new(1000, 5);

        assert_eq!(strategy.get_delay(0), 0);
        assert_eq!(strategy.get_delay(1), 1000);
        assert_eq!(strategy.get_delay(2), 1500);
        assert_eq!(strategy.get_delay(3), 2250);
    }

    #[test]
    fn test_max_delay() {
        let strategy = ReconnectionStrategy {
            base_interval_ms: 1000,
            max_interval_ms: 5000,
            max_attempts: 10,
            multiplier: 2.0,
        };

        assert_eq!(strategy.get_delay(1), 1000);
        assert_eq!(strategy.get_delay(2), 2000);
        assert_eq!(strategy.get_delay(3), 4000);
        assert_eq!(strategy.get_delay(4), 5000);
        assert_eq!(strategy.get_delay(5), 5000);
    }

    #[test]
    fn test_should_retry() {
        let strategy = ReconnectionStrategy::new(1000, 3);

        assert!(strategy.should_retry(0));
        assert!(strategy.should_retry(1));
        assert!(strategy.should_retry(2));
        assert!(!strategy.should_retry(3));
        assert!(!strategy.should_retry(4));
    }
}
