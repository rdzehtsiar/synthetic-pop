#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyState {
    NotImplemented,
}

#[must_use]
pub const fn current_policy_state() -> PolicyState {
    PolicyState::NotImplemented
}

#[must_use]
pub const fn policy_generation_enabled() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_is_not_claimed_as_implemented() {
        assert_eq!(current_policy_state(), PolicyState::NotImplemented);
        assert!(!policy_generation_enabled());
    }
}

