pub const PRODUCT_NAME: &str = "synthetic-pop";
pub const PROJECT_PROMISE: &str = "offline deterministic synthetic community generation";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneStatus {
    ScaffoldOnly,
    RustCoreFoundation,
}

impl MilestoneStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ScaffoldOnly => "milestone 1 scaffold",
            Self::RustCoreFoundation => "milestone 2 rust core foundation",
        }
    }
}

#[must_use]
pub const fn current_status() -> MilestoneStatus {
    MilestoneStatus::RustCoreFoundation
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreEngineConfig {
    pub offline_only: bool,
}

impl Default for CoreEngineConfig {
    fn default() -> Self {
        Self { offline_only: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreCapabilities {
    pub offline_core_available: bool,
    pub local_core_available: bool,
    pub generation_available: bool,
    pub policy_available: bool,
    pub export_available: bool,
    pub bindings_available: bool,
    pub desktop_available: bool,
    pub wasm_available: bool,
}

impl CoreCapabilities {
    #[must_use]
    pub const fn rust_core_foundation() -> Self {
        Self {
            offline_core_available: true,
            local_core_available: true,
            generation_available: false,
            policy_available: false,
            export_available: false,
            bindings_available: false,
            desktop_available: false,
            wasm_available: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreEngine {
    config: CoreEngineConfig,
}

impl CoreEngine {
    #[must_use]
    pub const fn new(config: CoreEngineConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub const fn config(&self) -> &CoreEngineConfig {
        &self.config
    }

    #[must_use]
    pub const fn capabilities(&self) -> CoreCapabilities {
        CoreCapabilities::rust_core_foundation()
    }
}

impl Default for CoreEngine {
    fn default() -> Self {
        Self::new(CoreEngineConfig::default())
    }
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
    fn current_status_reports_rust_core_foundation() {
        assert_eq!(current_status(), MilestoneStatus::RustCoreFoundation);
        assert_eq!(current_status().label(), "milestone 2 rust core foundation");
    }

    #[test]
    fn constructs_core_engine_with_default_config() {
        let engine = CoreEngine::default();

        assert_eq!(engine.config(), &CoreEngineConfig { offline_only: true });
    }

    #[test]
    fn constructs_core_engine_with_explicit_config() {
        let config = CoreEngineConfig {
            offline_only: false,
        };
        let engine = CoreEngine::new(config);

        assert_eq!(engine.config(), &config);
    }

    #[test]
    fn reports_foundation_capabilities_truthfully() {
        let capabilities = CoreEngine::default().capabilities();

        assert!(capabilities.offline_core_available);
        assert!(capabilities.local_core_available);
        assert!(!capabilities.generation_available);
        assert!(!capabilities.policy_available);
        assert!(!capabilities.export_available);
        assert!(!capabilities.bindings_available);
        assert!(!capabilities.desktop_available);
        assert!(!capabilities.wasm_available);
    }
}
