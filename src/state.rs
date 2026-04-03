#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Idle,
    CountingCoins(u64), // Amount in cents
    WithdrawReady(u64), // Amount in cents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_variants() {
        let idle = AppState::Idle;
        let counting = AppState::CountingCoins(150);
        let withdraw = AppState::WithdrawReady(200);

        assert_eq!(idle, AppState::Idle);
        assert_eq!(counting, AppState::CountingCoins(150));
        assert_ne!(counting, AppState::CountingCoins(100));
        assert_ne!(withdraw.clone(), idle);
        assert_eq!(withdraw, AppState::WithdrawReady(200));
    }

    #[test]
    fn test_state_clone() {
        let original = AppState::WithdrawReady(500);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_state_debug() {
        let state = AppState::CountingCoins(42);
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("CountingCoins"));
        assert!(debug_str.contains("42"));
    }
}
