use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use synthetic_pop_core::{
    random_bounded_u64, ActivityEvent, ActivityEventId, ActivityEventKind, ActivityObject, Comment,
    CommentId, Community, CommunityId, Locale, ModelTimestamp, ModelValidationError, Post, PostId,
    Relationship, RelationshipEndpoint, RelationshipId, RelationshipKind, User, UserId,
};

pub const FIRST_SCENARIO: &str = "forum";
pub const MAX_USERS: usize = 100_000;
pub const MAX_COMMUNITIES: usize = 100_000;
pub const MAX_CONTENT_ITEMS: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioState {
    Planned,
    Implemented,
}

#[must_use]
pub const fn first_scenario_state() -> ScenarioState {
    ScenarioState::Implemented
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Json,
    #[default]
    Jsonl,
    Csv,
    SqliteSql,
    PostgresSql,
    PrismaSeed,
}

pub fn parse_forum_config_yaml(input: &str) -> Result<ScenarioConfig, ScenarioConfigError> {
    let config: ScenarioConfig = serde_yaml::from_str(input).map_err(ScenarioConfigError::Yaml)?;
    config.validate().map_err(ScenarioConfigError::Validation)?;

    Ok(config)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForumDataset {
    pub users: Vec<User>,
    pub communities: Vec<Community>,
    pub posts: Vec<Post>,
    pub comments: Vec<Comment>,
    pub relationships: Vec<Relationship>,
    pub activity_events: Vec<ActivityEvent>,
}

pub fn generate_forum_dataset(
    config: &ForumScenarioConfig,
) -> Result<ForumDataset, ForumGenerationError> {
    config.validate()?;

    let mut activity_events = Vec::new();
    let communities = generate_communities(config, &mut activity_events)?;
    let mut users = generate_users(config, &communities, &mut activity_events)?;
    let relationships =
        generate_memberships(config, &mut users, &communities, &mut activity_events)?;
    let community_members = community_member_indexes(&users, &communities);
    let community_indexes = community_indexes(&communities);
    let posts = generate_posts(
        config,
        &users,
        &communities,
        &community_members,
        &mut activity_events,
    )?;
    let comments = generate_comments(
        config,
        &users,
        &posts,
        &community_indexes,
        &community_members,
        &mut activity_events,
    )?;

    Ok(ForumDataset {
        users,
        communities,
        posts,
        comments,
        relationships,
        activity_events,
    })
}

#[derive(Debug)]
pub enum ForumGenerationError {
    Validation(ScenarioValidationError),
    Model(ModelValidationError),
}

impl std::fmt::Display for ForumGenerationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(error) => write!(formatter, "invalid forum scenario config: {error}"),
            Self::Model(error) => write!(formatter, "failed to build forum model record: {error}"),
        }
    }
}

impl std::error::Error for ForumGenerationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            Self::Model(error) => Some(error),
        }
    }
}

impl From<ScenarioValidationError> for ForumGenerationError {
    fn from(error: ScenarioValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<ModelValidationError> for ForumGenerationError {
    fn from(error: ModelValidationError) -> Self {
        Self::Model(error)
    }
}

const LOCALES: [&str; 5] = ["en-US", "en-GB", "en-CA", "en-AU", "en-NZ"];
const FIRST_NAMES: [&str; 12] = [
    "Alex", "Blair", "Casey", "Devon", "Emery", "Finley", "Harper", "Jordan", "Kai", "Morgan",
    "Quinn", "Riley",
];
const LAST_NAMES: [&str; 12] = [
    "Adams", "Brooks", "Chen", "Diaz", "Ellis", "Foster", "Gray", "Hayes", "Ibrahim", "Jones",
    "Kim", "Lopez",
];
const POST_TOPICS: [&str; 8] = [
    "launch notes",
    "daily workflow",
    "resource list",
    "bug triage",
    "community norms",
    "tooling",
    "roadmap",
    "retrospective",
];
const COMMENT_TONES: [&str; 8] = [
    "This matches what I have seen as well.",
    "Could you share one more concrete example?",
    "The second option seems easier to maintain.",
    "I would document the tradeoff before changing it.",
    "That should work for smaller teams first.",
    "The edge case is worth testing before rollout.",
    "Thanks for writing up the context.",
    "I tried a similar approach last week.",
];

fn generate_communities(
    config: &ForumScenarioConfig,
    activity_events: &mut Vec<ActivityEvent>,
) -> Result<Vec<Community>, ForumGenerationError> {
    config
        .communities
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let id = community_id(index, name)?;
            let timestamp = timestamp_for("community", index)?;
            let mut community = Community::new(id.clone(), name.trim(), timestamp.clone())?;
            community.description = Some(format!("A forum space for {} discussions.", name.trim()));
            community.owner_id = Some(user_id(index % config.population.users)?);
            push_activity(
                activity_events,
                ActivityEventKind::CommunityJoined,
                None,
                ActivityObject::Community(id),
                timestamp,
            )?;
            Ok(community)
        })
        .collect()
}

fn generate_users(
    config: &ForumScenarioConfig,
    communities: &[Community],
    activity_events: &mut Vec<ActivityEvent>,
) -> Result<Vec<User>, ForumGenerationError> {
    (0..config.population.users)
        .map(|index| {
            let id = user_id(index)?;
            let entity_id = id.as_str();
            let first = choose(config, "users", entity_id, "first_name", &FIRST_NAMES);
            let last = choose(config, "users", entity_id, "last_name", &LAST_NAMES);
            let locale = choose(config, "users", entity_id, "locale", &LOCALES);
            let timestamp = timestamp_for("user", index)?;
            let mut user = User::new(
                id.clone(),
                format!(
                    "{}.{}{:04}",
                    first.to_lowercase(),
                    last.to_lowercase(),
                    index + 1
                ),
                format!("{first} {last}"),
                Locale::new(*locale)?,
                timestamp.clone(),
            )?;
            user.bio = Some(format!(
                "{} follows {} and practical forum discussions.",
                first,
                communities[index % communities.len()].name
            ));
            user.status = Some("active".to_string());
            push_activity(
                activity_events,
                ActivityEventKind::UserCreated,
                Some(id.clone()),
                ActivityObject::User(id),
                timestamp,
            )?;
            Ok(user)
        })
        .collect()
}

fn generate_memberships(
    config: &ForumScenarioConfig,
    users: &mut [User],
    communities: &[Community],
    activity_events: &mut Vec<ActivityEvent>,
) -> Result<Vec<Relationship>, ForumGenerationError> {
    let mut relationships = Vec::new();

    for (user_index, user) in users.iter_mut().enumerate() {
        let primary = deterministic_index(
            config,
            "memberships",
            user.id.as_str(),
            "primary_community",
            communities.len(),
        );
        push_membership(
            &mut relationships,
            activity_events,
            user,
            &communities[primary],
            user_index,
            primary,
        )?;

        if communities.len() > 1
            && deterministic_index(config, "memberships", user.id.as_str(), "secondary_gate", 3)
                == 0
        {
            let offset = deterministic_index(
                config,
                "memberships",
                user.id.as_str(),
                "secondary_community",
                communities.len() - 1,
            ) + 1;
            let secondary = (primary + offset) % communities.len();
            push_membership(
                &mut relationships,
                activity_events,
                user,
                &communities[secondary],
                user_index,
                secondary,
            )?;
        }
    }

    for (community_index, community) in communities.iter().enumerate() {
        let has_member = users
            .iter()
            .any(|user| user.community_ids.contains(&community.id));

        if !has_member {
            let user_index = deterministic_index(
                config,
                "memberships",
                community.id.as_str(),
                "coverage_user",
                users.len(),
            );
            push_membership(
                &mut relationships,
                activity_events,
                &mut users[user_index],
                community,
                user_index,
                community_index,
            )?;
        }
    }

    Ok(relationships)
}

fn push_membership(
    relationships: &mut Vec<Relationship>,
    activity_events: &mut Vec<ActivityEvent>,
    user: &mut User,
    community: &Community,
    user_index: usize,
    community_index: usize,
) -> Result<(), ForumGenerationError> {
    let relationship_index = relationships.len();
    let relationship_id =
        RelationshipId::new(format!("relationship-{:06}", relationship_index + 1))?;
    let timestamp = timestamp_for("membership", user_index + community_index)?;

    user.community_ids.push(community.id.clone());
    relationships.push(Relationship {
        id: relationship_id.clone(),
        source: RelationshipEndpoint::User(user.id.clone()),
        target: RelationshipEndpoint::Community(community.id.clone()),
        kind: RelationshipKind::MemberOf,
        created_at: timestamp.clone(),
    });
    push_activity(
        activity_events,
        ActivityEventKind::RelationshipCreated,
        Some(user.id.clone()),
        ActivityObject::Relationship(relationship_id),
        timestamp,
    )
}

fn generate_posts(
    config: &ForumScenarioConfig,
    users: &[User],
    communities: &[Community],
    community_members: &[Vec<usize>],
    activity_events: &mut Vec<ActivityEvent>,
) -> Result<Vec<Post>, ForumGenerationError> {
    (0..config.content.posts)
        .map(|index| {
            let id = post_id(index)?;
            let community_index =
                deterministic_index(config, "posts", id.as_str(), "community", communities.len());
            let community = &communities[community_index];
            let author = choose_member(
                config,
                users,
                &community_members[community_index],
                id.as_str(),
            );
            let topic = choose(config, "posts", id.as_str(), "topic", &POST_TOPICS);
            let timestamp = timestamp_for("post", index)?;
            let mut post = Post::new(
                id.clone(),
                author.id.clone(),
                format!(
                    "{} started a thread about {} in {}.",
                    author.display_name, topic, community.name
                ),
                timestamp.clone(),
            )?;
            post.community_id = Some(community.id.clone());
            post.title = Some(format!("{}: {}", community.name, title_case(topic)));
            push_activity(
                activity_events,
                ActivityEventKind::PostCreated,
                Some(author.id.clone()),
                ActivityObject::Post(id),
                timestamp,
            )?;
            Ok(post)
        })
        .collect()
}

fn generate_comments(
    config: &ForumScenarioConfig,
    users: &[User],
    posts: &[Post],
    community_indexes: &HashMap<CommunityId, usize>,
    community_members: &[Vec<usize>],
    activity_events: &mut Vec<ActivityEvent>,
) -> Result<Vec<Comment>, ForumGenerationError> {
    (0..config.content.comments)
        .map(|index| {
            let id = comment_id(index)?;
            let post_index =
                deterministic_index(config, "comments", id.as_str(), "post", posts.len());
            let post = &posts[post_index];
            let community_id = post
                .community_id
                .as_ref()
                .expect("generated posts always have a community");
            let community_index = community_indexes
                .get(community_id)
                .expect("generated post community should exist");
            let author = choose_comment_author(
                config,
                users,
                &community_members[*community_index],
                post.author_id.as_str(),
                id.as_str(),
            );
            let body = choose(config, "comments", id.as_str(), "body", &COMMENT_TONES);
            let timestamp = timestamp_for("comment", index)?;
            let comment = Comment::new(
                id.clone(),
                post.id.clone(),
                author.id.clone(),
                *body,
                timestamp.clone(),
            )?;
            push_activity(
                activity_events,
                ActivityEventKind::CommentCreated,
                Some(author.id.clone()),
                ActivityObject::Comment(id),
                timestamp,
            )?;
            Ok(comment)
        })
        .collect()
}

fn choose_member<'a>(
    config: &ForumScenarioConfig,
    users: &'a [User],
    members: &[usize],
    entity_id: &str,
) -> &'a User {
    let index = deterministic_index(config, "posts", entity_id, "author", members.len());

    &users[members[index]]
}

fn choose_comment_author<'a>(
    config: &ForumScenarioConfig,
    users: &'a [User],
    members: &[usize],
    post_author_id: &str,
    entity_id: &str,
) -> &'a User {
    let offset = deterministic_index(config, "comments", entity_id, "author", members.len());
    let author = &users[members[offset]];

    if members.len() == 1 || author.id.as_str() != post_author_id {
        return author;
    }

    &users[members[(offset + 1) % members.len()]]
}

fn community_member_indexes(users: &[User], communities: &[Community]) -> Vec<Vec<usize>> {
    communities
        .iter()
        .map(|community| {
            users
                .iter()
                .enumerate()
                .filter_map(|(index, user)| {
                    user.community_ids.contains(&community.id).then_some(index)
                })
                .collect()
        })
        .collect()
}

fn community_indexes(communities: &[Community]) -> HashMap<CommunityId, usize> {
    communities
        .iter()
        .enumerate()
        .map(|(index, community)| (community.id.clone(), index))
        .collect()
}

fn push_activity(
    activity_events: &mut Vec<ActivityEvent>,
    kind: ActivityEventKind,
    actor_id: Option<UserId>,
    object: ActivityObject,
    occurred_at: ModelTimestamp,
) -> Result<(), ForumGenerationError> {
    let id = ActivityEventId::new(format!("activity-{:06}", activity_events.len() + 1))?;
    activity_events.push(ActivityEvent {
        id,
        kind,
        actor_id,
        object,
        occurred_at,
    });

    Ok(())
}

fn user_id(index: usize) -> Result<UserId, ModelValidationError> {
    UserId::new(format!("user-{:06}", index + 1))
}

fn post_id(index: usize) -> Result<PostId, ModelValidationError> {
    PostId::new(format!("post-{:06}", index + 1))
}

fn comment_id(index: usize) -> Result<CommentId, ModelValidationError> {
    CommentId::new(format!("comment-{:06}", index + 1))
}

fn community_id(index: usize, name: &str) -> Result<CommunityId, ModelValidationError> {
    CommunityId::new(format!("community-{:06}-{}", index + 1, slug(name)))
}

fn deterministic_index(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    field: &str,
    len: usize,
) -> usize {
    let upper_bound = u64::try_from(len).expect("scenario count should fit in u64");
    let index = random_bounded_u64(&config.seed, namespace, entity_id, field, upper_bound)
        .expect("validated scenario counts must be non-zero");

    usize::try_from(index).expect("bounded random index should fit in usize")
}

fn choose<'a, T>(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    field: &str,
    values: &'a [T],
) -> &'a T {
    &values[deterministic_index(config, namespace, entity_id, field, values.len())]
}

fn timestamp_for(namespace: &str, index: usize) -> Result<ModelTimestamp, ModelValidationError> {
    let base_minutes = match namespace {
        "community" => 0,
        "user" => 10_000,
        "membership" => 20_000,
        "post" => 30_000,
        "comment" => 40_000,
        _ => 50_000,
    };

    ModelTimestamp::new(timestamp_from_minutes(base_minutes + index * 7))
}

fn timestamp_from_minutes(minutes: usize) -> String {
    let days = minutes / 1_440;
    let minute_of_day = minutes % 1_440;
    let hour = minute_of_day / 60;
    let minute = minute_of_day % 60;
    let (year, month, day) = ymd_after_2026_01_01(days);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:00Z")
}

fn ymd_after_2026_01_01(mut days: usize) -> (usize, usize, usize) {
    let mut year = 2026;

    loop {
        let year_days = if is_leap_year(year) { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }

    let month_lengths = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    for (month, month_days) in (1..).zip(month_lengths) {
        if days < month_days {
            return (year, month, days + 1);
        }
        days -= month_days;
    }

    unreachable!("day of year should map to a month")
}

fn is_leap_year(year: usize) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn slug(value: &str) -> String {
    let slug: String = value
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() {
        "community".to_string()
    } else {
        slug
    }
}

fn title_case(value: &str) -> String {
    let mut result = String::new();
    for word in value.split_whitespace() {
        if !result.is_empty() {
            result.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            result.push_str(chars.as_str());
        }
    }

    result
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
    use synthetic_pop_core::{ActivityObject, RelationshipEndpoint};

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
        assert_eq!(first_scenario_state(), ScenarioState::Implemented);
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

    #[test]
    fn generates_expected_forum_record_counts() {
        let dataset = generate_forum_dataset(&small_forum_config()).expect("dataset should build");

        assert_eq!(dataset.users.len(), 4);
        assert_eq!(dataset.communities.len(), 2);
        assert_eq!(dataset.posts.len(), 5);
        assert_eq!(dataset.comments.len(), 7);
        assert!(dataset.relationships.len() >= dataset.users.len());
        assert_eq!(
            dataset.activity_events.len(),
            dataset.communities.len()
                + dataset.users.len()
                + dataset.relationships.len()
                + dataset.posts.len()
                + dataset.comments.len()
        );
    }

    #[test]
    fn forum_generation_is_deterministic_for_same_config() {
        let config = small_forum_config();

        let first = generate_forum_dataset(&config).expect("first dataset should build");
        let second = generate_forum_dataset(&config).expect("second dataset should build");

        assert_eq!(first, second);
    }

    #[test]
    fn forum_generation_changes_with_seed() {
        let first_config = small_forum_config();
        let mut second_config = small_forum_config();
        second_config.seed = "milestone-5-other".to_string();

        let first = generate_forum_dataset(&first_config).expect("first dataset should build");
        let second = generate_forum_dataset(&second_config).expect("second dataset should build");

        assert_ne!(first.users[0].display_name, second.users[0].display_name);
    }

    #[test]
    fn generated_forum_records_have_representative_stable_values() {
        let dataset = generate_forum_dataset(&small_forum_config()).expect("dataset should build");

        assert_eq!(dataset.users[0].id.as_str(), "user-000001");
        assert_eq!(dataset.users[0].username, "harper.gray0001");
        assert_eq!(dataset.users[0].display_name, "Harper Gray");
        assert_eq!(dataset.users[0].created_at.as_str(), "2026-01-07T22:40:00Z");
        assert_eq!(
            dataset.communities[0].id.as_str(),
            "community-000001-general"
        );
        assert_eq!(
            dataset.communities[0].description.as_deref(),
            Some("A forum space for general discussions.")
        );
        assert_eq!(dataset.posts[0].id.as_str(), "post-000001");
        assert_eq!(dataset.posts[0].author_id.as_str(), "user-000001");
        assert_eq!(
            dataset.posts[0]
                .community_id
                .as_ref()
                .map(CommunityId::as_str),
            Some("community-000001-general")
        );
        assert_eq!(dataset.posts[0].title.as_deref(), Some("general: Roadmap"));
        assert_eq!(
            dataset.posts[0].body,
            "Harper Gray started a thread about roadmap in general."
        );
        assert_eq!(dataset.comments[0].id.as_str(), "comment-000001");
        assert_eq!(dataset.comments[0].post_id.as_str(), "post-000005");
        assert_eq!(
            dataset.comments[0].body,
            "I would document the tradeoff before changing it."
        );
    }

    #[test]
    fn validates_config_before_forum_generation() {
        let mut config = small_forum_config();
        config.seed = "  ".to_string();

        let error = generate_forum_dataset(&config).expect_err("invalid config should fail");

        match error {
            ForumGenerationError::Validation(actual) => {
                assert_eq!(actual, ScenarioValidationError::EmptySeed);
            }
            ForumGenerationError::Model(error) => panic!("expected validation error, got {error}"),
        }
    }

    #[test]
    fn generated_relationships_and_activity_events_are_consistent() {
        let dataset = generate_forum_dataset(&small_forum_config()).expect("dataset should build");

        for relationship in &dataset.relationships {
            let RelationshipEndpoint::User(user_id) = &relationship.source else {
                panic!("membership source should be a user");
            };
            let RelationshipEndpoint::Community(community_id) = &relationship.target else {
                panic!("membership target should be a community");
            };
            let user = dataset
                .users
                .iter()
                .find(|candidate| &candidate.id == user_id)
                .expect("relationship user should exist");

            assert!(user.community_ids.contains(community_id));
            assert!(dataset
                .communities
                .iter()
                .any(|community| &community.id == community_id));
            assert!(dataset.activity_events.iter().any(|event| {
                event.actor_id.as_ref() == Some(user_id)
                    && event.object == ActivityObject::Relationship(relationship.id.clone())
            }));
        }

        for post in &dataset.posts {
            let community_id = post
                .community_id
                .as_ref()
                .expect("generated post should have a community");
            let author = dataset
                .users
                .iter()
                .find(|user| user.id == post.author_id)
                .expect("post author should exist");

            assert!(author.community_ids.contains(community_id));
            assert!(dataset.activity_events.iter().any(|event| {
                event.actor_id.as_ref() == Some(&post.author_id)
                    && event.object == ActivityObject::Post(post.id.clone())
            }));
        }

        for comment in &dataset.comments {
            assert!(dataset.posts.iter().any(|post| post.id == comment.post_id));
            assert!(dataset
                .users
                .iter()
                .any(|user| user.id == comment.author_id));
            assert!(dataset.activity_events.iter().any(|event| {
                event.actor_id.as_ref() == Some(&comment.author_id)
                    && event.object == ActivityObject::Comment(comment.id.clone())
            }));
        }
    }

    #[test]
    fn generation_keeps_every_community_usable_for_post_authors() {
        let config = ForumScenarioConfig {
            seed: "sparse-membership".to_string(),
            population: PopulationConfig { users: 1 },
            communities: vec![
                "general".to_string(),
                "support".to_string(),
                "announcements".to_string(),
            ],
            content: ContentConfig {
                posts: 12,
                comments: 12,
            },
            output_format: OutputFormat::Jsonl,
        };

        let dataset = generate_forum_dataset(&config).expect("dataset should build");

        for community in &dataset.communities {
            assert!(
                dataset
                    .users
                    .iter()
                    .any(|user| user.community_ids.contains(&community.id)),
                "{} should have at least one member",
                community.id
            );
        }
        for post in &dataset.posts {
            let author = dataset
                .users
                .iter()
                .find(|user| user.id == post.author_id)
                .expect("post author should exist");
            assert!(author.community_ids.contains(
                post.community_id
                    .as_ref()
                    .expect("generated post should have a community")
            ));
        }
    }

    fn small_forum_config() -> ForumScenarioConfig {
        ForumScenarioConfig {
            seed: "milestone-5".to_string(),
            population: PopulationConfig { users: 4 },
            communities: vec!["general".to_string(), "support".to_string()],
            content: ContentConfig {
                posts: 5,
                comments: 7,
            },
            output_format: OutputFormat::Jsonl,
        }
    }
}
