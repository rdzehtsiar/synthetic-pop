pub const PRODUCT_NAME: &str = "synthetic-pop";
pub const PROJECT_PROMISE: &str = "offline deterministic synthetic community generation";
pub const MAX_GENERATION_SIZE: usize = 100_000;

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
pub struct Seed(String);

impl Seed {
    pub fn new(value: impl Into<String>) -> Result<Self, CoreValidationError> {
        let value = value.into();

        if value.trim().is_empty() {
            return Err(CoreValidationError::EmptySeed);
        }

        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationSize(usize);

impl GenerationSize {
    pub const MAX: usize = MAX_GENERATION_SIZE;

    pub const fn new(value: usize) -> Result<Self, CoreValidationError> {
        if value == 0 {
            return Err(CoreValidationError::ZeroGenerationSize);
        }

        if value > Self::MAX {
            return Err(CoreValidationError::GenerationSizeTooLarge {
                requested: value,
                max: Self::MAX,
            });
        }

        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreRunRequest {
    seed: Seed,
    size: GenerationSize,
}

impl CoreRunRequest {
    pub fn new(seed: impl Into<String>, size: usize) -> Result<Self, CoreValidationError> {
        Ok(Self {
            seed: Seed::new(seed)?,
            size: GenerationSize::new(size)?,
        })
    }

    #[must_use]
    pub const fn seed(&self) -> &Seed {
        &self.seed
    }

    #[must_use]
    pub const fn size(&self) -> GenerationSize {
        self.size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreValidationError {
    EmptySeed,
    ZeroGenerationSize,
    GenerationSizeTooLarge { requested: usize, max: usize },
}

impl std::fmt::Display for CoreValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySeed => formatter.write_str("seed must not be empty"),
            Self::ZeroGenerationSize => {
                formatter.write_str("generation size must be greater than zero")
            }
            Self::GenerationSizeTooLarge { requested, max } => {
                write!(
                    formatter,
                    "generation size {requested} exceeds maximum of {max}"
                )
            }
        }
    }
}

impl std::error::Error for CoreValidationError {}

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

    pub fn validate_run_request(
        &self,
        seed: impl Into<String>,
        size: usize,
    ) -> Result<CoreRunRequest, CoreValidationError> {
        CoreRunRequest::new(seed, size)
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

    #[test]
    fn accepts_valid_seed() {
        let seed = Seed::new("stable-seed-1").expect("seed should be valid");

        assert_eq!(seed.as_str(), "stable-seed-1");
    }

    #[test]
    fn rejects_empty_seed() {
        assert_eq!(Seed::new(""), Err(CoreValidationError::EmptySeed));
        assert_eq!(Seed::new("   "), Err(CoreValidationError::EmptySeed));
    }

    #[test]
    fn accepts_valid_generation_size() {
        let size = GenerationSize::new(42).expect("size should be valid");

        assert_eq!(size.get(), 42);
    }

    #[test]
    fn rejects_zero_generation_size() {
        assert_eq!(
            GenerationSize::new(0),
            Err(CoreValidationError::ZeroGenerationSize)
        );
    }

    #[test]
    fn accepts_generation_size_at_upper_boundary() {
        let size = GenerationSize::new(MAX_GENERATION_SIZE).expect("max size should be valid");

        assert_eq!(size.get(), MAX_GENERATION_SIZE);
    }

    #[test]
    fn rejects_generation_size_above_upper_boundary() {
        assert_eq!(
            GenerationSize::new(MAX_GENERATION_SIZE + 1),
            Err(CoreValidationError::GenerationSizeTooLarge {
                requested: MAX_GENERATION_SIZE + 1,
                max: MAX_GENERATION_SIZE,
            })
        );
    }

    #[test]
    fn builds_valid_core_run_request() {
        let request = CoreRunRequest::new("repeatable-run", 10).expect("request should be valid");

        assert_eq!(request.seed().as_str(), "repeatable-run");
        assert_eq!(request.size().get(), 10);
    }

    #[test]
    fn core_engine_validates_run_requests() {
        let engine = CoreEngine::default();

        assert!(engine.validate_run_request("seed", 1).is_ok());
        assert_eq!(
            engine.validate_run_request("", 1),
            Err(CoreValidationError::EmptySeed)
        );
        assert_eq!(
            engine.validate_run_request("seed", 0),
            Err(CoreValidationError::ZeroGenerationSize)
        );
    }
}
