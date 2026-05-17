use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use synthetic_pop_core::{
    random_bounded_u64, ActivityEvent, ActivityEventId, ActivityEventKind, ActivityObject,
    ActivityPattern, Comment, CommentId, Community, CommunityId, Interest, InterestId, Locale,
    ModelTimestamp, ModelValidationError, Persona, PersonaId, Post, PostId, Relationship,
    RelationshipEndpoint, RelationshipId, RelationshipKind, SleepPhase, User, UserId, Verbosity,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForumDataset {
    pub users: Vec<User>,
    pub personas: Vec<Persona>,
    pub interests: Vec<Interest>,
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
    let mut users = generate_users(config, &mut activity_events)?;
    let personas = generate_personas(config, &mut users, &mut activity_events)?;
    let interests = generate_interests()?;
    assign_user_interests(config, &mut users, &personas, &interests);
    assign_user_bios(config, &mut users, &personas, &interests);
    let mut relationships = generate_memberships(
        config,
        &mut users,
        &personas,
        &communities,
        &mut activity_events,
    )?;
    relationships.extend(generate_social_relationships(
        config,
        &users,
        &personas,
        relationships.len(),
        &mut activity_events,
    )?);
    let community_members = community_member_indexes(&users, &communities);
    let community_indexes = community_indexes(&communities);
    let posts = generate_posts(
        config,
        &users,
        &personas,
        &communities,
        &community_members,
        &mut activity_events,
    )?;
    let comments = generate_comments(
        config,
        &users,
        &personas,
        &posts,
        &community_indexes,
        &community_members,
        &mut activity_events,
    )?;
    sort_activity_events(&mut activity_events);

    Ok(ForumDataset {
        users,
        personas,
        interests,
        communities,
        posts,
        comments,
        relationships,
        activity_events,
    })
}

pub fn validate_forum_temporal_consistency(
    dataset: &ForumDataset,
) -> Result<(), TemporalConsistencyError> {
    let mut errors = Vec::new();
    let user_created_at = dataset
        .users
        .iter()
        .map(|user| (user.id.clone(), minutes_from_timestamp(&user.created_at)))
        .collect::<HashMap<_, _>>();
    let post_by_id = dataset
        .posts
        .iter()
        .map(|post| (post.id.clone(), post))
        .collect::<HashMap<_, _>>();
    let comment_by_id = dataset
        .comments
        .iter()
        .map(|comment| (comment.id.clone(), comment))
        .collect::<HashMap<_, _>>();
    let relationship_by_id = dataset
        .relationships
        .iter()
        .map(|relationship| (relationship.id.clone(), relationship))
        .collect::<HashMap<_, _>>();

    validate_stable_timestamp_values(dataset, &mut errors);
    validate_posts_after_authors(dataset, &user_created_at, &mut errors);
    validate_comments_after_posts(dataset, &user_created_at, &post_by_id, &mut errors);
    validate_relationships_after_users(dataset, &user_created_at, &mut errors);
    validate_activity_events(
        dataset,
        &user_created_at,
        &post_by_id,
        &comment_by_id,
        &relationship_by_id,
        &mut errors,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(TemporalConsistencyError {
            message: errors.join("; "),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalConsistencyError {
    message: String,
}

impl TemporalConsistencyError {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for TemporalConsistencyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TemporalConsistencyError {}

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
const VERBOSITIES: [Verbosity; 3] = [Verbosity::Terse, Verbosity::Balanced, Verbosity::Detailed];
const SLEEP_PHASES: [SleepPhase; 4] = [
    SleepPhase::EarlyBird,
    SleepPhase::Daytime,
    SleepPhase::NightOwl,
    SleepPhase::Irregular,
];
const INTEREST_CATALOG: [InterestSpec; 15] = [
    InterestSpec::new(
        "interest-programming",
        "Programming",
        "technical",
        1.00,
        0.10,
        0.00,
    ),
    InterestSpec::new("interest-linux", "Linux", "technical", 0.95, 0.00, 0.00),
    InterestSpec::new("interest-systems", "Systems", "technical", 0.90, 0.00, 0.00),
    InterestSpec::new("interest-tooling", "Tooling", "technical", 0.85, 0.05, 0.00),
    InterestSpec::new(
        "interest-community",
        "Community",
        "social",
        0.00,
        1.00,
        0.10,
    ),
    InterestSpec::new("interest-events", "Events", "social", 0.00, 0.90, 0.05),
    InterestSpec::new(
        "interest-collaboration",
        "Collaboration",
        "social",
        0.15,
        0.85,
        0.00,
    ),
    InterestSpec::new("interest-gaming", "Gaming", "culture", 0.05, 0.20, 0.95),
    InterestSpec::new("interest-memes", "Memes", "culture", 0.00, 0.10, 1.00),
    InterestSpec::new("interest-culture", "Culture", "culture", 0.00, 0.25, 0.80),
    InterestSpec::new(
        "interest-photography",
        "Photography",
        "creative",
        0.10,
        0.15,
        0.20,
    ),
    InterestSpec::new("interest-writing", "Writing", "creative", 0.05, 0.25, 0.10),
    InterestSpec::new("interest-design", "Design", "creative", 0.20, 0.20, 0.15),
    InterestSpec::new(
        "interest-productivity",
        "Productivity",
        "practical",
        0.25,
        0.15,
        0.00,
    ),
    InterestSpec::new(
        "interest-learning",
        "Learning",
        "practical",
        0.35,
        0.20,
        0.00,
    ),
];
const BIO_ROLES: [&str; 14] = [
    "backend engineer",
    "forum moderator",
    "product analyst",
    "interface maintainer",
    "support lead",
    "research coordinator",
    "platform operator",
    "documentation writer",
    "automation specialist",
    "privacy reviewer",
    "data quality lead",
    "release manager",
    "toolsmith",
    "ops tinkerer",
];
const BIO_TECH_PHRASES: [&str; 4] = [
    "keeps an eye on implementation details",
    "likes reproducible setups",
    "documents the edge cases",
    "prefers measured technical tradeoffs",
];
const BIO_PLAYFUL_PHRASES: [&str; 4] = [
    "adds a dry joke when the thread can use it",
    "enjoys the occasional meme detour",
    "keeps the mood light without derailing",
    "collects oddly specific references",
];
const BIO_SUPPORTIVE_TONES: [&str; 3] = [
    "patient with newcomers",
    "quick to share context",
    "careful about giving credit",
];
const BIO_DIRECT_TONES: [&str; 3] = [
    "comfortable challenging fuzzy claims",
    "direct about weak assumptions",
    "willing to debate the tradeoff",
];
const BIO_NEUTRAL_TONES: [&str; 3] = [
    "keeps notes practical",
    "leans toward concrete examples",
    "prefers threads with clear next steps",
];

#[derive(Debug, Clone, Copy)]
struct InterestSpec {
    id: &'static str,
    label: &'static str,
    category: &'static str,
    technical_weight: f32,
    social_weight: f32,
    culture_weight: f32,
}

impl InterestSpec {
    const fn new(
        id: &'static str,
        label: &'static str,
        category: &'static str,
        technical_weight: f32,
        social_weight: f32,
        culture_weight: f32,
    ) -> Self {
        Self {
            id,
            label,
            category,
            technical_weight,
            social_weight,
            culture_weight,
        }
    }
}

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
    activity_events: &mut Vec<ActivityEvent>,
) -> Result<Vec<User>, ForumGenerationError> {
    (0..config.population.users)
        .map(|index| {
            let id = user_id(index)?;
            let entity_id = id.as_str();
            let first = choose(config, "users", entity_id, "first_name", &FIRST_NAMES);
            let last = choose(config, "users", entity_id, "last_name", &LAST_NAMES);
            let locale = choose(config, "users", entity_id, "locale", &LOCALES);
            let timestamp = account_created_at_timestamp(config, index, entity_id)?;
            let account_age = account_age_days(config, index, entity_id);
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
            user.status = Some(account_status(config, entity_id, account_age).to_string());
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

fn generate_personas(
    config: &ForumScenarioConfig,
    users: &mut [User],
    activity_events: &mut Vec<ActivityEvent>,
) -> Result<Vec<Persona>, ForumGenerationError> {
    users
        .iter_mut()
        .enumerate()
        .map(|(index, user)| {
            let id = persona_id(index)?;
            let entity_id = id.as_str();
            let timestamp = persona_created_at_timestamp(config, index, entity_id, user)?;
            let verbosity = *choose(config, "personas", entity_id, "verbosity", &VERBOSITIES);
            let sleep_phase = *choose(config, "personas", entity_id, "sleep_phase", &SLEEP_PHASES);
            let activity_pattern = activity_pattern_for(config, entity_id);
            let openness = persona_score(config, entity_id, "openness");
            let extroversion = persona_score(config, entity_id, "extroversion");
            let conscientiousness = persona_score(config, entity_id, "conscientiousness");
            let agreeableness = persona_score(config, entity_id, "agreeableness");
            let neuroticism = persona_score(config, entity_id, "neuroticism");
            let posting_frequency = persona_score(config, entity_id, "posting_frequency");
            let controversy_affinity = persona_score(config, entity_id, "controversy_affinity");
            let humor_affinity = persona_score(config, entity_id, "humor_affinity");
            let technical_depth = persona_score(config, entity_id, "technical_depth");
            let meme_affinity = persona_score(config, entity_id, "meme_affinity");
            let mut persona = Persona::new(
                id.clone(),
                user.id.clone(),
                persona_summary(verbosity, activity_pattern, technical_depth, humor_affinity),
                timestamp.clone(),
            )?;
            persona.traits = persona_traits(
                openness,
                extroversion,
                conscientiousness,
                agreeableness,
                technical_depth,
                meme_affinity,
            );
            persona.openness = openness;
            persona.extroversion = extroversion;
            persona.conscientiousness = conscientiousness;
            persona.agreeableness = agreeableness;
            persona.neuroticism = neuroticism;
            persona.posting_frequency = posting_frequency;
            persona.controversy_affinity = controversy_affinity;
            persona.humor_affinity = humor_affinity;
            persona.technical_depth = technical_depth;
            persona.meme_affinity = meme_affinity;
            persona.verbosity = verbosity;
            persona.sleep_phase = sleep_phase;
            persona.activity_pattern = activity_pattern;
            user.persona_id = Some(id.clone());
            push_activity(
                activity_events,
                ActivityEventKind::PersonaCreated,
                Some(user.id.clone()),
                ActivityObject::Persona(id),
                timestamp,
            )?;
            Ok(persona)
        })
        .collect()
}

fn generate_interests() -> Result<Vec<Interest>, ForumGenerationError> {
    INTEREST_CATALOG
        .iter()
        .map(|spec| {
            let mut interest = Interest::new(InterestId::new(spec.id)?, spec.label)?;
            interest.category = Some(spec.category.to_string());
            Ok(interest)
        })
        .collect()
}

fn assign_user_interests(
    config: &ForumScenarioConfig,
    users: &mut [User],
    personas: &[Persona],
    interests: &[Interest],
) {
    for (user, persona) in users.iter_mut().zip(personas) {
        let count =
            2 + deterministic_index(config, "interests", user.id.as_str(), "interest_count", 4);
        let mut scored: Vec<(usize, f32)> = INTEREST_CATALOG
            .iter()
            .enumerate()
            .map(|(index, spec)| {
                let tie_breaker = persona_interest_tie_breaker(config, persona, spec);
                (index, persona_interest_score(persona, spec) + tie_breaker)
            })
            .collect();
        scored.sort_by(|left, right| {
            right.1.total_cmp(&left.1).then_with(|| {
                INTEREST_CATALOG[left.0]
                    .id
                    .cmp(INTEREST_CATALOG[right.0].id)
            })
        });
        user.interest_ids = scored
            .into_iter()
            .take(count)
            .map(|(index, _)| interests[index].id.clone())
            .collect();
    }
}

fn assign_user_bios(
    config: &ForumScenarioConfig,
    users: &mut [User],
    personas: &[Persona],
    interests: &[Interest],
) {
    for (user, persona) in users.iter_mut().zip(personas) {
        let labels = assigned_interest_labels(user, interests);
        let primary_interest = labels
            .first()
            .copied()
            .expect("users receive interests before bios are generated");
        let secondary_interest = labels.get(1).copied().unwrap_or(primary_interest);
        let role = choose_phrase(config, user.id.as_str(), "role", &BIO_ROLES);
        let detail = bio_detail_phrase(config, user, persona);
        let tone = bio_tone_phrase(config, user, persona);
        let bio = match persona.verbosity {
            Verbosity::Terse => format!(
                "{}; into {}. {}.",
                capitalize_first(role),
                primary_interest,
                capitalize_first(tone)
            ),
            Verbosity::Balanced => format!(
                "{} focused on {} and {}; {} and {}.",
                capitalize_first(role),
                primary_interest,
                secondary_interest,
                detail,
                tone
            ),
            Verbosity::Detailed => format!(
                "{} who follows {} and {}; {}, {}, and prefers discussions with usable takeaways.",
                capitalize_first(role),
                primary_interest,
                secondary_interest,
                detail,
                tone
            ),
        };
        user.bio = Some(bio.trim().to_string());
    }
}

fn generate_memberships(
    config: &ForumScenarioConfig,
    users: &mut [User],
    personas: &[Persona],
    communities: &[Community],
    activity_events: &mut Vec<ActivityEvent>,
) -> Result<Vec<Relationship>, ForumGenerationError> {
    let mut relationships = Vec::new();
    let interest_communities = community_interest_indexes(communities);

    for (user_index, user) in users.iter_mut().enumerate() {
        let primary = choose_primary_community(
            config,
            user,
            &personas[user_index],
            communities,
            &interest_communities,
        );
        push_membership(
            &mut relationships,
            activity_events,
            config,
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
                config,
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
                config,
                &mut users[user_index],
                community,
                user_index,
                community_index,
            )?;
        }
    }

    Ok(relationships)
}

fn generate_social_relationships(
    config: &ForumScenarioConfig,
    users: &[User],
    personas: &[Persona],
    relationship_offset: usize,
    activity_events: &mut Vec<ActivityEvent>,
) -> Result<Vec<Relationship>, ForumGenerationError> {
    if users.len() <= 1 {
        return Ok(Vec::new());
    }

    let mut relationships = Vec::new();
    let mut emitted = HashSet::new();
    let mut friend_pairs = HashSet::new();
    let active_users = high_activity_user_indexes(users, personas);
    let ranked_by_user = users
        .iter()
        .enumerate()
        .map(|(user_index, user)| {
            ranked_social_candidates(config, users, personas, &active_users, user_index, user)
        })
        .collect::<Vec<_>>();

    for (source_index, ranked) in ranked_by_user.iter().enumerate() {
        let source_user = &users[source_index];
        let source_persona = &personas[source_index];
        let friend_limit = friend_limit_for(source_persona).min(ranked.len());

        for candidate in ranked.iter().take(friend_limit) {
            if candidate.affinity < friendship_threshold(source_persona, &personas[candidate.index])
            {
                continue;
            }

            let target_user = &users[candidate.index];
            let (source, target) = canonical_user_pair_users(source_user, target_user);
            if !friend_pairs.insert((source.id.to_string(), target.id.to_string())) {
                continue;
            }

            push_social_relationship(
                &mut relationships,
                activity_events,
                relationship_offset,
                RelationshipKind::Friend,
                config,
                source,
                target,
            )?;
        }
    }

    for (source_index, ranked) in ranked_by_user.iter().enumerate() {
        let source_user = &users[source_index];
        let follow_limit = follow_limit_for(&personas[source_index]).min(ranked.len());

        for candidate in ranked.iter().take(follow_limit) {
            let target_user = &users[candidate.index];
            let pair = canonical_user_pair(source_user, target_user);
            if friend_pairs.contains(&(pair.0.to_string(), pair.1.to_string())) {
                continue;
            }

            let key = (
                RelationshipKind::Follows,
                source_user.id.to_string(),
                target_user.id.to_string(),
            );
            if !emitted.insert(key) {
                continue;
            }

            push_social_relationship(
                &mut relationships,
                activity_events,
                relationship_offset,
                RelationshipKind::Follows,
                config,
                source_user,
                target_user,
            )?;
        }
    }

    Ok(relationships)
}

#[derive(Debug, Clone, Copy)]
struct SocialCandidate {
    index: usize,
    affinity: f32,
    score: f32,
}

fn ranked_social_candidates(
    config: &ForumScenarioConfig,
    users: &[User],
    personas: &[Persona],
    active_users: &[usize],
    source_index: usize,
    source_user: &User,
) -> Vec<SocialCandidate> {
    let candidate_budget = (users.len() - 1).min(32);
    let mut candidate_indexes = Vec::with_capacity(candidate_budget);
    let mut seen = HashSet::new();

    for offset in 1..=8.min(users.len() - 1) {
        push_social_candidate_index(
            &mut candidate_indexes,
            &mut seen,
            candidate_budget,
            (source_index + offset) % users.len(),
            source_index,
        );
        push_social_candidate_index(
            &mut candidate_indexes,
            &mut seen,
            candidate_budget,
            (source_index + users.len() - offset) % users.len(),
            source_index,
        );
    }

    for &candidate_index in active_users.iter().take(16) {
        push_social_candidate_index(
            &mut candidate_indexes,
            &mut seen,
            candidate_budget,
            candidate_index,
            source_index,
        );
    }

    for random_index in 0..(candidate_budget * 4) {
        if candidate_indexes.len() >= candidate_budget {
            break;
        }
        let offset = deterministic_social_offset(
            config,
            source_user.id.as_str(),
            "random_candidate",
            random_index,
            users.len() - 1,
        );
        let candidate_index = if offset >= source_index {
            offset + 1
        } else {
            offset
        };
        push_social_candidate_index(
            &mut candidate_indexes,
            &mut seen,
            candidate_budget,
            candidate_index,
            source_index,
        );
    }

    let source_persona = &personas[source_index];
    let mut ranked = candidate_indexes
        .into_iter()
        .map(|candidate_index| {
            let target_user = &users[candidate_index];
            let target_persona = &personas[candidate_index];
            let affinity = affinity_score(
                config,
                source_user,
                source_persona,
                target_user,
                target_persona,
            );
            let tie_breaker = directed_social_tie_breaker(
                config,
                source_user.id.as_str(),
                target_user.id.as_str(),
            );
            SocialCandidate {
                index: candidate_index,
                affinity,
                score: affinity * 0.70
                    + author_activity_score(target_user, target_persona) * 0.20
                    + tie_breaker * 0.10,
            }
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right.score.total_cmp(&left.score).then_with(|| {
            users[left.index]
                .id
                .as_str()
                .cmp(users[right.index].id.as_str())
        })
    });
    ranked
}

fn push_social_candidate_index(
    candidates: &mut Vec<usize>,
    seen: &mut HashSet<usize>,
    candidate_budget: usize,
    candidate_index: usize,
    source_index: usize,
) {
    if candidates.len() < candidate_budget
        && candidate_index != source_index
        && seen.insert(candidate_index)
    {
        candidates.push(candidate_index);
    }
}

fn high_activity_user_indexes(users: &[User], personas: &[Persona]) -> Vec<usize> {
    let mut indexes = (0..personas.len()).collect::<Vec<_>>();
    indexes.sort_by(|left, right| {
        author_activity_score(&users[*right], &personas[*right])
            .total_cmp(&author_activity_score(&users[*left], &personas[*left]))
            .then_with(|| {
                personas[*left]
                    .user_id
                    .as_str()
                    .cmp(personas[*right].user_id.as_str())
            })
    });
    indexes
}

fn deterministic_social_offset(
    config: &ForumScenarioConfig,
    source_id: &str,
    field: &str,
    index: usize,
    len: usize,
) -> usize {
    let upper_bound = u64::try_from(len).expect("scenario count should fit in u64");
    let value = random_bounded_u64(
        &config.seed,
        "social_relationships",
        source_id,
        format!("{field}_{index}"),
        upper_bound,
    )
    .expect("non-zero social candidate bound should produce a value");

    usize::try_from(value).expect("bounded random index should fit in usize")
}

fn follow_limit_for(persona: &Persona) -> usize {
    match persona.activity_pattern {
        ActivityPattern::Lurker => 1,
        ActivityPattern::Casual => 2,
        ActivityPattern::Regular => 4,
        ActivityPattern::Bursty => 6,
        ActivityPattern::PowerUser => 8,
    }
}

fn friend_limit_for(persona: &Persona) -> usize {
    match persona.activity_pattern {
        ActivityPattern::Lurker | ActivityPattern::Casual => 1,
        ActivityPattern::Regular | ActivityPattern::Bursty => 2,
        ActivityPattern::PowerUser => 3,
    }
}

fn friendship_threshold(source: &Persona, target: &Persona) -> f32 {
    let sociability = (source.extroversion + target.extroversion) / 2.0;
    0.74 - sociability * 0.08
}

fn canonical_user_pair<'a>(left: &'a User, right: &'a User) -> (&'a UserId, &'a UserId) {
    if left.id.as_str() <= right.id.as_str() {
        (&left.id, &right.id)
    } else {
        (&right.id, &left.id)
    }
}

fn canonical_user_pair_users<'a>(left: &'a User, right: &'a User) -> (&'a User, &'a User) {
    if left.id.as_str() <= right.id.as_str() {
        (left, right)
    } else {
        (right, left)
    }
}

fn push_social_relationship(
    relationships: &mut Vec<Relationship>,
    activity_events: &mut Vec<ActivityEvent>,
    relationship_offset: usize,
    kind: RelationshipKind,
    config: &ForumScenarioConfig,
    source: &User,
    target: &User,
) -> Result<(), ForumGenerationError> {
    let relationship_index = relationship_offset + relationships.len();
    let relationship_id =
        RelationshipId::new(format!("relationship-{:06}", relationship_index + 1))?;
    let timestamp = relationship_timestamp_after_users(
        config,
        "social_relationship",
        relationship_index,
        source,
        Some(target),
    )?;

    relationships.push(Relationship {
        id: relationship_id.clone(),
        source: RelationshipEndpoint::User(source.id.clone()),
        target: RelationshipEndpoint::User(target.id.clone()),
        kind,
        created_at: timestamp.clone(),
    });
    push_activity(
        activity_events,
        ActivityEventKind::RelationshipCreated,
        Some(source.id.clone()),
        ActivityObject::Relationship(relationship_id),
        timestamp,
    )
}

fn choose_primary_community(
    config: &ForumScenarioConfig,
    user: &User,
    persona: &Persona,
    communities: &[Community],
    interest_communities: &HashMap<String, usize>,
) -> usize {
    for interest_id in &user.interest_ids {
        let key = interest_id
            .as_str()
            .strip_prefix("interest-")
            .unwrap_or_else(|| interest_id.as_str());
        if let Some(index) = interest_communities.get(key) {
            return *index;
        }
    }

    if persona.extroversion + persona.posting_frequency >= 1.25 {
        if let Some(index) = interest_communities.get("community") {
            return *index;
        }
    }

    deterministic_index(
        config,
        "memberships",
        user.id.as_str(),
        "primary_community",
        communities.len(),
    )
}

fn community_interest_indexes(communities: &[Community]) -> HashMap<String, usize> {
    let interest_slugs = INTEREST_CATALOG
        .iter()
        .map(|interest| slug(interest.label))
        .collect::<Vec<_>>();
    let mut indexes = HashMap::new();

    for (community_index, community) in communities.iter().enumerate() {
        let community_slug = slug(&community.name);
        if interest_slugs
            .iter()
            .any(|interest| interest == &community_slug)
        {
            indexes.entry(community_slug).or_insert(community_index);
        }
    }

    indexes
}

fn push_membership(
    relationships: &mut Vec<Relationship>,
    activity_events: &mut Vec<ActivityEvent>,
    config: &ForumScenarioConfig,
    user: &mut User,
    community: &Community,
    user_index: usize,
    community_index: usize,
) -> Result<(), ForumGenerationError> {
    let relationship_index = relationships.len();
    let relationship_id =
        RelationshipId::new(format!("relationship-{:06}", relationship_index + 1))?;
    let timestamp = relationship_timestamp_after_users(
        config,
        "membership",
        user_index + community_index,
        user,
        None,
    )?;

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
    personas: &[Persona],
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
                personas,
                &community_members[community_index],
                id.as_str(),
            );
            let topic = choose(config, "posts", id.as_str(), "topic", &POST_TOPICS);
            let persona = persona_for_user(personas, &author.id);
            let timestamp = scheduled_post_timestamp(config, index, id.as_str(), author, persona)?;
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
    personas: &[Persona],
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
                personas,
                &community_members[*community_index],
                post.author_id.as_str(),
                id.as_str(),
            );
            let body = choose(config, "comments", id.as_str(), "body", &COMMENT_TONES);
            let persona = persona_for_user(personas, &author.id);
            let timestamp =
                scheduled_comment_timestamp(config, index, id.as_str(), post, author, persona)?;
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
    personas: &[Persona],
    members: &[usize],
    entity_id: &str,
) -> &'a User {
    let index = choose_weighted_member_index(config, users, personas, members, "posts", entity_id);

    &users[index]
}

fn choose_comment_author<'a>(
    config: &ForumScenarioConfig,
    users: &'a [User],
    personas: &[Persona],
    members: &[usize],
    post_author_id: &str,
    entity_id: &str,
) -> &'a User {
    let author_index =
        choose_weighted_member_index(config, users, personas, members, "comments", entity_id);
    let author = &users[author_index];

    if members.len() == 1 || author.id.as_str() != post_author_id {
        return author;
    }

    let fallback_offset = deterministic_index(
        config,
        "comments",
        entity_id,
        "fallback_author",
        members.len(),
    );
    for offset in 1..=members.len() {
        let candidate = &users[members[(fallback_offset + offset) % members.len()]];
        if candidate.id.as_str() != post_author_id {
            return candidate;
        }
    }

    author
}

fn choose_weighted_member_index(
    config: &ForumScenarioConfig,
    users: &[User],
    personas: &[Persona],
    members: &[usize],
    namespace: &str,
    entity_id: &str,
) -> usize {
    let candidate_count = members.len().min(3);
    let mut best =
        members[deterministic_index(config, namespace, entity_id, "author", members.len())];
    let mut best_score = author_activity_score(&users[best], &personas[best])
        + author_tie_breaker(config, namespace, entity_id, personas[best].id.as_str());

    for candidate_index in 0..candidate_count {
        let field = match candidate_index {
            0 => "author_candidate_0",
            1 => "author_candidate_1",
            _ => "author_candidate_2",
        };
        let offset = deterministic_index(config, namespace, entity_id, field, members.len());
        let user_index = members[offset];
        let score = author_activity_score(&users[user_index], &personas[user_index])
            + author_tie_breaker(
                config,
                namespace,
                entity_id,
                personas[user_index].id.as_str(),
            );

        if score > best_score {
            best = user_index;
            best_score = score;
        }
    }

    best
}

fn author_activity_score(user: &User, persona: &Persona) -> f32 {
    let activity = match persona.activity_pattern {
        ActivityPattern::Lurker => 0.05,
        ActivityPattern::Casual => 0.25,
        ActivityPattern::Regular => 0.55,
        ActivityPattern::Bursty => 0.70,
        ActivityPattern::PowerUser => 0.90,
    };

    let base = persona.posting_frequency * 0.50 + persona.extroversion * 0.25 + activity * 0.25;
    let maturity = (account_age_days_for_user(user) as f32 / ACCOUNT_REFERENCE_DAY as f32)
        .clamp(0.0, 1.0)
        * 0.12;

    ((base + maturity) * account_activity_multiplier(user)).clamp(0.0, 1.0)
}

fn author_tie_breaker(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    persona_id: &str,
) -> f32 {
    let value = random_bounded_u64(&config.seed, namespace, entity_id, persona_id, 1_000)
        .expect("non-zero author tie-breaker bound should produce a value");

    (value as f32) / 1_000_000.0
}

fn persona_for_user<'a>(personas: &'a [Persona], user_id: &UserId) -> &'a Persona {
    personas
        .iter()
        .find(|persona| &persona.user_id == user_id)
        .expect("generated users should have matching personas")
}

fn account_age_days_for_user(user: &User) -> usize {
    let created_at = minutes_from_timestamp(&user.created_at);

    if created_at >= CONTENT_START_MINUTES {
        0
    } else {
        (CONTENT_START_MINUTES - created_at) / MINUTES_PER_DAY
    }
}

fn account_activity_multiplier(user: &User) -> f32 {
    match user.status.as_deref() {
        Some("dormant") => 0.15,
        Some("veteran") => 1.08,
        _ if account_age_days_for_user(user) <= 45 => 0.75,
        _ => 1.0,
    }
}

fn affinity_score(
    config: &ForumScenarioConfig,
    left_user: &User,
    left_persona: &Persona,
    right_user: &User,
    right_persona: &Persona,
) -> f32 {
    let score = shared_interest_score(left_user, right_user) * 0.40
        + personality_compatibility(left_persona, right_persona) * 0.25
        + locale_similarity(&left_user.locale, &right_user.locale) * 0.15
        + activity_overlap(left_persona, right_persona) * 0.15
        + affinity_tie_breaker(config, left_user.id.as_str(), right_user.id.as_str()) * 0.05;

    score.clamp(0.0, 1.0)
}

fn shared_interest_score(left: &User, right: &User) -> f32 {
    let shared = left
        .interest_ids
        .iter()
        .filter(|interest_id| right.interest_ids.contains(interest_id))
        .count();
    let union = left.interest_ids.len() + right.interest_ids.len() - shared;

    if union == 0 {
        0.0
    } else {
        shared as f32 / union as f32
    }
}

fn personality_compatibility(left: &Persona, right: &Persona) -> f32 {
    let similarity = [
        (left.openness, right.openness),
        (left.extroversion, right.extroversion),
        (left.conscientiousness, right.conscientiousness),
        (left.agreeableness, right.agreeableness),
        (left.neuroticism, right.neuroticism),
        (left.technical_depth, right.technical_depth),
        (left.humor_affinity, right.humor_affinity),
        (left.meme_affinity, right.meme_affinity),
    ]
    .into_iter()
    .map(|(left, right)| 1.0 - (left - right).abs())
    .sum::<f32>()
        / 8.0;

    let controversy = (left.controversy_affinity + right.controversy_affinity) / 2.0;
    let agreeableness = (left.agreeableness + right.agreeableness) / 2.0;
    let conflict_penalty = if controversy >= 0.70 && agreeableness <= 0.35 {
        0.25
    } else if controversy >= 0.62 && agreeableness <= 0.45 {
        0.12
    } else {
        0.0
    };

    (similarity - conflict_penalty).clamp(0.0, 1.0)
}

fn locale_similarity(left: &Locale, right: &Locale) -> f32 {
    if left == right {
        return 1.0;
    }

    match (
        left.as_str().split_once('-'),
        right.as_str().split_once('-'),
    ) {
        (Some((left_language, _)), Some((right_language, _)))
            if left_language == right_language =>
        {
            0.70
        }
        _ => 0.0,
    }
}

fn activity_overlap(left: &Persona, right: &Persona) -> f32 {
    let sleep = match (left.sleep_phase, right.sleep_phase) {
        (left, right) if left == right => 1.0,
        (SleepPhase::Irregular, _) | (_, SleepPhase::Irregular) => 0.60,
        (SleepPhase::EarlyBird, SleepPhase::Daytime)
        | (SleepPhase::Daytime, SleepPhase::EarlyBird)
        | (SleepPhase::Daytime, SleepPhase::NightOwl)
        | (SleepPhase::NightOwl, SleepPhase::Daytime) => 0.65,
        _ => 0.20,
    };
    let cadence = 1.0 - (left.posting_frequency - right.posting_frequency).abs();

    (sleep * 0.70 + cadence * 0.30).clamp(0.0, 1.0)
}

fn affinity_tie_breaker(config: &ForumScenarioConfig, left_id: &str, right_id: &str) -> f32 {
    let (first, second) = if left_id <= right_id {
        (left_id, right_id)
    } else {
        (right_id, left_id)
    };
    let value = random_bounded_u64(&config.seed, "affinity", first, second, 1_000)
        .expect("non-zero affinity tie-breaker bound should produce a value");

    (value as f32) / 999.0
}

fn directed_social_tie_breaker(
    config: &ForumScenarioConfig,
    source_id: &str,
    target_id: &str,
) -> f32 {
    let value = random_bounded_u64(
        &config.seed,
        "social_relationships",
        source_id,
        target_id,
        1_000,
    )
    .expect("non-zero directed tie-breaker bound should produce a value");

    (value as f32) / 999.0
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

fn sort_activity_events(activity_events: &mut [ActivityEvent]) {
    activity_events.sort_by(|left, right| {
        left.occurred_at
            .as_str()
            .cmp(right.occurred_at.as_str())
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });
}

fn validate_stable_timestamp_values(dataset: &ForumDataset, errors: &mut Vec<String>) {
    for user in &dataset.users {
        require_stable_timestamp("user", user.id.as_str(), &user.created_at, errors);
    }
    for persona in &dataset.personas {
        require_stable_timestamp("persona", persona.id.as_str(), &persona.created_at, errors);
    }
    for community in &dataset.communities {
        require_stable_timestamp(
            "community",
            community.id.as_str(),
            &community.created_at,
            errors,
        );
    }
    for post in &dataset.posts {
        require_stable_timestamp("post", post.id.as_str(), &post.created_at, errors);
    }
    for comment in &dataset.comments {
        require_stable_timestamp("comment", comment.id.as_str(), &comment.created_at, errors);
    }
    for relationship in &dataset.relationships {
        require_stable_timestamp(
            "relationship",
            relationship.id.as_str(),
            &relationship.created_at,
            errors,
        );
    }
    for event in &dataset.activity_events {
        require_stable_timestamp("activity", event.id.as_str(), &event.occurred_at, errors);
    }
}

fn validate_posts_after_authors(
    dataset: &ForumDataset,
    user_created_at: &HashMap<UserId, usize>,
    errors: &mut Vec<String>,
) {
    for post in &dataset.posts {
        require_after_user(
            "post",
            post.id.as_str(),
            &post.author_id,
            minutes_from_timestamp(&post.created_at),
            user_created_at,
            errors,
        );
    }
}

fn validate_comments_after_posts(
    dataset: &ForumDataset,
    user_created_at: &HashMap<UserId, usize>,
    post_by_id: &HashMap<PostId, &Post>,
    errors: &mut Vec<String>,
) {
    for comment in &dataset.comments {
        let comment_minutes = minutes_from_timestamp(&comment.created_at);
        require_after_user(
            "comment",
            comment.id.as_str(),
            &comment.author_id,
            comment_minutes,
            user_created_at,
            errors,
        );
        match post_by_id.get(&comment.post_id) {
            Some(post) if comment_minutes > minutes_from_timestamp(&post.created_at) => {}
            Some(post) => errors.push(format!(
                "comment {} occurs at or before post {}",
                comment.id, post.id
            )),
            None => errors.push(format!("comment {} references missing post", comment.id)),
        }
    }
}

fn validate_relationships_after_users(
    dataset: &ForumDataset,
    user_created_at: &HashMap<UserId, usize>,
    errors: &mut Vec<String>,
) {
    for relationship in &dataset.relationships {
        let relationship_minutes = minutes_from_timestamp(&relationship.created_at);
        require_relationship_endpoint_after_user(
            relationship.id.as_str(),
            &relationship.source,
            relationship_minutes,
            user_created_at,
            errors,
        );
        require_relationship_endpoint_after_user(
            relationship.id.as_str(),
            &relationship.target,
            relationship_minutes,
            user_created_at,
            errors,
        );
    }
}

fn validate_activity_events(
    dataset: &ForumDataset,
    user_created_at: &HashMap<UserId, usize>,
    post_by_id: &HashMap<PostId, &Post>,
    comment_by_id: &HashMap<CommentId, &Comment>,
    relationship_by_id: &HashMap<RelationshipId, &Relationship>,
    errors: &mut Vec<String>,
) {
    let mut previous = None;

    for event in &dataset.activity_events {
        let event_minutes = minutes_from_timestamp(&event.occurred_at);
        if let Some(previous_minutes) = previous {
            if event_minutes < previous_minutes {
                errors.push(format!("activity event {} is out of order", event.id));
            }
        }
        previous = Some(event_minutes);

        if let Some(actor_id) = &event.actor_id {
            require_after_user(
                "activity",
                event.id.as_str(),
                actor_id,
                event_minutes,
                user_created_at,
                errors,
            );
        }

        validate_activity_object_time(
            event,
            event_minutes,
            post_by_id,
            comment_by_id,
            relationship_by_id,
            errors,
        );
    }
}

fn validate_activity_object_time(
    event: &ActivityEvent,
    event_minutes: usize,
    post_by_id: &HashMap<PostId, &Post>,
    comment_by_id: &HashMap<CommentId, &Comment>,
    relationship_by_id: &HashMap<RelationshipId, &Relationship>,
    errors: &mut Vec<String>,
) {
    match &event.object {
        ActivityObject::Post(post_id) if event.kind == ActivityEventKind::PostCreated => {
            require_activity_matches(
                event,
                event_minutes,
                post_by_id.get(post_id),
                "post",
                errors,
            );
        }
        ActivityObject::Comment(comment_id) if event.kind == ActivityEventKind::CommentCreated => {
            require_activity_matches(
                event,
                event_minutes,
                comment_by_id.get(comment_id),
                "comment",
                errors,
            );
        }
        ActivityObject::Relationship(relationship_id)
            if event.kind == ActivityEventKind::RelationshipCreated =>
        {
            require_activity_matches(
                event,
                event_minutes,
                relationship_by_id.get(relationship_id),
                "relationship",
                errors,
            );
        }
        _ => {}
    }
}

trait CreatedAt {
    fn created_at(&self) -> &ModelTimestamp;
    fn id_text(&self) -> String;
}

impl CreatedAt for Post {
    fn created_at(&self) -> &ModelTimestamp {
        &self.created_at
    }

    fn id_text(&self) -> String {
        self.id.to_string()
    }
}

impl CreatedAt for Comment {
    fn created_at(&self) -> &ModelTimestamp {
        &self.created_at
    }

    fn id_text(&self) -> String {
        self.id.to_string()
    }
}

impl CreatedAt for Relationship {
    fn created_at(&self) -> &ModelTimestamp {
        &self.created_at
    }

    fn id_text(&self) -> String {
        self.id.to_string()
    }
}

fn require_activity_matches<T: CreatedAt>(
    event: &ActivityEvent,
    event_minutes: usize,
    record: Option<&&T>,
    entity: &str,
    errors: &mut Vec<String>,
) {
    match record {
        Some(record) if event_minutes == minutes_from_timestamp(record.created_at()) => {}
        Some(record) => errors.push(format!(
            "{entity} activity {} does not match {entity} {} timestamp",
            event.id,
            record.id_text()
        )),
        None => errors.push(format!(
            "{} activity {} references missing record",
            entity, event.id
        )),
    }
}

fn require_stable_timestamp(
    entity: &str,
    id: &str,
    timestamp: &ModelTimestamp,
    errors: &mut Vec<String>,
) {
    match parse_timestamp_minutes(timestamp.as_str()) {
        Some(minutes) if timestamp.as_str() == timestamp_from_minutes(minutes) => {}
        _ => errors.push(format!("{entity} {id} has unstable timestamp {timestamp}")),
    }
}

fn require_after_user(
    entity: &str,
    id: &str,
    user_id: &UserId,
    occurred_at: usize,
    user_created_at: &HashMap<UserId, usize>,
    errors: &mut Vec<String>,
) {
    match user_created_at.get(user_id) {
        Some(created_at) if occurred_at >= *created_at => {}
        Some(_) => errors.push(format!("{entity} {id} occurs before user {user_id} exists")),
        None => errors.push(format!("{entity} {id} references missing user {user_id}")),
    }
}

fn require_relationship_endpoint_after_user(
    relationship_id: &str,
    endpoint: &RelationshipEndpoint,
    occurred_at: usize,
    user_created_at: &HashMap<UserId, usize>,
    errors: &mut Vec<String>,
) {
    if let RelationshipEndpoint::User(user_id) = endpoint {
        require_after_user(
            "relationship",
            relationship_id,
            user_id,
            occurred_at,
            user_created_at,
            errors,
        );
    }
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

fn persona_id(index: usize) -> Result<PersonaId, ModelValidationError> {
    PersonaId::new(format!("persona-{:06}", index + 1))
}

fn community_id(index: usize, name: &str) -> Result<CommunityId, ModelValidationError> {
    CommunityId::new(format!("community-{:06}-{}", index + 1, slug(name)))
}

fn persona_score(config: &ForumScenarioConfig, entity_id: &str, field: &str) -> f32 {
    let value = random_bounded_u64(&config.seed, "personas", entity_id, field, 1_001)
        .expect("non-zero persona score bound should produce a value");

    (value as f32) / 1_000.0
}

fn activity_pattern_for(config: &ForumScenarioConfig, persona_id: &str) -> ActivityPattern {
    let value = random_bounded_u64(
        &config.seed,
        "personas",
        persona_id,
        "activity_pattern",
        10_000,
    )
    .expect("non-zero activity-pattern bound should produce a value");

    match value {
        0..=4_499 => ActivityPattern::Lurker,
        4_500..=7_499 => ActivityPattern::Casual,
        7_500..=9_199 => ActivityPattern::Regular,
        9_200..=9_799 => ActivityPattern::Bursty,
        _ => ActivityPattern::PowerUser,
    }
}

fn persona_interest_score(persona: &Persona, spec: &InterestSpec) -> f32 {
    let technical = persona.technical_depth * spec.technical_weight;
    let social = ((persona.extroversion + persona.posting_frequency) / 2.0) * spec.social_weight;
    let culture = ((persona.meme_affinity + persona.humor_affinity) / 2.0) * spec.culture_weight;
    let openness = persona.openness * 0.12;
    let conscientiousness = persona.conscientiousness * 0.04;

    technical + social + culture + openness + conscientiousness
}

fn persona_interest_tie_breaker(
    config: &ForumScenarioConfig,
    persona: &Persona,
    spec: &InterestSpec,
) -> f32 {
    let value = random_bounded_u64(
        &config.seed,
        "interests",
        persona.id.as_str(),
        spec.id,
        1_000,
    )
    .expect("non-zero interest tie-breaker bound should produce a value");

    (value as f32) / 1_000_000.0
}

fn assigned_interest_labels<'a>(user: &User, interests: &'a [Interest]) -> Vec<&'a str> {
    user.interest_ids
        .iter()
        .filter_map(|interest_id| {
            interests
                .iter()
                .find(|interest| interest.id == *interest_id)
                .map(|interest| interest.label.as_str())
        })
        .collect()
}

fn bio_detail_phrase(config: &ForumScenarioConfig, user: &User, persona: &Persona) -> &'static str {
    if persona.technical_depth >= 0.62 {
        return choose_phrase(
            config,
            user.id.as_str(),
            "technical_phrase",
            &BIO_TECH_PHRASES,
        );
    }

    if persona.humor_affinity >= 0.62 || persona.meme_affinity >= 0.62 {
        return choose_phrase(
            config,
            user.id.as_str(),
            "playful_phrase",
            &BIO_PLAYFUL_PHRASES,
        );
    }

    choose_phrase(
        config,
        user.id.as_str(),
        "neutral_detail",
        &BIO_NEUTRAL_TONES,
    )
}

fn bio_tone_phrase(config: &ForumScenarioConfig, user: &User, persona: &Persona) -> &'static str {
    if persona.agreeableness >= 0.62 {
        return choose_phrase(
            config,
            user.id.as_str(),
            "supportive_tone",
            &BIO_SUPPORTIVE_TONES,
        );
    }

    if persona.controversy_affinity >= 0.62 {
        return choose_phrase(config, user.id.as_str(), "direct_tone", &BIO_DIRECT_TONES);
    }

    choose_phrase(config, user.id.as_str(), "neutral_tone", &BIO_NEUTRAL_TONES)
}

fn persona_summary(
    verbosity: Verbosity,
    activity_pattern: ActivityPattern,
    technical_depth: f32,
    humor_affinity: f32,
) -> String {
    let detail = if technical_depth >= 0.67 {
        "technical"
    } else if humor_affinity >= 0.67 {
        "playful"
    } else {
        "practical"
    };
    let cadence = match activity_pattern {
        ActivityPattern::Lurker => "quiet reader",
        ActivityPattern::Casual => "casual participant",
        ActivityPattern::Regular => "regular contributor",
        ActivityPattern::Bursty => "bursty contributor",
        ActivityPattern::PowerUser => "high-volume contributor",
    };
    let style = match verbosity {
        Verbosity::Terse => "concise",
        Verbosity::Balanced => "balanced",
        Verbosity::Detailed => "detailed",
    };

    format!("{style} {detail} {cadence}")
}

fn persona_traits(
    openness: f32,
    extroversion: f32,
    conscientiousness: f32,
    agreeableness: f32,
    technical_depth: f32,
    meme_affinity: f32,
) -> Vec<String> {
    [
        (openness, "curious"),
        (extroversion, "social"),
        (conscientiousness, "organized"),
        (agreeableness, "supportive"),
        (technical_depth, "technical"),
        (meme_affinity, "playful"),
    ]
    .into_iter()
    .filter_map(|(score, label)| (score >= 0.6).then_some(label.to_string()))
    .collect()
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

fn choose_phrase(
    config: &ForumScenarioConfig,
    entity_id: &str,
    field: &str,
    values: &'static [&'static str],
) -> &'static str {
    values[deterministic_index(config, "bios", entity_id, field, values.len())]
}

fn timestamp_for(namespace: &str, index: usize) -> Result<ModelTimestamp, ModelValidationError> {
    let base_minutes = match namespace {
        "community" => 0,
        "user" => 10_000,
        "persona" => 15_000,
        "membership" => 20_000,
        "post" => 30_000,
        "comment" => 40_000,
        _ => 50_000,
    };

    ModelTimestamp::new(timestamp_from_minutes(base_minutes + index * 7))
}

const MINUTES_PER_DAY: usize = 1_440;
const ACCOUNT_REFERENCE_DAY: usize = 420;
const CONTENT_START_MINUTES: usize = ACCOUNT_REFERENCE_DAY * MINUTES_PER_DAY;

fn account_created_at_timestamp(
    config: &ForumScenarioConfig,
    index: usize,
    entity_id: &str,
) -> Result<ModelTimestamp, ModelValidationError> {
    let age_days = account_age_days(config, index, entity_id);
    let minute_of_day =
        deterministic_index(config, "accounts", entity_id, "join_minute", 1_020) + 8 * 60;
    let created_minutes = CONTENT_START_MINUTES - age_days * MINUTES_PER_DAY + minute_of_day;

    ModelTimestamp::new(timestamp_from_minutes(created_minutes))
}

fn persona_created_at_timestamp(
    config: &ForumScenarioConfig,
    index: usize,
    entity_id: &str,
    user: &User,
) -> Result<ModelTimestamp, ModelValidationError> {
    let base = minutes_from_timestamp(&timestamp_for("persona", index)?);
    let delay = 20 + deterministic_index(config, "personas", entity_id, "creation_delay", 180);

    ModelTimestamp::new(timestamp_from_minutes(
        base.max(minutes_from_timestamp(&user.created_at) + delay),
    ))
}

fn account_age_days(config: &ForumScenarioConfig, index: usize, entity_id: &str) -> usize {
    let band = deterministic_index(config, "accounts", entity_id, "age_band", 100);
    let offset = deterministic_index(config, "accounts", entity_id, "age_offset", 90);
    let cohort_age = match band {
        0..=14 => 7 + offset.min(28),
        15..=34 => 45 + offset.min(75),
        35..=79 => 120 + offset * 2,
        _ => 300 + offset,
    };
    let index_spread = index % 21;

    (cohort_age + index_spread).min(ACCOUNT_REFERENCE_DAY - 1)
}

fn account_status(
    config: &ForumScenarioConfig,
    entity_id: &str,
    account_age_days: usize,
) -> &'static str {
    let dormancy = deterministic_index(config, "accounts", entity_id, "dormancy", 100);

    if account_age_days >= 180 && dormancy < 20 {
        "dormant"
    } else if account_age_days >= 300 {
        "veteran"
    } else {
        "active"
    }
}

fn relationship_timestamp_after_users(
    config: &ForumScenarioConfig,
    namespace: &str,
    index: usize,
    source: &User,
    target: Option<&User>,
) -> Result<ModelTimestamp, ModelValidationError> {
    let base = minutes_from_timestamp(&timestamp_for(namespace, index)?);
    let target_created_at = target
        .map(|user| minutes_from_timestamp(&user.created_at))
        .unwrap_or(0);
    let minimum = minutes_from_timestamp(&source.created_at).max(target_created_at)
        + 60
        + deterministic_index(
            config,
            namespace,
            source.id.as_str(),
            "relationship_delay",
            2_880,
        );

    ModelTimestamp::new(timestamp_from_minutes(base.max(minimum)))
}

fn scheduled_post_timestamp(
    config: &ForumScenarioConfig,
    index: usize,
    entity_id: &str,
    author: &User,
    persona: &Persona,
) -> Result<ModelTimestamp, ModelValidationError> {
    ModelTimestamp::new(timestamp_from_minutes(scheduled_activity_minutes(
        config, "posts", index, entity_id, author, persona,
    )))
}

fn scheduled_comment_timestamp(
    config: &ForumScenarioConfig,
    index: usize,
    entity_id: &str,
    post: &Post,
    author: &User,
    persona: &Persona,
) -> Result<ModelTimestamp, ModelValidationError> {
    let candidate =
        scheduled_activity_minutes(config, "comments", index, entity_id, author, persona);
    let reply_delay = comment_reply_delay_minutes(config, entity_id, persona);
    let minimum = minutes_from_timestamp(&post.created_at) + reply_delay;
    let minutes = if candidate >= minimum {
        candidate
    } else {
        let days_needed = (minimum - candidate) / MINUTES_PER_DAY + 1;
        candidate + days_needed * MINUTES_PER_DAY
    };

    ModelTimestamp::new(timestamp_from_minutes(minutes))
}

fn scheduled_activity_minutes(
    config: &ForumScenarioConfig,
    namespace: &str,
    index: usize,
    entity_id: &str,
    author: &User,
    persona: &Persona,
) -> usize {
    let day = scheduled_activity_day(config, namespace, index, entity_id, author, persona);
    let local_hour = scheduled_local_hour(config, namespace, entity_id, day, author, persona);
    let local_minute = scheduled_local_minute(config, namespace, entity_id, persona);
    let timezone_offset = timezone_offset_hours(config, author);
    let utc_hour = (local_hour + 24 - timezone_offset).rem_euclid(24);
    let scheduled =
        CONTENT_START_MINUTES + day * MINUTES_PER_DAY + utc_hour as usize * 60 + local_minute;
    let minimum = minutes_from_timestamp(&author.created_at) + 60;

    if scheduled >= minimum {
        scheduled
    } else {
        scheduled + ((minimum - scheduled) / MINUTES_PER_DAY + 1) * MINUTES_PER_DAY
    }
}

fn scheduled_activity_day(
    config: &ForumScenarioConfig,
    namespace: &str,
    index: usize,
    entity_id: &str,
    author: &User,
    persona: &Persona,
) -> usize {
    let cluster_size = activity_cluster_size(persona.activity_pattern);
    let cadence = activity_cadence_days(persona.activity_pattern);
    let cluster = index / cluster_size;
    let anchor = deterministic_index(
        config,
        "temporal",
        author.id.as_str(),
        "activity_day_anchor",
        14,
    );
    let cluster_day = deterministic_index(config, namespace, entity_id, "activity_day", cadence);
    let burst_day = deterministic_index(config, namespace, entity_id, "burst_day", 3);

    anchor
        + cluster * cadence
        + if matches!(
            persona.activity_pattern,
            ActivityPattern::Bursty | ActivityPattern::PowerUser
        ) {
            burst_day
        } else {
            cluster_day
        }
}

fn scheduled_local_hour(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    day: usize,
    author: &User,
    persona: &Persona,
) -> i32 {
    let (start, width) = active_hour_window(persona);
    let mut hour = start
        + i32::try_from(deterministic_index(
            config,
            namespace,
            entity_id,
            "active_hour",
            width,
        ))
        .expect("active hour should fit in i32");

    if is_weekend(day) {
        hour += weekend_hour_shift(persona.activity_pattern);
    }

    if matches!(
        persona.activity_pattern,
        ActivityPattern::Bursty | ActivityPattern::PowerUser
    ) {
        let anchor = burst_anchor_hour(config, author, persona);
        let offset = i32::try_from(deterministic_index(
            config,
            namespace,
            entity_id,
            "burst_hour_offset",
            3,
        ))
        .expect("burst hour offset should fit in i32")
            - 1;
        hour = (hour + anchor + offset) / 2;
    }

    hour.rem_euclid(24)
}

fn scheduled_local_minute(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    persona: &Persona,
) -> usize {
    let minute = deterministic_index(config, namespace, entity_id, "active_minute", 60);

    if persona.activity_pattern == ActivityPattern::Bursty {
        (minute / 15) * 15
    } else {
        minute
    }
}

fn active_hour_window(persona: &Persona) -> (i32, usize) {
    match (persona.sleep_phase, persona.activity_pattern) {
        (SleepPhase::EarlyBird, ActivityPattern::PowerUser) => (6, 5),
        (SleepPhase::EarlyBird, _) => (7, 4),
        (SleepPhase::Daytime, ActivityPattern::PowerUser) => (10, 8),
        (SleepPhase::Daytime, _) => (12, 7),
        (SleepPhase::NightOwl, ActivityPattern::PowerUser) => (18, 6),
        (SleepPhase::NightOwl, _) => (19, 5),
        (SleepPhase::Irregular, ActivityPattern::Bursty) => (16, 8),
        (SleepPhase::Irregular, _) => (8, 14),
    }
}

fn burst_anchor_hour(config: &ForumScenarioConfig, author: &User, persona: &Persona) -> i32 {
    let base = match persona.sleep_phase {
        SleepPhase::EarlyBird => 8,
        SleepPhase::Daytime => 15,
        SleepPhase::NightOwl => 21,
        SleepPhase::Irregular => 18,
    };
    let jitter = deterministic_index(
        config,
        "temporal",
        author.id.as_str(),
        "burst_anchor_hour",
        5,
    ) as i32
        - 2;

    base + jitter
}

fn timezone_offset_hours(config: &ForumScenarioConfig, user: &User) -> i32 {
    let base = match user.locale.as_str() {
        "en-GB" => 0,
        "en-AU" => 10,
        "en-NZ" => 12,
        "en-CA" | "en-US" => -5,
        _ => 0,
    };
    let jitter =
        deterministic_index(config, "temporal", user.id.as_str(), "timezone_jitter", 3) as i32 - 1;

    (base + jitter).clamp(-11, 13)
}

fn is_weekend(day: usize) -> bool {
    matches!((3 + day) % 7, 5 | 6)
}

fn weekend_hour_shift(pattern: ActivityPattern) -> i32 {
    match pattern {
        ActivityPattern::Lurker => 1,
        ActivityPattern::Casual | ActivityPattern::Bursty => 2,
        ActivityPattern::Regular => 1,
        ActivityPattern::PowerUser => 0,
    }
}

fn activity_cluster_size(pattern: ActivityPattern) -> usize {
    match pattern {
        ActivityPattern::Lurker => 1,
        ActivityPattern::Casual => 2,
        ActivityPattern::Regular => 3,
        ActivityPattern::Bursty => 6,
        ActivityPattern::PowerUser => 4,
    }
}

fn activity_cadence_days(pattern: ActivityPattern) -> usize {
    match pattern {
        ActivityPattern::Lurker => 21,
        ActivityPattern::Casual => 10,
        ActivityPattern::Regular => 5,
        ActivityPattern::Bursty => 12,
        ActivityPattern::PowerUser => 2,
    }
}

fn comment_reply_delay_minutes(
    config: &ForumScenarioConfig,
    entity_id: &str,
    persona: &Persona,
) -> usize {
    let upper_bound = match persona.activity_pattern {
        ActivityPattern::Lurker => 3 * MINUTES_PER_DAY,
        ActivityPattern::Casual => MINUTES_PER_DAY,
        ActivityPattern::Regular => 12 * 60,
        ActivityPattern::Bursty => 6 * 60,
        ActivityPattern::PowerUser => 2 * 60,
    };

    15 + deterministic_index(config, "comments", entity_id, "reply_delay", upper_bound)
}

fn minutes_from_timestamp(timestamp: &ModelTimestamp) -> usize {
    parse_timestamp_minutes(timestamp.as_str()).expect("generated timestamps should be parseable")
}

fn parse_timestamp_minutes(value: &str) -> Option<usize> {
    if value.len() != 20 || !value.ends_with('Z') {
        return None;
    }
    if value.get(4..5)? != "-"
        || value.get(7..8)? != "-"
        || value.get(10..11)? != "T"
        || value.get(13..14)? != ":"
        || value.get(16..17)? != ":"
        || value.get(17..19)? != "00"
    {
        return None;
    }

    let year = value.get(0..4)?.parse::<usize>().ok()?;
    let month = value.get(5..7)?.parse::<usize>().ok()?;
    let day = value.get(8..10)?.parse::<usize>().ok()?;
    let hour = value.get(11..13)?.parse::<usize>().ok()?;
    let minute = value.get(14..16)?.parse::<usize>().ok()?;

    Some(days_after_2026_01_01(year, month, day)? * MINUTES_PER_DAY + hour * 60 + minute)
}

fn days_after_2026_01_01(year: usize, month: usize, day: usize) -> Option<usize> {
    if year < 2026 || !(1..=12).contains(&month) || day == 0 {
        return None;
    }

    let mut days = 0;
    for candidate_year in 2026..year {
        days += if is_leap_year(candidate_year) {
            366
        } else {
            365
        };
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
    if day > month_lengths[month - 1] {
        return None;
    }

    days += month_lengths.iter().take(month - 1).sum::<usize>();
    Some(days + day - 1)
}

fn timestamp_from_minutes(minutes: usize) -> String {
    let days = minutes / MINUTES_PER_DAY;
    let minute_of_day = minutes % MINUTES_PER_DAY;
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

fn capitalize_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
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
        assert_eq!(dataset.personas.len(), dataset.users.len());
        assert_eq!(dataset.interests.len(), INTEREST_CATALOG.len());
        assert_eq!(dataset.communities.len(), 2);
        assert_eq!(dataset.posts.len(), 5);
        assert_eq!(dataset.comments.len(), 7);
        assert!(dataset.relationships.len() >= dataset.users.len());
        assert_eq!(
            dataset.activity_events.len(),
            dataset.communities.len()
                + dataset.users.len()
                + dataset.personas.len()
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
    fn activity_schedule_hourly_distribution_is_not_uniform() {
        let dataset =
            generate_forum_dataset(&forum_config_with("activity-hour-histogram", 400, 2_000, 1))
                .expect("dataset should build");
        let mut hourly_counts = [0usize; 24];

        for post in &dataset.posts {
            hourly_counts[timestamp_hour(&post.created_at)] += 1;
        }

        let max = hourly_counts.iter().copied().max().unwrap_or_default();
        let min = hourly_counts.iter().copied().min().unwrap_or_default();

        assert!(max > min * 2);
        assert!(hourly_counts.iter().filter(|&&count| count > 0).count() >= 12);
    }

    #[test]
    fn activity_schedule_is_deterministic() {
        let config = forum_config_with("activity-deterministic", 200, 400, 200);
        let first = generate_forum_dataset(&config).expect("first dataset should build");
        let second = generate_forum_dataset(&config).expect("second dataset should build");

        let first_post_times = first
            .posts
            .iter()
            .map(|post| post.created_at.clone())
            .collect::<Vec<_>>();
        let second_post_times = second
            .posts
            .iter()
            .map(|post| post.created_at.clone())
            .collect::<Vec<_>>();

        assert_eq!(first_post_times, second_post_times);
    }

    #[test]
    fn activity_schedule_varies_by_persona_pattern() {
        let config = forum_config_with("activity-persona-variation", 1, 1, 1);
        let user = test_user("user-temporal", "en-US", &[]);
        let mut early = affinity_persona("persona-early", SleepPhase::EarlyBird, 0.70, 0.1, 0.8);
        early.user_id = user.id.clone();
        early.activity_pattern = ActivityPattern::Regular;
        let mut night = affinity_persona("persona-night", SleepPhase::NightOwl, 0.70, 0.1, 0.8);
        night.user_id = user.id.clone();
        night.activity_pattern = ActivityPattern::Regular;
        let mut bursty = early.clone();
        bursty.activity_pattern = ActivityPattern::Bursty;

        let early_time =
            scheduled_post_timestamp(&config, 12, "post-temporal", &user, &early).unwrap();
        let night_time =
            scheduled_post_timestamp(&config, 12, "post-temporal", &user, &night).unwrap();
        let bursty_time =
            scheduled_post_timestamp(&config, 12, "post-temporal", &user, &bursty).unwrap();

        assert_ne!(timestamp_hour(&early_time), timestamp_hour(&night_time));
        assert_ne!(early_time, bursty_time);
    }

    #[test]
    fn generated_forum_records_have_representative_stable_values() {
        let dataset = generate_forum_dataset(&small_forum_config()).expect("dataset should build");

        assert_eq!(dataset.users[0].id.as_str(), "user-000001");
        assert_eq!(
            dataset.users[0].persona_id.as_ref().map(PersonaId::as_str),
            Some("persona-000001")
        );
        assert_eq!(dataset.users[0].username, "harper.gray0001");
        assert_eq!(dataset.users[0].display_name, "Harper Gray");
        assert_eq!(dataset.users[0].created_at.as_str(), "2026-05-07T11:49:00Z");
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
        assert_eq!(dataset.personas[0].id.as_str(), "persona-000001");
        assert_eq!(dataset.personas[0].user_id, dataset.users[0].id);
        assert!((0.0..=1.0).contains(&dataset.personas[0].technical_depth));
        assert!(dataset.activity_events.iter().any(|event| {
            event.actor_id.as_ref() == Some(&dataset.users[0].id)
                && event.kind == ActivityEventKind::PersonaCreated
                && event.object == ActivityObject::Persona(dataset.personas[0].id.clone())
        }));
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
    fn generated_population_includes_new_and_veteran_accounts() {
        let dataset = generate_forum_dataset(&forum_config_with("account-aging", 600, 100, 50))
            .expect("dataset should build");
        let ages = dataset
            .users
            .iter()
            .map(account_age_days_for_user)
            .collect::<Vec<_>>();

        assert!(ages.iter().any(|&age| age <= 45));
        assert!(ages.iter().any(|&age| age >= 300));
        assert!(dataset
            .users
            .iter()
            .any(|user| user.status.as_deref() == Some("veteran")));
        assert!(dataset
            .users
            .iter()
            .any(|user| user.status.as_deref() == Some("dormant")));
    }

    #[test]
    fn account_age_and_dormancy_influence_authored_activity() {
        let config = forum_config_with("account-aging-activity", 1_000, 3_000, 1);
        let first = generate_forum_dataset(&config).expect("first dataset should build");
        let second = generate_forum_dataset(&config).expect("second dataset should build");
        let mut post_counts = HashMap::<UserId, usize>::new();

        assert_eq!(
            first
                .users
                .iter()
                .map(|user| (&user.id, &user.created_at, &user.status))
                .collect::<Vec<_>>(),
            second
                .users
                .iter()
                .map(|user| (&user.id, &user.created_at, &user.status))
                .collect::<Vec<_>>()
        );

        for post in &first.posts {
            *post_counts.entry(post.author_id.clone()).or_insert(0) += 1;
        }

        let active_average = authored_average_for_status(&first, &post_counts, "active");
        let dormant_average = authored_average_for_status(&first, &post_counts, "dormant");

        assert!(active_average > dormant_average * 2.0);
    }

    #[test]
    fn generated_timestamps_are_causally_ordered() {
        let dataset = generate_forum_dataset(&forum_config_with("temporal-causal", 200, 500, 800))
            .expect("dataset should build");

        validate_forum_temporal_consistency(&dataset).expect("timestamps should be consistent");
    }

    #[test]
    fn activity_events_are_chronologically_ordered() {
        let dataset =
            generate_forum_dataset(&forum_config_with("temporal-activity-order", 120, 300, 300))
                .expect("dataset should build");
        let mut previous = None;

        for event in &dataset.activity_events {
            let minutes = parse_timestamp_minutes(event.occurred_at.as_str())
                .expect("activity timestamp should parse");
            if let Some(previous_minutes) = previous {
                assert!(minutes >= previous_minutes);
            }
            previous = Some(minutes);
        }
    }

    #[test]
    fn generated_timestamps_are_stable_iso_utc_values() {
        let dataset =
            generate_forum_dataset(&forum_config_with("temporal-stable-iso", 80, 120, 120))
                .expect("dataset should build");

        for timestamp in all_generated_timestamps(&dataset) {
            let minutes =
                parse_timestamp_minutes(timestamp.as_str()).expect("timestamp should parse");
            assert_eq!(timestamp.as_str(), timestamp_from_minutes(minutes));
        }
    }

    #[test]
    fn generated_relationships_and_activity_events_are_consistent() {
        let dataset = generate_forum_dataset(&small_forum_config()).expect("dataset should build");

        for relationship in &dataset.relationships {
            match relationship.kind {
                RelationshipKind::MemberOf => {
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
                RelationshipKind::Follows | RelationshipKind::Friend => {
                    let RelationshipEndpoint::User(source_id) = &relationship.source else {
                        panic!("social relationship source should be a user");
                    };
                    let RelationshipEndpoint::User(target_id) = &relationship.target else {
                        panic!("social relationship target should be a user");
                    };

                    assert_ne!(source_id, target_id);
                    assert!(dataset.users.iter().any(|user| &user.id == source_id));
                    assert!(dataset.users.iter().any(|user| &user.id == target_id));
                    assert!(dataset.activity_events.iter().any(|event| {
                        event.actor_id.as_ref() == Some(source_id)
                            && event.object == ActivityObject::Relationship(relationship.id.clone())
                    }));
                }
                RelationshipKind::Blocks | RelationshipKind::WorksAt => {
                    panic!("forum generation should not emit {:?}", relationship.kind);
                }
            }
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
    fn generated_interest_records_and_user_references_are_valid() {
        let dataset = generate_forum_dataset(&small_forum_config()).expect("dataset should build");
        let valid_ids = dataset
            .interests
            .iter()
            .map(|interest| interest.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let assigned_ids = dataset
            .users
            .iter()
            .flat_map(|user| user.interest_ids.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(dataset.interests.len(), INTEREST_CATALOG.len());
        assert!(dataset
            .interests
            .iter()
            .all(|interest| interest.category.is_some()));
        assert!(assigned_ids.len() >= 4);
        for user in &dataset.users {
            assert!((2..=5).contains(&user.interest_ids.len()));
            assert!(user.interest_ids.iter().all(|id| valid_ids.contains(id)));
        }
    }

    #[test]
    fn interest_scores_follow_visible_persona_correlations() {
        let technical = persona_for_scores(1.0, 0.1, 0.1, 0.1, 0.1);
        let social = persona_for_scores(0.1, 1.0, 1.0, 0.1, 0.1);
        let culture = persona_for_scores(0.1, 0.1, 0.1, 1.0, 1.0);
        let programming = catalog_interest("interest-programming");
        let linux = catalog_interest("interest-linux");
        let community = catalog_interest("interest-community");
        let events = catalog_interest("interest-events");
        let gaming = catalog_interest("interest-gaming");
        let memes = catalog_interest("interest-memes");

        assert!(
            persona_interest_score(&technical, programming)
                > persona_interest_score(&social, programming)
        );
        assert!(
            persona_interest_score(&technical, linux) > persona_interest_score(&culture, linux)
        );
        assert!(
            persona_interest_score(&social, community)
                > persona_interest_score(&technical, community)
        );
        assert!(persona_interest_score(&social, events) > persona_interest_score(&culture, events));
        assert!(persona_interest_score(&culture, gaming) > persona_interest_score(&social, gaming));
        assert!(
            persona_interest_score(&culture, memes) > persona_interest_score(&technical, memes)
        );
    }

    #[test]
    fn matching_interests_bias_primary_community_membership() {
        let config = ForumScenarioConfig {
            seed: "interest-community-bias".to_string(),
            population: PopulationConfig { users: 20 },
            communities: vec![
                "programming".to_string(),
                "gaming".to_string(),
                "linux".to_string(),
                "community".to_string(),
            ],
            content: ContentConfig {
                posts: 12,
                comments: 12,
            },
            output_format: OutputFormat::Jsonl,
        };
        let dataset = generate_forum_dataset(&config).expect("dataset should build");

        for user in &dataset.users {
            let Some(first_community_id) = user.community_ids.first() else {
                panic!("generated user should have at least one community")
            };
            let matching_interest = user.interest_ids.iter().find_map(|interest_id| {
                interest_id
                    .as_str()
                    .strip_prefix("interest-")
                    .filter(|slug| config.communities.iter().any(|community| community == slug))
            });
            if let Some(slug) = matching_interest {
                assert!(
                    first_community_id.as_str().contains(slug),
                    "{} should select a primary community matching {}",
                    user.id,
                    slug
                );
            }
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

    #[test]
    fn generated_bios_are_deterministic_non_empty_and_persona_aware() {
        let config = small_forum_config();
        let first = generate_forum_dataset(&config).expect("first dataset should build");
        let second = generate_forum_dataset(&config).expect("second dataset should build");

        for (first_user, second_user) in first.users.iter().zip(&second.users) {
            let bio = first_user.bio.as_deref().expect("bio should exist");

            assert_eq!(first_user.bio, second_user.bio);
            assert!(!bio.trim().is_empty());
            assert!(!bio.contains("practical forum discussions"));
        }
    }

    #[test]
    fn generated_bios_change_with_seed() {
        let mut first_config = small_forum_config();
        first_config.seed = "bio-seed-a".to_string();
        let mut second_config = small_forum_config();
        second_config.seed = "bio-seed-b".to_string();
        let first = generate_forum_dataset(&first_config).expect("first dataset should build");
        let second = generate_forum_dataset(&second_config).expect("second dataset should build");

        assert_ne!(first.users[0].bio, second.users[0].bio);
    }

    #[test]
    fn generated_bios_reference_only_assigned_interests() {
        let dataset = generate_forum_dataset(&small_forum_config()).expect("dataset should build");

        for user in &dataset.users {
            let bio = user.bio.as_deref().expect("bio should exist");
            for interest in &dataset.interests {
                if bio.contains(&interest.label) {
                    assert!(
                        user.interest_ids.contains(&interest.id),
                        "{} bio referenced unassigned interest {}: {}",
                        user.id,
                        interest.label,
                        bio
                    );
                }
            }
        }
    }

    #[test]
    fn generated_bios_have_low_duplicate_rate_at_10k_users() {
        let config = ForumScenarioConfig {
            seed: "bio-duplicate-smoke".to_string(),
            population: PopulationConfig { users: 10_000 },
            communities: vec![
                "programming".to_string(),
                "gaming".to_string(),
                "linux".to_string(),
                "photography".to_string(),
            ],
            content: ContentConfig {
                posts: 1,
                comments: 1,
            },
            output_format: OutputFormat::Jsonl,
        };
        let dataset = generate_forum_dataset(&config).expect("dataset should build");
        let unique_bios = dataset
            .users
            .iter()
            .filter_map(|user| user.bio.as_deref())
            .collect::<std::collections::BTreeSet<_>>();

        assert!(
            unique_bios.len() >= 1_000,
            "expected at least 1000 unique bios, got {}",
            unique_bios.len()
        );
    }

    #[test]
    fn affinity_scores_are_bounded_deterministic_and_order_independent() {
        let config = small_forum_config();
        let left = test_user(
            "user-left",
            "en-US",
            &["interest-programming", "interest-linux"],
        );
        let right = test_user(
            "user-right",
            "en-GB",
            &["interest-programming", "interest-gaming"],
        );
        let left_persona = affinity_persona("persona-left", SleepPhase::Daytime, 0.45, 0.10, 0.75);
        let right_persona =
            affinity_persona("persona-right", SleepPhase::Daytime, 0.50, 0.12, 0.70);

        let score = affinity_score(&config, &left, &left_persona, &right, &right_persona);
        let reverse = affinity_score(&config, &right, &right_persona, &left, &left_persona);

        assert!((0.0..=1.0).contains(&score));
        assert_eq!(score, reverse);
        assert_eq!(
            score,
            affinity_score(&config, &left, &left_persona, &right, &right_persona)
        );
    }

    #[test]
    fn affinity_components_track_interests_locale_personality_and_activity() {
        let technical = test_user(
            "user-technical",
            "en-US",
            &["interest-programming", "interest-linux", "interest-tools"],
        );
        let similar = test_user(
            "user-similar",
            "en-CA",
            &["interest-programming", "interest-linux", "interest-gaming"],
        );
        let distant = test_user(
            "user-distant",
            "fr-FR",
            &["interest-photography", "interest-memes"],
        );
        let calm_daytime = affinity_persona("persona-calm", SleepPhase::Daytime, 0.40, 0.05, 0.80);
        let similar_daytime =
            affinity_persona("persona-similar", SleepPhase::Daytime, 0.42, 0.08, 0.78);
        let combative_night =
            affinity_persona("persona-combative", SleepPhase::NightOwl, 0.95, 0.90, 0.10);

        assert!(
            shared_interest_score(&technical, &similar)
                > shared_interest_score(&technical, &distant)
        );
        assert_eq!(locale_similarity(&technical.locale, &technical.locale), 1.0);
        assert!(
            locale_similarity(&technical.locale, &similar.locale)
                > locale_similarity(&technical.locale, &distant.locale)
        );
        assert!(
            personality_compatibility(&calm_daytime, &similar_daytime)
                > personality_compatibility(&calm_daytime, &combative_night)
        );
        assert!(
            activity_overlap(&calm_daytime, &similar_daytime)
                > activity_overlap(&calm_daytime, &combative_night)
        );
    }

    #[test]
    fn social_relationships_are_valid_unique_and_canonical() {
        let dataset = generate_forum_dataset(&forum_config_with("social-validity", 80, 40, 20))
            .expect("dataset should build");
        let mut keys = HashSet::new();
        let mut friend_pairs = std::collections::BTreeSet::new();
        let user_ids = dataset
            .users
            .iter()
            .map(|user| user.id.to_string())
            .collect::<std::collections::BTreeSet<_>>();

        for (index, relationship) in dataset.relationships.iter().enumerate() {
            assert_eq!(
                relationship.id.as_str(),
                format!("relationship-{:06}", index + 1)
            );

            if matches!(
                relationship.kind,
                RelationshipKind::Follows | RelationshipKind::Friend
            ) {
                let RelationshipEndpoint::User(source_id) = &relationship.source else {
                    panic!("social source should be a user");
                };
                let RelationshipEndpoint::User(target_id) = &relationship.target else {
                    panic!("social target should be a user");
                };
                let source = source_id.to_string();
                let target = target_id.to_string();

                assert_ne!(source, target);
                assert!(user_ids.contains(&source));
                assert!(user_ids.contains(&target));
                assert!(keys.insert((relationship.kind, source.clone(), target.clone())));
                if relationship.kind == RelationshipKind::Friend {
                    assert!(source < target);
                    friend_pairs.insert((source, target));
                }
            }
        }

        for relationship in &dataset.relationships {
            if relationship.kind == RelationshipKind::Follows {
                let RelationshipEndpoint::User(source_id) = &relationship.source else {
                    unreachable!("checked above");
                };
                let RelationshipEndpoint::User(target_id) = &relationship.target else {
                    unreachable!("checked above");
                };
                let pair = if source_id.as_str() <= target_id.as_str() {
                    (source_id.to_string(), target_id.to_string())
                } else {
                    (target_id.to_string(), source_id.to_string())
                };

                assert!(!friend_pairs.contains(&pair));
            }
        }
    }

    #[test]
    fn social_relationship_generation_is_deterministic_and_seed_sensitive() {
        let first = generate_forum_dataset(&forum_config_with("social-seed-a", 64, 20, 10))
            .expect("first dataset should build");
        let second = generate_forum_dataset(&forum_config_with("social-seed-a", 64, 20, 10))
            .expect("second dataset should build");
        let third = generate_forum_dataset(&forum_config_with("social-seed-b", 64, 20, 10))
            .expect("third dataset should build");

        assert_eq!(social_triples(&first), social_triples(&second));
        assert_ne!(social_triples(&first), social_triples(&third));
    }

    #[test]
    fn social_relationship_generation_handles_tiny_populations() {
        let single = generate_forum_dataset(&forum_config_with("single-social", 1, 1, 1))
            .expect("single-user dataset should build");
        let pair = generate_forum_dataset(&forum_config_with("pair-social", 2, 1, 1))
            .expect("two-user dataset should build");

        assert!(social_relationships(&single).is_empty());
        assert!((1..=2).contains(&social_relationships(&pair).len()));
        for relationship in social_relationships(&pair) {
            let RelationshipEndpoint::User(source_id) = &relationship.source else {
                panic!("social source should be a user");
            };
            let RelationshipEndpoint::User(target_id) = &relationship.target else {
                panic!("social target should be a user");
            };
            assert_ne!(source_id, target_id);
        }
    }

    #[test]
    fn activity_patterns_follow_realistic_weighted_distribution() {
        let config = forum_config_with("activity-distribution", 10_000, 1, 1);
        let mut counts = HashMap::new();

        for index in 0..10_000 {
            let id = format!("persona-{:06}", index + 1);
            *counts
                .entry(activity_pattern_for(&config, &id))
                .or_insert(0usize) += 1;
        }

        assert_count_near(&counts, ActivityPattern::Lurker, 4_500, 500);
        assert_count_near(&counts, ActivityPattern::Casual, 3_000, 400);
        assert_count_near(&counts, ActivityPattern::Regular, 1_700, 300);
        assert_count_near(&counts, ActivityPattern::Bursty, 600, 180);
        assert_count_near(&counts, ActivityPattern::PowerUser, 200, 100);
    }

    #[test]
    fn authored_content_is_skewed_toward_more_active_roles() {
        let dataset = generate_forum_dataset(&forum_config_with("author-skew", 2_000, 2_000, 1))
            .expect("dataset should build");
        let pattern_by_user = dataset
            .personas
            .iter()
            .map(|persona| (persona.user_id.clone(), persona.activity_pattern))
            .collect::<HashMap<_, _>>();
        let mut user_counts = HashMap::new();
        let mut post_counts = HashMap::new();

        for persona in &dataset.personas {
            *user_counts
                .entry(persona.activity_pattern)
                .or_insert(0usize) += 1;
        }
        for post in &dataset.posts {
            let pattern = pattern_by_user
                .get(&post.author_id)
                .expect("post author should have a persona");
            *post_counts.entry(*pattern).or_insert(0usize) += 1;
        }

        let lurker_average = authored_average(&user_counts, &post_counts, ActivityPattern::Lurker);
        let regular_average =
            authored_average(&user_counts, &post_counts, ActivityPattern::Regular);
        let power_average =
            authored_average(&user_counts, &post_counts, ActivityPattern::PowerUser);

        assert!(regular_average > lurker_average);
        assert!(power_average > regular_average);
    }

    #[test]
    fn generated_social_graph_has_realistic_large_scale_metrics() {
        let dataset = generate_forum_dataset(&forum_config_with("social-metrics", 1_000, 100, 100))
            .expect("dataset should build");
        let social = social_relationships(&dataset);
        let mut degree = vec![0usize; dataset.users.len()];
        let user_index = dataset
            .users
            .iter()
            .enumerate()
            .map(|(index, user)| (user.id.clone(), index))
            .collect::<HashMap<_, _>>();
        let mut union_find = UnionFind::new(dataset.users.len());

        for relationship in &social {
            let RelationshipEndpoint::User(source_id) = &relationship.source else {
                panic!("social source should be a user");
            };
            let RelationshipEndpoint::User(target_id) = &relationship.target else {
                panic!("social target should be a user");
            };
            let source = user_index[source_id];
            let target = user_index[target_id];

            degree[source] += 1;
            degree[target] += 1;
            union_find.union(source, target);
        }

        let users_with_social_edges = degree.iter().filter(|&&count| count > 0).count();
        let largest_component = union_find.largest_component_size();
        let max_degree = degree.iter().copied().max().unwrap_or_default();
        let average_degree = degree.iter().sum::<usize>() as f32 / degree.len() as f32;

        assert!(users_with_social_edges >= dataset.users.len() * 80 / 100);
        assert!(largest_component >= dataset.users.len() * 60 / 100);
        assert!(social.len() <= dataset.users.len() * 50);
        assert!(max_degree as f32 > average_degree * 2.0);
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

    fn forum_config_with(
        seed: &str,
        users: usize,
        posts: usize,
        comments: usize,
    ) -> ForumScenarioConfig {
        ForumScenarioConfig {
            seed: seed.to_string(),
            population: PopulationConfig { users },
            communities: vec![
                "general".to_string(),
                "support".to_string(),
                "programming".to_string(),
                "gaming".to_string(),
                "linux".to_string(),
            ],
            content: ContentConfig { posts, comments },
            output_format: OutputFormat::Jsonl,
        }
    }

    fn social_relationships(dataset: &ForumDataset) -> Vec<&Relationship> {
        dataset
            .relationships
            .iter()
            .filter(|relationship| {
                matches!(
                    relationship.kind,
                    RelationshipKind::Follows | RelationshipKind::Friend
                )
            })
            .collect()
    }

    fn social_triples(dataset: &ForumDataset) -> Vec<(RelationshipKind, String, String)> {
        social_relationships(dataset)
            .into_iter()
            .map(|relationship| {
                let RelationshipEndpoint::User(source_id) = &relationship.source else {
                    panic!("social source should be a user");
                };
                let RelationshipEndpoint::User(target_id) = &relationship.target else {
                    panic!("social target should be a user");
                };

                (
                    relationship.kind,
                    source_id.to_string(),
                    target_id.to_string(),
                )
            })
            .collect()
    }

    fn test_user(id: &str, locale: &str, interests: &[&str]) -> User {
        let mut user = User::new(
            UserId::new(id).expect("user ID should be valid"),
            id,
            id,
            Locale::new(locale).expect("locale should be valid"),
            ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid"),
        )
        .expect("user should be valid");
        user.interest_ids = interests
            .iter()
            .map(|interest| InterestId::new(*interest).expect("interest ID should be valid"))
            .collect();
        user
    }

    fn affinity_persona(
        id: &str,
        sleep_phase: SleepPhase,
        posting_frequency: f32,
        controversy_affinity: f32,
        agreeableness: f32,
    ) -> Persona {
        let mut persona = Persona::new(
            PersonaId::new(id).expect("persona ID should be valid"),
            UserId::new(format!("user-for-{id}")).expect("user ID should be valid"),
            "affinity test",
            ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid"),
        )
        .expect("persona should be valid");
        persona.openness = 0.60;
        persona.extroversion = 0.55;
        persona.conscientiousness = 0.60;
        persona.agreeableness = agreeableness;
        persona.neuroticism = 0.30;
        persona.posting_frequency = posting_frequency;
        persona.controversy_affinity = controversy_affinity;
        persona.humor_affinity = 0.45;
        persona.technical_depth = 0.65;
        persona.meme_affinity = 0.35;
        persona.sleep_phase = sleep_phase;
        persona
    }

    fn assert_count_near(
        counts: &HashMap<ActivityPattern, usize>,
        pattern: ActivityPattern,
        expected: usize,
        tolerance: usize,
    ) {
        let actual = *counts.get(&pattern).unwrap_or(&0);
        assert!(
            actual.abs_diff(expected) <= tolerance,
            "expected {:?} count near {}, got {}",
            pattern,
            expected,
            actual
        );
    }

    fn authored_average(
        user_counts: &HashMap<ActivityPattern, usize>,
        post_counts: &HashMap<ActivityPattern, usize>,
        pattern: ActivityPattern,
    ) -> f32 {
        let users = *user_counts.get(&pattern).unwrap_or(&0);
        let posts = *post_counts.get(&pattern).unwrap_or(&0);

        assert!(users > 0, "expected generated users for {:?}", pattern);
        posts as f32 / users as f32
    }

    fn authored_average_for_status(
        dataset: &ForumDataset,
        post_counts: &HashMap<UserId, usize>,
        status: &str,
    ) -> f32 {
        let users = dataset
            .users
            .iter()
            .filter(|user| user.status.as_deref() == Some(status))
            .collect::<Vec<_>>();
        let posts = users
            .iter()
            .map(|user| post_counts.get(&user.id).copied().unwrap_or_default())
            .sum::<usize>();

        assert!(!users.is_empty(), "expected generated {status} users");
        posts as f32 / users.len() as f32
    }

    fn all_generated_timestamps(dataset: &ForumDataset) -> Vec<&ModelTimestamp> {
        dataset
            .users
            .iter()
            .map(|user| &user.created_at)
            .chain(dataset.personas.iter().map(|persona| &persona.created_at))
            .chain(
                dataset
                    .communities
                    .iter()
                    .map(|community| &community.created_at),
            )
            .chain(dataset.posts.iter().map(|post| &post.created_at))
            .chain(dataset.comments.iter().map(|comment| &comment.created_at))
            .chain(
                dataset
                    .relationships
                    .iter()
                    .map(|relationship| &relationship.created_at),
            )
            .chain(
                dataset
                    .activity_events
                    .iter()
                    .map(|event| &event.occurred_at),
            )
            .collect()
    }

    fn timestamp_hour(timestamp: &ModelTimestamp) -> usize {
        timestamp.as_str()[11..13]
            .parse()
            .expect("timestamp hour should parse")
    }

    struct UnionFind {
        parents: Vec<usize>,
        sizes: Vec<usize>,
    }

    impl UnionFind {
        fn new(len: usize) -> Self {
            Self {
                parents: (0..len).collect(),
                sizes: vec![1; len],
            }
        }

        fn find(&mut self, index: usize) -> usize {
            if self.parents[index] != index {
                self.parents[index] = self.find(self.parents[index]);
            }

            self.parents[index]
        }

        fn union(&mut self, left: usize, right: usize) {
            let mut left_root = self.find(left);
            let mut right_root = self.find(right);
            if left_root == right_root {
                return;
            }
            if self.sizes[left_root] < self.sizes[right_root] {
                std::mem::swap(&mut left_root, &mut right_root);
            }
            self.parents[right_root] = left_root;
            self.sizes[left_root] += self.sizes[right_root];
        }

        fn largest_component_size(&mut self) -> usize {
            let mut counts = std::collections::BTreeMap::new();
            for index in 0..self.parents.len() {
                let root = self.find(index);
                *counts.entry(root).or_insert(0usize) += 1;
            }

            counts.values().copied().max().unwrap_or_default()
        }
    }

    fn catalog_interest(id: &str) -> &'static InterestSpec {
        INTEREST_CATALOG
            .iter()
            .find(|spec| spec.id == id)
            .expect("catalog interest should exist")
    }

    fn persona_for_scores(
        technical_depth: f32,
        extroversion: f32,
        posting_frequency: f32,
        meme_affinity: f32,
        humor_affinity: f32,
    ) -> Persona {
        let mut persona = Persona::new(
            PersonaId::new("persona-score-test").expect("valid"),
            UserId::new("user-score-test").expect("valid"),
            "score test",
            ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid"),
        )
        .expect("persona should be valid");
        persona.technical_depth = technical_depth;
        persona.extroversion = extroversion;
        persona.posting_frequency = posting_frequency;
        persona.meme_affinity = meme_affinity;
        persona.humor_affinity = humor_affinity;
        persona
    }
}
