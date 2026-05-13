use serde::{Deserialize, Serialize};

pub const FIRST_SCENARIO: &str = "forum";
pub const MAX_USERS: usize = 100_000;
pub const MAX_COMMUNITIES: usize = 100_000;
pub const MAX_CONTENT_ITEMS: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioState {
    Planned,
}

#[must_use]
pub const fn first_scenario_state() -> ScenarioState {
    ScenarioState::Planned
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scenario", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScenarioConfig {
    Forum(ForumScenarioConfig),
}

impl ScenarioConfig {
    pub fn validate(&self) -> Result<(), ScenarioValidationError> {
        match self {
            Self::Forum(config) => config.validate(),
        }
    }

    #[must_use]
    pub const fn scenario_name(&self) -> &'static str {
        match self {
            Self::Forum(_) => FIRST_SCENARIO,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForumScenarioConfig {
    pub seed: String,
    pub population: PopulationConfig,
    pub communities: Vec<String>,
    pub content: ContentConfig,
    #[serde(default)]
    pub output_format: OutputFormat,
}

impl ForumScenarioConfig {
    pub fn validate(&self) -> Result<(), ScenarioValidationError> {
        validate_seed(&self.seed)?;
        self.population.validate()?;
        validate_communities(&self.communities)?;
        self.content.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationConfig {
    #[serde(alias = "user_count")]
    pub users: usize,
}

impl PopulationConfig {
    pub fn validate(&self) -> Result<(), ScenarioValidationError> {
        validate_count("users", self.users, MAX_USERS)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentConfig {
    #[serde(alias = "post_count")]
    pub posts: usize,
    #[serde(alias = "comment_count")]
    pub comments: usize,
}

impl ContentConfig {
    pub fn validate(&self) -> Result<(), ScenarioValidationError> {
        validate_count("posts", self.posts, MAX_CONTENT_ITEMS)?;
        validate_count("comments", self.comments, MAX_CONTENT_ITEMS)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Json,
    Jsonl,
    Csv,
    SqliteSql,
    PostgresSql,
    PrismaSeed,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Jsonl
    }
}

pub fn parse_forum_config_yaml(input: &str) -> Result<ScenarioConfig, ScenarioConfigError> {
    let config: ScenarioConfig = serde_yaml::from_str(input).map_err(ScenarioConfigError::Yaml)?;
    config.validate().map_err(ScenarioConfigError::Validation)?;

    Ok(config)
}

#[derive(Debug)]
pub enum ScenarioConfigError {
    Yaml(serde_yaml::Error),
    Validation(ScenarioValidationError),
}

impl std::fmt::Display for ScenarioConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Yaml(error) => write!(formatter, "failed to parse scenario YAML: {error}"),
            Self::Validation(error) => write!(formatter, "invalid scenario config: {error}"),
        }
    }
}

impl std::error::Error for ScenarioConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Yaml(error) => Some(error),
            Self::Validation(error) => Some(error),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioValidationError {
    EmptySeed,
    ZeroCount {
        field: &'static str,
    },
    CountTooLarge {
        field: &'static str,
        requested: usize,
        max: usize,
    },
    EmptyCommunities,
    EmptyCommunityName,
}

impl std::fmt::Display for ScenarioValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySeed => formatter.write_str("seed must not be empty"),
            Self::ZeroCount { field } => write!(formatter, "{field} must be greater than zero"),
            Self::CountTooLarge {
                field,
                requested,
                max,
            } => write!(
                formatter,
                "{field} count {requested} exceeds maximum of {max}"
            ),
            Self::EmptyCommunities => formatter.write_str("communities must not be empty"),
            Self::EmptyCommunityName => formatter.write_str("community names must not be empty"),
        }
    }
}

impl std::error::Error for ScenarioValidationError {}

fn validate_seed(seed: &str) -> Result<(), ScenarioValidationError> {
    if seed.trim().is_empty() {
        return Err(ScenarioValidationError::EmptySeed);
    }

    Ok(())
}

fn validate_count(
    field: &'static str,
    value: usize,
    max: usize,
) -> Result<(), ScenarioValidationError> {
    if value == 0 {
        return Err(ScenarioValidationError::ZeroCount { field });
    }

    if value > max {
        return Err(ScenarioValidationError::CountTooLarge {
            field,
            requested: value,
            max,
        });
    }

    Ok(())
}

fn validate_communities(communities: &[String]) -> Result<(), ScenarioValidationError> {
    if communities.is_empty() {
        return Err(ScenarioValidationError::EmptyCommunities);
    }

    if communities.len() > MAX_COMMUNITIES {
        return Err(ScenarioValidationError::CountTooLarge {
            field: "communities",
            requested: communities.len(),
            max: MAX_COMMUNITIES,
        });
    }

    if communities
        .iter()
        .any(|community| community.trim().is_empty())
    {
        return Err(ScenarioValidationError::EmptyCommunityName);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_FORUM_CONFIG: &str = r#"
scenario: forum
seed: milestone-5
population:
  users: 100
communities:
  - general
  - support
content:
  posts: 500
  comments: 300000
output_format: jsonl
"#;

    #[test]
    fn first_scenario_matches_the_initial_wedge() {
        assert_eq!(FIRST_SCENARIO, "forum");
        assert_eq!(first_scenario_state(), ScenarioState::Planned);
    }

    #[test]
    fn parses_and_validates_forum_config_yaml() {
        let config = parse_forum_config_yaml(VALID_FORUM_CONFIG).expect("config should parse");

        assert_eq!(config.scenario_name(), "forum");

        let ScenarioConfig::Forum(config) = config;
        assert_eq!(config.seed, "milestone-5");
        assert_eq!(config.population.users, 100);
        assert_eq!(config.communities, ["general", "support"]);
        assert_eq!(config.content.posts, 500);
        assert_eq!(config.content.comments, 300000);
        assert_eq!(config.output_format, OutputFormat::Jsonl);
    }

    #[test]
    fn defaults_output_format_to_jsonl() {
        let input = r#"
scenario: forum
seed: demo
population:
  users: 1
communities:
  - general
content:
  posts: 1
  comments: 1
"#;

        let ScenarioConfig::Forum(config) =
            parse_forum_config_yaml(input).expect("config should parse");

        assert_eq!(config.output_format, OutputFormat::Jsonl);
    }

    #[test]
    fn rejects_empty_seed() {
        let error = parse_forum_config_yaml(&VALID_FORUM_CONFIG.replace("milestone-5", "\"   \""))
            .expect_err("empty seed should fail");

        assert_validation_error(error, ScenarioValidationError::EmptySeed);
    }

    #[test]
    fn rejects_unknown_scenario() {
        let error = parse_forum_config_yaml(&VALID_FORUM_CONFIG.replace("forum", "marketplace"))
            .expect_err("unknown scenario should fail");

        let message = error.to_string();
        assert!(message.contains("failed to parse scenario YAML"));
        assert!(message.contains("marketplace"));
    }

    #[test]
    fn rejects_empty_communities() {
        let input = r#"
scenario: forum
seed: demo
population:
  users: 1
communities: []
content:
  posts: 1
  comments: 1
"#;

        let error = parse_forum_config_yaml(input).expect_err("empty communities should fail");

        assert_validation_error(error, ScenarioValidationError::EmptyCommunities);
    }

    #[test]
    fn rejects_zero_users() {
        let error = parse_forum_config_yaml(&VALID_FORUM_CONFIG.replace("users: 100", "users: 0"))
            .expect_err("zero users should fail");

        assert_validation_error(error, ScenarioValidationError::ZeroCount { field: "users" });
    }

    #[test]
    fn rejects_oversized_counts() {
        let error =
            parse_forum_config_yaml(&VALID_FORUM_CONFIG.replace("posts: 500", "posts: 1000001"))
                .expect_err("oversized posts should fail");

        assert_validation_error(
            error,
            ScenarioValidationError::CountTooLarge {
                field: "posts",
                requested: MAX_CONTENT_ITEMS + 1,
                max: MAX_CONTENT_ITEMS,
            },
        );
    }

    #[test]
    fn accepts_count_aliases() {
        let input = r#"
scenario: forum
seed: demo
population:
  user_count: 10
communities:
  - general
content:
  post_count: 20
  comment_count: 30
output_format: postgres-sql
"#;

        let ScenarioConfig::Forum(config) =
            parse_forum_config_yaml(input).expect("aliased config should parse");

        assert_eq!(config.population.users, 10);
        assert_eq!(config.content.posts, 20);
        assert_eq!(config.content.comments, 30);
        assert_eq!(config.output_format, OutputFormat::PostgresSql);
    }

    fn assert_validation_error(error: ScenarioConfigError, expected: ScenarioValidationError) {
        match error {
            ScenarioConfigError::Validation(actual) => assert_eq!(actual, expected),
            ScenarioConfigError::Yaml(error) => panic!("expected validation error, got {error}"),
        }
    }
}
