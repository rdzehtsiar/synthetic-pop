pub const PRODUCT_NAME: &str = "synthetic-pop";
pub const PROJECT_PROMISE: &str =
    "offline deterministic synthetic community generation";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneStatus {
    ScaffoldOnly,
}

impl MilestoneStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ScaffoldOnly => "milestone 1 scaffold",
        }
    }
}

#[must_use]
pub const fn current_status() -> MilestoneStatus {
    MilestoneStatus::ScaffoldOnly
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_product_identity() {
        assert_eq!(PRODUCT_NAME, "synthetic-pop");
        assert!(PROJECT_PROMISE.contains("offline"));
    }

    #[test]
    fn current_status_is_scaffold_only() {
        assert_eq!(current_status().label(), "milestone 1 scaffold");
    }
}

