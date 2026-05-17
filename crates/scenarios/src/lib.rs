use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    let relationships = generate_memberships(
        config,
        &mut users,
        &personas,
        &communities,
        &mut activity_events,
    )?;
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
const ACTIVITY_PATTERNS: [ActivityPattern; 5] = [
    ActivityPattern::Lurker,
    ActivityPattern::Casual,
    ActivityPattern::Regular,
    ActivityPattern::Bursty,
    ActivityPattern::PowerUser,
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
            let timestamp = timestamp_for("persona", index)?;
            let verbosity = *choose(config, "personas", entity_id, "verbosity", &VERBOSITIES);
            let sleep_phase = *choose(config, "personas", entity_id, "sleep_phase", &SLEEP_PHASES);
            let activity_pattern = *choose(
                config,
                "personas",
                entity_id,
                "activity_pattern",
                &ACTIVITY_PATTERNS,
            );
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
    personas: &[Persona],
    members: &[usize],
    entity_id: &str,
) -> &'a User {
    let index = choose_weighted_member_index(config, personas, members, "posts", entity_id);

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
        choose_weighted_member_index(config, personas, members, "comments", entity_id);
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
    personas: &[Persona],
    members: &[usize],
    namespace: &str,
    entity_id: &str,
) -> usize {
    let candidate_count = members.len().min(3);
    let mut best =
        members[deterministic_index(config, namespace, entity_id, "author", members.len())];
    let mut best_score = author_activity_score(&personas[best])
        + author_tie_breaker(config, namespace, entity_id, personas[best].id.as_str());

    for candidate_index in 0..candidate_count {
        let field = match candidate_index {
            0 => "author_candidate_0",
            1 => "author_candidate_1",
            _ => "author_candidate_2",
        };
        let offset = deterministic_index(config, namespace, entity_id, field, members.len());
        let user_index = members[offset];
        let score = author_activity_score(&personas[user_index])
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

fn author_activity_score(persona: &Persona) -> f32 {
    let activity = match persona.activity_pattern {
        ActivityPattern::Lurker => 0.05,
        ActivityPattern::Casual => 0.25,
        ActivityPattern::Regular => 0.55,
        ActivityPattern::Bursty => 0.70,
        ActivityPattern::PowerUser => 0.90,
    };

    persona.posting_frequency * 0.50 + persona.extroversion * 0.25 + activity * 0.25
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
    fn generated_forum_records_have_representative_stable_values() {
        let dataset = generate_forum_dataset(&small_forum_config()).expect("dataset should build");

        assert_eq!(dataset.users[0].id.as_str(), "user-000001");
        assert_eq!(
            dataset.users[0].persona_id.as_ref().map(PersonaId::as_str),
            Some("persona-000001")
        );
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
