#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportState {
    NotImplemented,
}

#[must_use]
pub const fn current_export_state() -> ExportState {
    ExportState::NotImplemented
}

#[must_use]
pub const fn export_enabled() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_is_not_claimed_as_implemented() {
        assert_eq!(current_export_state(), ExportState::NotImplemented);
        assert!(!export_enabled());
    }
}

