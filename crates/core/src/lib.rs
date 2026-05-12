pub mod model;
pub use model::*;

pub const PRODUCT_NAME: &str = "synthetic-pop";
pub const PROJECT_PROMISE: &str = "offline deterministic synthetic community generation";
pub const MAX_GENERATION_SIZE: usize = 100_000;
pub const REPRODUCIBILITY_METADATA_VERSION: u32 = 1;
pub const DETERMINISTIC_RNG_ALGORITHM_VERSION: u32 = 1;
/// Stable deterministic RNG algorithm identifier.
///
/// Version 1 serializes every UTF-8 input component as a little-endian `u64`
/// byte length followed by raw bytes, hashes those bytes with fixed FNV-1a-64
/// constants, and applies the SplitMix64 finalizer for the returned `u64`.
pub const DETERMINISTIC_RNG_ALGORITHM: &str =
    "synthetic-pop-deterministic-rng-v1:length-delimited-utf8+fnv1a64+splitmix64";

const FNV1A64_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A64_PRIME: u64 = 0x0000_0100_0000_01b3;
const SPLITMIX64_GAMMA: u64 = 0x9e37_79b9_7f4a_7c15;
const SPLITMIX64_MIX_1: u64 = 0xbf58_476d_1ce4_e5b9;
const SPLITMIX64_MIX_2: u64 = 0x94d0_49bb_1331_11eb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneStatus {
    ScaffoldOnly,
    RustCoreFoundation,
    DeterministicGenerationSystem,
    CanonicalDataModel,
}

impl MilestoneStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ScaffoldOnly => "milestone 1 scaffold",
            Self::RustCoreFoundation => "milestone 2 rust core foundation",
            Self::DeterministicGenerationSystem => "milestone 3 deterministic generation system",
            Self::CanonicalDataModel => "milestone 4 canonical data model",
        }
    }
}

#[must_use]
pub const fn current_status() -> MilestoneStatus {
    MilestoneStatus::CanonicalDataModel
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReproducibilityMetadata {
    pub metadata_version: u32,
    pub deterministic_rng_algorithm: &'static str,
    pub deterministic_rng_algorithm_version: u32,
}

impl ReproducibilityMetadata {
    #[must_use]
    pub const fn current() -> Self {
        Self {
            metadata_version: REPRODUCIBILITY_METADATA_VERSION,
            deterministic_rng_algorithm: DETERMINISTIC_RNG_ALGORITHM,
            deterministic_rng_algorithm_version: DETERMINISTIC_RNG_ALGORITHM_VERSION,
        }
    }
}

#[must_use]
pub const fn reproducibility_metadata() -> ReproducibilityMetadata {
    ReproducibilityMetadata::current()
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
    pub deterministic_rng_available: bool,
    pub canonical_data_model_available: bool,
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
            deterministic_rng_available: false,
            canonical_data_model_available: false,
            generation_available: false,
            policy_available: false,
            export_available: false,
            bindings_available: false,
            desktop_available: false,
            wasm_available: false,
        }
    }

    #[must_use]
    pub const fn deterministic_generation_system() -> Self {
        Self {
            offline_core_available: true,
            local_core_available: true,
            deterministic_rng_available: true,
            canonical_data_model_available: false,
            generation_available: false,
            policy_available: false,
            export_available: false,
            bindings_available: false,
            desktop_available: false,
            wasm_available: false,
        }
    }

    #[must_use]
    pub const fn canonical_data_model() -> Self {
        Self {
            offline_core_available: true,
            local_core_available: true,
            deterministic_rng_available: true,
            canonical_data_model_available: true,
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

impl AsRef<str> for Seed {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Deterministic field-level randomness derived from seed, namespace, entity,
/// and field inputs without shared mutable RNG state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeterministicRandom<'a> {
    seed: &'a str,
    namespace: &'a str,
    entity_id: &'a str,
    field: &'a str,
}

impl<'a> DeterministicRandom<'a> {
    #[must_use]
    pub const fn new(
        seed: &'a str,
        namespace: &'a str,
        entity_id: &'a str,
        field: &'a str,
    ) -> Self {
        Self {
            seed,
            namespace,
            entity_id,
            field,
        }
    }

    #[must_use]
    pub const fn seed(self) -> &'a str {
        self.seed
    }

    #[must_use]
    pub const fn namespace(self) -> &'a str {
        self.namespace
    }

    #[must_use]
    pub const fn entity_id(self) -> &'a str {
        self.entity_id
    }

    #[must_use]
    pub const fn field(self) -> &'a str {
        self.field
    }

    #[must_use]
    pub fn u64(self) -> u64 {
        self.u64_at(0)
    }

    #[must_use]
    pub fn bounded_u64(self, upper_bound: u64) -> Option<u64> {
        if upper_bound == 0 {
            return None;
        }

        let threshold = upper_bound.wrapping_neg() % upper_bound;
        let mut counter = 0;

        loop {
            let random = self.u64_at(counter);
            let product = u128::from(random) * u128::from(upper_bound);
            let low = product as u64;

            if low >= threshold {
                return Some((product >> 64) as u64);
            }

            counter = counter.wrapping_add(1);
        }
    }

    #[must_use]
    pub fn bool(self) -> bool {
        self.u64() & 1 == 1
    }

    #[must_use]
    pub fn choose_index<T>(self, values: &[T]) -> Option<usize> {
        let upper_bound = u64::try_from(values.len()).ok()?;
        let index = self.bounded_u64(upper_bound)?;

        usize::try_from(index).ok()
    }

    #[must_use]
    pub fn choose<T>(self, values: &[T]) -> Option<&T> {
        self.choose_index(values).map(|index| &values[index])
    }

    fn u64_at(self, counter: u64) -> u64 {
        deterministic_u64_at(
            self.seed,
            self.namespace,
            self.entity_id,
            self.field,
            counter,
        )
    }
}

#[must_use]
pub fn random_u64(
    seed: impl AsRef<str>,
    namespace: impl AsRef<str>,
    entity_id: impl AsRef<str>,
    field: impl AsRef<str>,
) -> u64 {
    DeterministicRandom::new(
        seed.as_ref(),
        namespace.as_ref(),
        entity_id.as_ref(),
        field.as_ref(),
    )
    .u64()
}

#[must_use]
pub fn random_bounded_u64(
    seed: impl AsRef<str>,
    namespace: impl AsRef<str>,
    entity_id: impl AsRef<str>,
    field: impl AsRef<str>,
    upper_bound: u64,
) -> Option<u64> {
    DeterministicRandom::new(
        seed.as_ref(),
        namespace.as_ref(),
        entity_id.as_ref(),
        field.as_ref(),
    )
    .bounded_u64(upper_bound)
}

#[must_use]
pub fn random_bool(
    seed: impl AsRef<str>,
    namespace: impl AsRef<str>,
    entity_id: impl AsRef<str>,
    field: impl AsRef<str>,
) -> bool {
    DeterministicRandom::new(
        seed.as_ref(),
        namespace.as_ref(),
        entity_id.as_ref(),
        field.as_ref(),
    )
    .bool()
}

#[must_use]
pub fn random_index<T>(
    seed: impl AsRef<str>,
    namespace: impl AsRef<str>,
    entity_id: impl AsRef<str>,
    field: impl AsRef<str>,
    values: &[T],
) -> Option<usize> {
    DeterministicRandom::new(
        seed.as_ref(),
        namespace.as_ref(),
        entity_id.as_ref(),
        field.as_ref(),
    )
    .choose_index(values)
}

#[must_use]
pub fn random_choice<T>(
    seed: impl AsRef<str>,
    namespace: impl AsRef<str>,
    entity_id: impl AsRef<str>,
    field: impl AsRef<str>,
    values: &[T],
) -> Option<&T> {
    DeterministicRandom::new(
        seed.as_ref(),
        namespace.as_ref(),
        entity_id.as_ref(),
        field.as_ref(),
    )
    .choose(values)
}

fn deterministic_u64_at(
    seed: &str,
    namespace: &str,
    entity_id: &str,
    field: &str,
    counter: u64,
) -> u64 {
    let mut hash = FNV1A64_OFFSET_BASIS;

    hash = fnv1a64_component(hash, DETERMINISTIC_RNG_ALGORITHM.as_bytes());
    hash = fnv1a64_component(hash, seed.as_bytes());
    hash = fnv1a64_component(hash, namespace.as_bytes());
    hash = fnv1a64_component(hash, entity_id.as_bytes());
    hash = fnv1a64_component(hash, field.as_bytes());
    hash = fnv1a64_component(hash, &counter.to_le_bytes());

    splitmix64_finalize(hash)
}

fn fnv1a64_component(hash: u64, bytes: &[u8]) -> u64 {
    let hash = fnv1a64_bytes(hash, &(bytes.len() as u64).to_le_bytes());

    fnv1a64_bytes(hash, bytes)
}

fn fnv1a64_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV1A64_PRIME);
    }

    hash
}

fn splitmix64_finalize(mut value: u64) -> u64 {
    value = value.wrapping_add(SPLITMIX64_GAMMA);
    value = (value ^ (value >> 30)).wrapping_mul(SPLITMIX64_MIX_1);
    value = (value ^ (value >> 27)).wrapping_mul(SPLITMIX64_MIX_2);

    value ^ (value >> 31)
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
        CoreCapabilities::canonical_data_model()
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

    const GOLDEN_RANDOM_U64_VECTORS: [(&str, &str, &str, &str, u64); 6] = [
        (
            "seed-a",
            "users",
            "user-42",
            "age",
            2_209_488_964_295_464_862,
        ),
        (
            "seed-a",
            "users",
            "user-42",
            "display_name",
            4_459_706_444_465_804_371,
        ),
        (
            "seed-a",
            "posts",
            "post-1",
            "title",
            9_773_582_861_819_595_104,
        ),
        (
            "release-2026-05",
            "users",
            "user-000001",
            "locale",
            17_647_173_114_432_058_453,
        ),
        (
            "release-2026-05",
            "posts",
            "post-000001",
            "toxicity_score",
            6_776_290_718_759_254_825,
        ),
        (
            "tenant:alpha",
            "relationships",
            "user-42->user-99",
            "strength",
            3_627_064_159_707_263_269,
        ),
    ];

    #[test]
    fn exposes_product_identity() {
        assert_eq!(PRODUCT_NAME, "synthetic-pop");
        assert!(PROJECT_PROMISE.contains("offline"));
    }

    #[test]
    fn current_status_reports_canonical_data_model() {
        assert_eq!(current_status(), MilestoneStatus::CanonicalDataModel);
        assert_eq!(current_status().label(), "milestone 4 canonical data model");
    }

    #[test]
    fn exposes_reproducibility_metadata() {
        let metadata = reproducibility_metadata();

        assert_eq!(metadata.metadata_version, REPRODUCIBILITY_METADATA_VERSION);
        assert_eq!(
            metadata.deterministic_rng_algorithm,
            DETERMINISTIC_RNG_ALGORITHM
        );
        assert_eq!(
            metadata.deterministic_rng_algorithm_version,
            DETERMINISTIC_RNG_ALGORITHM_VERSION
        );
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
    fn capability_constructors_report_milestone_boundaries_truthfully() {
        let foundation = CoreCapabilities::rust_core_foundation();

        assert!(foundation.offline_core_available);
        assert!(foundation.local_core_available);
        assert!(!foundation.deterministic_rng_available);
        assert!(!foundation.canonical_data_model_available);
        assert!(!foundation.generation_available);
        assert!(!foundation.policy_available);
        assert!(!foundation.export_available);
        assert!(!foundation.bindings_available);
        assert!(!foundation.desktop_available);
        assert!(!foundation.wasm_available);

        let deterministic = CoreCapabilities::deterministic_generation_system();

        assert!(deterministic.offline_core_available);
        assert!(deterministic.local_core_available);
        assert!(deterministic.deterministic_rng_available);
        assert!(!deterministic.canonical_data_model_available);
        assert!(!deterministic.generation_available);
        assert!(!deterministic.policy_available);
        assert!(!deterministic.export_available);
        assert!(!deterministic.bindings_available);
        assert!(!deterministic.desktop_available);
        assert!(!deterministic.wasm_available);
    }

    #[test]
    fn reports_current_capabilities_truthfully() {
        let capabilities = CoreEngine::default().capabilities();

        assert!(capabilities.offline_core_available);
        assert!(capabilities.local_core_available);
        assert!(capabilities.deterministic_rng_available);
        assert!(capabilities.canonical_data_model_available);
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
        assert_eq!(seed.as_ref(), "stable-seed-1");
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

    #[test]
    fn deterministic_random_matches_golden_u64_vectors() {
        for (seed, namespace, entity_id, field, expected) in GOLDEN_RANDOM_U64_VECTORS {
            assert_eq!(
                random_u64(seed, namespace, entity_id, field),
                expected,
                "golden vector changed for ({seed}, {namespace}, {entity_id}, {field})"
            );
        }
    }

    #[test]
    fn deterministic_random_repeats_for_same_inputs() {
        let first = random_u64("seed-a", "users", "user-42", "age");
        let second = random_u64("seed-a", "users", "user-42", "age");

        assert_eq!(first, second);
    }

    #[test]
    fn deterministic_random_separates_input_components() {
        let baseline = random_u64("seed-a", "users", "user-42", "age");

        assert_ne!(baseline, random_u64("seed-b", "users", "user-42", "age"));
        assert_ne!(baseline, random_u64("seed-a", "posts", "user-42", "age"));
        assert_ne!(baseline, random_u64("seed-a", "users", "user-43", "age"));
        assert_ne!(
            baseline,
            random_u64("seed-a", "users", "user-42", "display_name")
        );
    }

    #[test]
    fn deterministic_random_is_independent_of_call_order() {
        let expected: Vec<u64> = GOLDEN_RANDOM_U64_VECTORS
            .iter()
            .map(|&(seed, namespace, entity_id, field, _)| {
                random_u64(seed, namespace, entity_id, field)
            })
            .collect();

        let mut later: Vec<(usize, u64)> = GOLDEN_RANDOM_U64_VECTORS
            .iter()
            .enumerate()
            .rev()
            .map(|(index, &(seed, namespace, entity_id, field, _))| {
                (index, random_u64(seed, namespace, entity_id, field))
            })
            .collect();
        later.sort_by_key(|(index, _)| *index);

        assert_eq!(
            later
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn deterministic_random_supports_string_like_seed_inputs() {
        let seed = Seed::new("seed-a").expect("seed should be valid");
        let owned_namespace = String::from("users");

        assert_eq!(
            random_u64(&seed, &owned_namespace, "user-42", "age"),
            random_u64("seed-a", "users", "user-42", "age")
        );
    }

    #[test]
    fn deterministic_random_bounds_values() {
        assert_eq!(
            random_bounded_u64("seed-a", "users", "user-42", "age", 0),
            None
        );
        assert_eq!(
            random_bounded_u64("seed-a", "users", "user-42", "age", 1),
            Some(0)
        );

        let value = random_bounded_u64("seed-a", "users", "user-42", "age", 37)
            .expect("upper bound should produce a value");

        assert_eq!(value, 4);
        assert!(value < 37);
        assert_eq!(
            value,
            random_bounded_u64("seed-a", "users", "user-42", "age", 37)
                .expect("upper bound should produce a value")
        );
    }

    #[test]
    fn deterministic_random_produces_stable_booleans() {
        let first = random_bool("seed-a", "users", "user-42", "is_active");
        let second = random_bool("seed-a", "users", "user-42", "is_active");

        assert!(!first);
        assert_eq!(first, second);
    }

    #[test]
    fn deterministic_random_selects_from_slices() {
        let values = ["low", "medium", "high"];
        let index =
            random_index("seed-a", "users", "user-42", "risk", &values).expect("index exists");
        let choice =
            random_choice("seed-a", "users", "user-42", "risk", &values).expect("choice exists");

        assert_eq!(index, 1);
        assert!(index < values.len());
        assert_eq!(choice, &values[index]);
        let empty: [&str; 0] = [];

        assert_eq!(
            random_index("seed-a", "users", "user-42", "risk", &empty),
            None
        );
    }

    #[test]
    fn deterministic_random_key_exposes_inputs_and_helpers() {
        let key = DeterministicRandom::new("seed-a", "users", "user-42", "age");
        let values = [10, 20, 30, 40];

        assert_eq!(key.seed(), "seed-a");
        assert_eq!(key.namespace(), "users");
        assert_eq!(key.entity_id(), "user-42");
        assert_eq!(key.field(), "age");
        assert_eq!(key.u64(), random_u64("seed-a", "users", "user-42", "age"));
        assert!(key.bounded_u64(10).expect("value exists") < 10);
        assert!(key.choose_index(&values).expect("index exists") < values.len());
        assert!(values.contains(key.choose(&values).expect("choice exists")));
    }

    #[test]
    fn deterministic_random_is_safe_to_compute_in_parallel() {
        let expected: Vec<u64> = GOLDEN_RANDOM_U64_VECTORS
            .iter()
            .map(|&(seed, namespace, entity_id, field, _)| {
                random_u64(seed, namespace, entity_id, field)
            })
            .collect();

        let handles: Vec<_> = GOLDEN_RANDOM_U64_VECTORS
            .iter()
            .enumerate()
            .rev()
            .map(|(index, &(seed, namespace, entity_id, field, _))| {
                std::thread::spawn(move || (index, random_u64(seed, namespace, entity_id, field)))
            })
            .collect();

        let mut actual = vec![0; GOLDEN_RANDOM_U64_VECTORS.len()];
        for handle in handles {
            let (index, value) = handle.join().expect("deterministic RNG thread panicked");
            actual[index] = value;
        }

        assert_eq!(actual, expected);
    }
}
