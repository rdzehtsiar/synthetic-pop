use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
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
        &communities,
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
        .filter_map(|user| {
            parse_timestamp_minutes(user.created_at.as_str())
                .map(|minutes| (user.id.clone(), minutes))
        })
        .collect::<HashMap<_, _>>();
    let user_by_id = dataset
        .users
        .iter()
        .map(|user| (user.id.clone(), user))
        .collect::<HashMap<_, _>>();
    let persona_by_id = dataset
        .personas
        .iter()
        .map(|persona| (persona.id.clone(), persona))
        .collect::<HashMap<_, _>>();
    let community_by_id = dataset
        .communities
        .iter()
        .map(|community| (community.id.clone(), community))
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
    let activity_context = ActivityValidationContext {
        user_created_at: &user_created_at,
        user_by_id: &user_by_id,
        persona_by_id: &persona_by_id,
        community_by_id: &community_by_id,
        post_by_id: &post_by_id,
        comment_by_id: &comment_by_id,
        relationship_by_id: &relationship_by_id,
    };
    validate_activity_events(dataset, &activity_context, &mut errors);

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
const VERBOSITIES: [Verbosity; 3] = [Verbosity::Terse, Verbosity::Balanced, Verbosity::Detailed];
const SLEEP_PHASES: [SleepPhase; 4] = [
    SleepPhase::EarlyBird,
    SleepPhase::Daytime,
    SleepPhase::NightOwl,
    SleepPhase::Irregular,
];
const INTEREST_IDS: [&str; 15] = [
    "interest-programming",
    "interest-linux",
    "interest-systems",
    "interest-tooling",
    "interest-community",
    "interest-events",
    "interest-collaboration",
    "interest-gaming",
    "interest-memes",
    "interest-culture",
    "interest-photography",
    "interest-writing",
    "interest-design",
    "interest-productivity",
    "interest-learning",
];
const INTEREST_LABELS: [&str; 15] = [
    "Programming",
    "Linux",
    "Systems",
    "Tooling",
    "Community",
    "Events",
    "Collaboration",
    "Gaming",
    "Memes",
    "Culture",
    "Photography",
    "Writing",
    "Design",
    "Productivity",
    "Learning",
];
const INTEREST_CATEGORIES: [&str; 15] = [
    "technical",
    "technical",
    "technical",
    "technical",
    "social",
    "social",
    "social",
    "culture",
    "culture",
    "culture",
    "creative",
    "creative",
    "creative",
    "practical",
    "practical",
];
const INTEREST_WEIGHTS: [(f32, f32, f32); 15] = [
    (1.00, 0.10, 0.00),
    (0.95, 0.00, 0.00),
    (0.90, 0.00, 0.00),
    (0.85, 0.05, 0.00),
    (0.00, 1.00, 0.10),
    (0.00, 0.90, 0.05),
    (0.15, 0.85, 0.00),
    (0.05, 0.20, 0.95),
    (0.00, 0.10, 1.00),
    (0.00, 0.25, 0.80),
    (0.10, 0.15, 0.20),
    (0.05, 0.25, 0.10),
    (0.20, 0.20, 0.15),
    (0.25, 0.15, 0.00),
    (0.35, 0.20, 0.00),
];
const INTEREST_CATALOG_LEN: usize = INTEREST_IDS.len();
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticIntent {
    Frustration,
    Excitement,
    TechnicalAdvice,
    Agreement,
    Disagreement,
    Question,
    Clarification,
    Humor,
    Sarcasm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UtteranceKind {
    Post,
    Comment,
}

#[derive(Debug, Clone)]
struct LanguageDraft {
    intent: SemanticIntent,
    kind: UtteranceKind,
    author_name: String,
    community_name: String,
    topic: String,
    referenced_post_title: Option<String>,
    referenced_post_topic: Option<String>,
}

struct StyleProfile {
    verbosity: Verbosity,
    detail_bias: usize,
    abbreviation_bias: usize,
    humor_bias: usize,
    sarcasm_bias: usize,
    typo_bias: usize,
    emoji_bias: usize,
    question_bias: bool,
}

const POST_FRUSTRATION_TEMPLATES: &[&str] = &[
    "{author} is frustrated by recurring blockers around {topic} in {community}.",
    "{author} says the {topic} path in {community} keeps getting messy.",
    "{author} is not happy with how {topic} keeps getting pushed around in {community}.",
];
const POST_EXCITEMENT_TEMPLATES: &[&str] = &[
    "{author} is excited to launch a fresh discussion on {topic} in {community}.",
    "{author} sees a strong turn forward for {topic} in {community}.",
    "A lot of energy is building around {topic} in {community}, according to {author}.",
];
const POST_TECHNICAL_ADVICE_TEMPLATES: &[&str] = &[
    "{author} suggests a technical route for {topic} in {community}: clarify assumptions, then measure.",
    "{author} shares implementation advice for {topic} in {community} with repeatable checks.",
    "{author} recommends a practical {topic} pattern for {community} with explicit steps.",
];
const POST_AGREEMENT_TEMPLATES: &[&str] = &[
    "{author} agrees that progress on {topic} in {community} is worth moving forward.",
    "{author} is aligned that {topic} needs a stable baseline in {community}.",
    "{author} concurs and thinks {topic} can be done cleanly in {community}.",
];
const POST_DISAGREEMENT_TEMPLATES: &[&str] = &[
    "{author} disagrees with the current framing of {topic} in {community}.",
    "{author} questions whether {topic} belongs in {community} this way.",
    "{author} is pushing back on the direction for {topic} in {community}.",
];
const POST_QUESTION_TEMPLATES: &[&str] = &[
    "{author} asks how {topic} should behave first inside {community}.",
    "{author} wants to confirm what success for {topic} looks like in {community}.",
    "{author} is asking whether {topic} should be prioritized for {community}.",
];
const POST_CLARIFICATION_TEMPLATES: &[&str] = &[
    "{author} is asking for clarification on {topic} expectations in {community}.",
    "{author} needs one clear boundary for {topic} in {community} before proceeding.",
    "{author} requests concrete definition for the {topic} scope in {community}.",
];
const POST_HUMOR_TEMPLATES: &[&str] = &[
    "{author} says {topic} in {community} feels like a surprise weekend project.",
    "{author} jokes that {topic} in {community} is the fastest route to a spirited thread.",
    "{author} adds a playful take: {topic} in {community} always finds a way to surprise us.",
];
const POST_SARCASTIC_TEMPLATES: &[&str] = &[
    "{author} notes that {topic} in {community} is obviously perfectly straightforward.",
    "{author} dryly observes that {topic} in {community} somehow always improves at scale.",
    "{author} points out that {topic} in {community} may be easier after the third try.",
];

const COMMENT_FRUSTRATION_TEMPLATES: &[&str] = &[
    "I keep running into friction on {topic} here; I'd like a simpler path.",
    "I'm still seeing edge cases with {post_topic} and this feels brittle.",
    "I'm not fully convinced this {topic} framing is complete yet.",
];
const COMMENT_EXCITEMENT_TEMPLATES: &[&str] = &[
    "This is exciting; {topic} here looks like a good direction.",
    "I like this push on {topic}; the result is promising.",
    "This should help a lot, especially around {post_topic}.",
];
const COMMENT_TECHNICAL_ADVICE_TEMPLATES: &[&str] = &[
    "I'd suggest tracing ownership and contract boundaries before changing {topic}.",
    "The safer route is to add a test matrix for {topic} first.",
    "One practical approach is to document assumptions and then automate {post_topic}.",
];
const COMMENT_AGREEMENT_TEMPLATES: &[&str] = &[
    "Agreed, this is a solid read on {topic}.",
    "You're right that this framing of {topic} is useful.",
    "Totally, {topic} here is the right first step.",
];
const COMMENT_DISAGREEMENT_TEMPLATES: &[&str] = &[
    "I see a different path for {topic}; this might overcomplicate things.",
    "I'm not sold on this version of {topic} yet.",
    "I can't quite get behind this {topic} direction right now.",
];
const COMMENT_QUESTION_TEMPLATES: &[&str] = &[
    "Could you clarify {topic} by sharing what changed first?",
    "Could this work for {post_topic}, or is there a hidden constraint?",
    "Do you expect this to hold across all {topic} contexts?",
];
const COMMENT_CLARIFICATION_TEMPLATES: &[&str] = &[
    "A quick clarification on {topic} would help reduce ambiguity.",
    "Can you define what {topic} success looks like after rollout?",
    "What specifically does {topic} guarantee in {post_topic}?",
];
const COMMENT_HUMOR_TEMPLATES: &[&str] = &[
    "Haha, this {topic} story has the right amount of chaos.",
    "That's the kind of {topic} energy people remember and talk about.",
    "I'm here for this {topic} momentum and the accidental comedy in it.",
];
const COMMENT_SARCASTIC_TEMPLATES: &[&str] = &[
    "Sure, this {topic} plan is exactly what everyone asked for.",
    "If this lands exactly as written, we can close that can of worms.",
    "Brilliant move—let's see how long before the first rollback.",
];

const OPENER_TEMPLATES: &[&str] = &["", "Quick thought:", "FYI:", "Note:", "In my view:"];
const CLOSER_TEMPLATES: &[&str] = &[
    "",
    "Thoughts?",
    "Would be good to confirm.",
    "What do others think?",
    "Totally open to adjustments.",
];

const AGREEMENT_VARIANTS: &[&str] = &[
    "I can get behind that.",
    "That direction reads correctly.",
    "I agree with the core idea.",
];
const ADVICE_VARIANTS: &[&str] = &[
    "I'd document assumptions first.",
    "Try a small dry run.",
    "A staged rollout usually helps.",
];
const CONCERN_VARIANTS: &[&str] = &[
    "Still, there is a tradeoff to call out.",
    "The main risk is context drift.",
    "The hidden cost might be operational overhead.",
];
const SYNONYM_GROUPS: &[&[&str]] = &[
    &["discussion", "thread", "conversation"],
    &["issue", "problem", "concern"],
    &["community", "group", "space"],
    &["technical", "practical", "engineering"],
];
const ABBREVIATION_PAIRS: &[(&str, &str)] = &[
    ("because", "bc"),
    ("before", "b4"),
    ("without", "w/o"),
    ("between", "btwn"),
    ("message", "msg"),
    ("example", "ex"),
];
const TYPO_PAIRS: &[(&str, &str)] = &[
    ("definitely", "definately"),
    ("separate", "seperate"),
    ("their", "thier"),
    ("community", "communtiy"),
    ("documentation", "documenation"),
];
const HUMOR_VARIANTS: &[&str] = &[
    "haha.",
    "That energy is a real win.",
    "This one made my day.",
    "Love this direction.",
];
const SARCASTIC_VARIANTS: &[&str] = &[
    "genuinely, this reads like a long game.",
    "classic.",
    "as always.",
];
const STRUCTURAL_REWRITES: &[&str] = &[
    "{body}",
    "{body}; {connector}",
    "{connector}: {body}",
    "{body}, and {connector}.",
    "{connector} in that thread: {body}",
    "{body}; {connector}",
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

fn interest_spec(index: usize) -> InterestSpec {
    let (technical_weight, social_weight, culture_weight) = INTEREST_WEIGHTS[index];

    InterestSpec::new(
        INTEREST_IDS[index],
        INTEREST_LABELS[index],
        INTEREST_CATEGORIES[index],
        technical_weight,
        social_weight,
        culture_weight,
    )
}

fn interest_specs() -> impl Iterator<Item = InterestSpec> {
    (0..INTEREST_CATALOG_LEN).map(interest_spec)
}

#[derive(Debug, Clone, Copy)]
struct PersonaAttributes {
    verbosity: Verbosity,
    sleep_phase: SleepPhase,
    activity_pattern: ActivityPattern,
    openness: f32,
    extroversion: f32,
    conscientiousness: f32,
    agreeableness: f32,
    neuroticism: f32,
    posting_frequency: f32,
    controversy_affinity: f32,
    humor_affinity: f32,
    technical_depth: f32,
    meme_affinity: f32,
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
                Locale::new(locale)?,
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
            let attributes = persona_attributes(config, entity_id);
            let persona = build_persona(id.clone(), user, timestamp.clone(), attributes)?;
            user.persona_id = Some(id.clone());
            push_persona_created_activity(activity_events, user, id, timestamp)?;
            Ok(persona)
        })
        .collect()
}

fn persona_attributes(config: &ForumScenarioConfig, entity_id: &str) -> PersonaAttributes {
    PersonaAttributes {
        verbosity: choose(config, "personas", entity_id, "verbosity", &VERBOSITIES),
        sleep_phase: choose(config, "personas", entity_id, "sleep_phase", &SLEEP_PHASES),
        activity_pattern: activity_pattern_for(config, entity_id),
        openness: persona_score(config, entity_id, "openness"),
        extroversion: persona_score(config, entity_id, "extroversion"),
        conscientiousness: persona_score(config, entity_id, "conscientiousness"),
        agreeableness: persona_score(config, entity_id, "agreeableness"),
        neuroticism: persona_score(config, entity_id, "neuroticism"),
        posting_frequency: persona_score(config, entity_id, "posting_frequency"),
        controversy_affinity: persona_score(config, entity_id, "controversy_affinity"),
        humor_affinity: persona_score(config, entity_id, "humor_affinity"),
        technical_depth: persona_score(config, entity_id, "technical_depth"),
        meme_affinity: persona_score(config, entity_id, "meme_affinity"),
    }
}

fn build_persona(
    id: PersonaId,
    user: &User,
    timestamp: ModelTimestamp,
    attributes: PersonaAttributes,
) -> Result<Persona, ModelValidationError> {
    let mut persona = Persona::new(
        id,
        user.id.clone(),
        persona_summary(
            attributes.verbosity,
            attributes.activity_pattern,
            attributes.technical_depth,
            attributes.humor_affinity,
        ),
        timestamp,
    )?;

    apply_persona_attributes(&mut persona, attributes);
    Ok(persona)
}

fn apply_persona_attributes(persona: &mut Persona, attributes: PersonaAttributes) {
    persona.traits = persona_traits(
        attributes.openness,
        attributes.extroversion,
        attributes.conscientiousness,
        attributes.agreeableness,
        attributes.technical_depth,
        attributes.meme_affinity,
    );
    persona.openness = attributes.openness;
    persona.extroversion = attributes.extroversion;
    persona.conscientiousness = attributes.conscientiousness;
    persona.agreeableness = attributes.agreeableness;
    persona.neuroticism = attributes.neuroticism;
    persona.posting_frequency = attributes.posting_frequency;
    persona.controversy_affinity = attributes.controversy_affinity;
    persona.humor_affinity = attributes.humor_affinity;
    persona.technical_depth = attributes.technical_depth;
    persona.meme_affinity = attributes.meme_affinity;
    persona.verbosity = attributes.verbosity;
    persona.sleep_phase = attributes.sleep_phase;
    persona.activity_pattern = attributes.activity_pattern;
}

fn push_persona_created_activity(
    activity_events: &mut Vec<ActivityEvent>,
    user: &User,
    id: PersonaId,
    timestamp: ModelTimestamp,
) -> Result<(), ForumGenerationError> {
    push_activity(
        activity_events,
        ActivityEventKind::PersonaCreated,
        Some(user.id.clone()),
        ActivityObject::Persona(id),
        timestamp,
    )
}

fn generate_interests() -> Result<Vec<Interest>, ForumGenerationError> {
    interest_specs()
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
        let mut scored: Vec<(usize, f32)> = interest_specs()
            .enumerate()
            .map(|(index, spec)| {
                let tie_breaker = persona_interest_tie_breaker(config, persona, &spec);
                (index, persona_interest_score(persona, &spec) + tie_breaker)
            })
            .collect();
        scored.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| INTEREST_IDS[left.0].cmp(INTEREST_IDS[right.0]))
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
    let interest_slugs = interest_specs()
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
            let intent = post_intent_for(
                config,
                id.as_str(),
                topic,
                &community.name,
                &author.display_name,
                persona,
            );
            let draft = LanguageDraft {
                intent,
                kind: UtteranceKind::Post,
                author_name: author.display_name.clone(),
                community_name: community.name.clone(),
                topic: topic.to_string(),
                referenced_post_title: None,
                referenced_post_topic: None,
            };
            let rendered = render_intent_draft(config, "posts", id.as_str(), &draft);
            let body =
                apply_style_pipeline(config, "posts", id.as_str(), persona, &draft, rendered);
            let mut post = Post::new(id.clone(), author.id.clone(), body, timestamp.clone())?;
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

#[allow(clippy::too_many_arguments)]
fn generate_comments(
    config: &ForumScenarioConfig,
    users: &[User],
    personas: &[Persona],
    posts: &[Post],
    communities: &[Community],
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
            let referenced_title = post.title.clone();
            let referenced_topic = referenced_title
                .as_deref()
                .and_then(|title| title.split_once(": ").map(|(_, topic)| topic))
                .map(str::to_string)
                .unwrap_or_else(|| topic_for_post_title(post));
            let intent = comment_intent_for(
                config,
                id.as_str(),
                post,
                &author.display_name,
                persona_for_user(personas, &author.id),
            );
            let community_name = communities
                .get(*community_indexes.get(community_id).expect(
                    "generated post community should be available for comment language generation",
                ))
                .expect("community index should resolve for post comment language")
                .name
                .clone();
            let draft = LanguageDraft {
                intent,
                kind: UtteranceKind::Comment,
                author_name: author.display_name.clone(),
                community_name,
                topic: post_topic_from_title(post),
                referenced_post_title: referenced_title,
                referenced_post_topic: Some(referenced_topic),
            };
            let rendered = render_intent_draft(config, "comments", id.as_str(), &draft);
            let body = apply_style_pipeline(
                config,
                "comments",
                id.as_str(),
                persona_for_user(personas, &author.id),
                &draft,
                rendered,
            );
            let persona = persona_for_user(personas, &author.id);
            let timestamp =
                scheduled_comment_timestamp(config, index, id.as_str(), post, author, persona)?;
            let comment = Comment::new(
                id.clone(),
                post.id.clone(),
                author.id.clone(),
                body,
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
        compare_activity_timestamps(left, right)
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });
}

fn compare_activity_timestamps(left: &ActivityEvent, right: &ActivityEvent) -> Ordering {
    match (
        parse_timestamp_minutes(left.occurred_at.as_str()),
        parse_timestamp_minutes(right.occurred_at.as_str()),
    ) {
        (Some(left_minutes), Some(right_minutes)) => left_minutes.cmp(&right_minutes),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.occurred_at.as_str().cmp(right.occurred_at.as_str()),
    }
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
        let Some(post_minutes) = parse_timestamp_minutes(post.created_at.as_str()) else {
            continue;
        };
        require_after_user(
            "post",
            post.id.as_str(),
            &post.author_id,
            post_minutes,
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
        let Some(comment_minutes) = parse_timestamp_minutes(comment.created_at.as_str()) else {
            continue;
        };
        require_after_user(
            "comment",
            comment.id.as_str(),
            &comment.author_id,
            comment_minutes,
            user_created_at,
            errors,
        );
        match post_by_id.get(&comment.post_id) {
            Some(post)
                if parse_timestamp_minutes(post.created_at.as_str())
                    .is_some_and(|post_minutes| comment_minutes > post_minutes) => {}
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
        let Some(relationship_minutes) = parse_timestamp_minutes(relationship.created_at.as_str())
        else {
            continue;
        };
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

struct ActivityValidationContext<'a> {
    user_created_at: &'a HashMap<UserId, usize>,
    user_by_id: &'a HashMap<UserId, &'a User>,
    persona_by_id: &'a HashMap<PersonaId, &'a Persona>,
    community_by_id: &'a HashMap<CommunityId, &'a Community>,
    post_by_id: &'a HashMap<PostId, &'a Post>,
    comment_by_id: &'a HashMap<CommentId, &'a Comment>,
    relationship_by_id: &'a HashMap<RelationshipId, &'a Relationship>,
}

fn validate_activity_events(
    dataset: &ForumDataset,
    context: &ActivityValidationContext<'_>,
    errors: &mut Vec<String>,
) {
    let mut previous = None;

    for event in &dataset.activity_events {
        let event_minutes = parse_timestamp_minutes(event.occurred_at.as_str());
        if let Some(event_minutes) = event_minutes {
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
                    context.user_created_at,
                    errors,
                );
            }
        }

        validate_activity_object_time(event, event_minutes, context, errors);
    }
}

fn validate_activity_object_time(
    event: &ActivityEvent,
    event_minutes: Option<usize>,
    context: &ActivityValidationContext<'_>,
    errors: &mut Vec<String>,
) {
    let Some(expected_object) = expected_activity_object(event.kind) else {
        return;
    };
    let actual_object = activity_object_name(&event.object);
    if actual_object != expected_object {
        errors.push(format!(
            "activity event {} kind/object mismatch: {:?} must reference {expected_object}, got {actual_object}",
            event.id, event.kind
        ));
        return;
    }

    match &event.object {
        ActivityObject::User(user_id) => {
            require_activity_matches(
                event,
                event_minutes,
                context.user_by_id.get(user_id),
                "user",
                errors,
            );
        }
        ActivityObject::Persona(persona_id) => {
            require_activity_matches(
                event,
                event_minutes,
                context.persona_by_id.get(persona_id),
                "persona",
                errors,
            );
        }
        ActivityObject::Community(community_id) => {
            require_activity_matches(
                event,
                event_minutes,
                context.community_by_id.get(community_id),
                "community",
                errors,
            );
        }
        ActivityObject::Post(post_id) => {
            require_activity_matches(
                event,
                event_minutes,
                context.post_by_id.get(post_id),
                "post",
                errors,
            );
        }
        ActivityObject::Comment(comment_id) => {
            require_activity_matches(
                event,
                event_minutes,
                context.comment_by_id.get(comment_id),
                "comment",
                errors,
            );
        }
        ActivityObject::Relationship(relationship_id) => {
            require_activity_matches(
                event,
                event_minutes,
                context.relationship_by_id.get(relationship_id),
                "relationship",
                errors,
            );
        }
        _ => {}
    }
}

fn expected_activity_object(kind: ActivityEventKind) -> Option<&'static str> {
    match kind {
        ActivityEventKind::UserCreated => Some("user"),
        ActivityEventKind::PersonaCreated => Some("persona"),
        ActivityEventKind::PostCreated => Some("post"),
        ActivityEventKind::CommentCreated => Some("comment"),
        ActivityEventKind::RelationshipCreated => Some("relationship"),
        ActivityEventKind::CommunityJoined => Some("community"),
        ActivityEventKind::ProfileUpdated
        | ActivityEventKind::StatusChanged
        | ActivityEventKind::ReactionCreated
        | ActivityEventKind::OrganizationJoined => None,
    }
}

fn activity_object_name(object: &ActivityObject) -> &'static str {
    match object {
        ActivityObject::User(_) => "user",
        ActivityObject::Profile(_) => "profile",
        ActivityObject::Username(_) => "username",
        ActivityObject::Persona(_) => "persona",
        ActivityObject::Interest(_) => "interest",
        ActivityObject::Status(_) => "status",
        ActivityObject::Post(_) => "post",
        ActivityObject::Comment(_) => "comment",
        ActivityObject::Reaction(_) => "reaction",
        ActivityObject::Relationship(_) => "relationship",
        ActivityObject::Community(_) => "community",
        ActivityObject::Organization(_) => "organization",
    }
}

trait CreatedAt {
    fn created_at(&self) -> &ModelTimestamp;
    fn id_text(&self) -> String;
}

impl CreatedAt for User {
    fn created_at(&self) -> &ModelTimestamp {
        &self.created_at
    }

    fn id_text(&self) -> String {
        self.id.to_string()
    }
}

impl CreatedAt for Persona {
    fn created_at(&self) -> &ModelTimestamp {
        &self.created_at
    }

    fn id_text(&self) -> String {
        self.id.to_string()
    }
}

impl CreatedAt for Community {
    fn created_at(&self) -> &ModelTimestamp {
        &self.created_at
    }

    fn id_text(&self) -> String {
        self.id.to_string()
    }
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
    event_minutes: Option<usize>,
    record: Option<&&T>,
    entity: &str,
    errors: &mut Vec<String>,
) {
    match record {
        Some(record)
            if event_minutes.is_some_and(|event_minutes| {
                parse_timestamp_minutes(record.created_at().as_str())
                    .is_some_and(|record_minutes| event_minutes == record_minutes)
            }) => {}
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

fn choose<T: Copy>(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    field: &str,
    values: &[T],
) -> T {
    values[deterministic_index(config, namespace, entity_id, field, values.len())]
}

fn choose_phrase(
    config: &ForumScenarioConfig,
    entity_id: &str,
    field: &str,
    values: &'static [&'static str],
) -> &'static str {
    values[deterministic_index(config, "bios", entity_id, field, values.len())]
}

fn post_intent_for(
    config: &ForumScenarioConfig,
    post_id: &str,
    topic: &str,
    community: &str,
    _author: &str,
    persona: &Persona,
) -> SemanticIntent {
    let technical_topic =
        topic_contains(topic, &["bug", "tooling", "architecture", "roadmap", "api"]);
    let social_topic = topic_contains(topic, &["community", "norms", "resource"]);
    let excitement_topic = topic_contains(
        topic,
        &["daily", "retrospective", "launch", "announcements"],
    );
    let humor_topic = topic_contains(topic, &["memes", "humor", "light", "fun"]);
    let profile = style_profile_for(persona);
    let topic_pressure = (topic.len() as f32) / 22.0;
    let community_pressure = (community.len() as f32) / 12.0;

    let mut weights = [
        (SemanticIntent::Frustration, 600usize),
        (SemanticIntent::Excitement, 600usize),
        (SemanticIntent::TechnicalAdvice, 600usize),
        (SemanticIntent::Agreement, 600usize),
        (SemanticIntent::Disagreement, 600usize),
        (SemanticIntent::Question, 600usize),
        (SemanticIntent::Clarification, 600usize),
        (SemanticIntent::Humor, 600usize),
        (SemanticIntent::Sarcasm, 600usize),
    ];

    weights[0].1 = weights[0].1.saturating_add(
        ((1.0 - persona.technical_depth) * 420.0 + (1.0 - persona.agreeableness) * 260.0) as usize,
    );
    weights[1].1 = weights[1].1.saturating_add(
        ((1.0 + excitement_topic as f32) * 190.0 + community_pressure * 15.0) as usize,
    );
    weights[2].1 = weights[2].1.saturating_add(
        ((persona.technical_depth * 2_800.0)
            + (persona.conscientiousness * 200.0)
            + technical_topic as f32 * 240.0) as usize,
    );
    weights[3].1 = weights[3]
        .1
        .saturating_add(((persona.agreeableness + persona.technical_depth) * 350.0) as usize);
    weights[4].1 = weights[4].1.saturating_add(
        ((1.0 + persona.controversy_affinity) * 340.0 + profile.sarcasm_bias as f32 * 0.8) as usize,
    );
    weights[5].1 = weights[5].1.saturating_add(
        ((persona.extroversion + persona.posting_frequency) * 360.0 + topic_pressure * 80.0)
            as usize,
    );
    weights[6].1 = weights[6].1.saturating_add(
        ((persona.conscientiousness * 360.0 + social_topic as f32 * 90.0)
            + profile.question_bias as usize as f32) as usize,
    );
    weights[7].1 = weights[7].1.saturating_add(
        ((profile.humor_bias as f32 * 1.1) + humor_topic as f32 * 130.0 + community_pressure * 5.0)
            as usize,
    );
    weights[8].1 = weights[8].1.saturating_add(
        ((profile.sarcasm_bias as f32 * 1.0) + persona.neuroticism * 280.0 + topic_pressure * 70.0)
            as usize,
    );

    select_intent(
        deterministic_index(config, "posts", post_id, "intent", 10_000),
        &weights,
    )
}

fn comment_intent_for(
    config: &ForumScenarioConfig,
    comment_id: &str,
    post: &Post,
    _author: &str,
    persona: &Persona,
) -> SemanticIntent {
    let mut weights = [
        (SemanticIntent::Frustration, 560usize),
        (SemanticIntent::Excitement, 560usize),
        (SemanticIntent::TechnicalAdvice, 560usize),
        (SemanticIntent::Agreement, 560usize),
        (SemanticIntent::Disagreement, 560usize),
        (SemanticIntent::Question, 560usize),
        (SemanticIntent::Clarification, 560usize),
        (SemanticIntent::Humor, 560usize),
        (SemanticIntent::Sarcasm, 560usize),
    ];
    let topic = post_topic_from_title(post);
    let technical_topic =
        topic_contains(&topic, &["bug", "tooling", "architecture", "api", "data"]);
    let question_density = topic_contains(
        &topic,
        &[
            "how", "why", "should", "whether", "if", "can", "when", "where",
        ],
    );
    let style = style_profile_for(persona);
    let post_title_bias = if post.title.is_some() { 20 } else { 0 };
    let complexity = (topic.len() as f32) / 24.0;

    weights[0].1 = weights[0]
        .1
        .saturating_add(((1.0 - persona.openness) * 390.0 + complexity * 85.0) as usize);
    weights[1].1 = weights[1]
        .1
        .saturating_add((style.detail_bias as f32 * 0.35 + post_title_bias as f32) as usize);
    weights[2].1 = weights[2].1.saturating_add(
        ((persona.technical_depth * 920.0) + (technical_topic as f32) * 230.0 + complexity * 40.0)
            as usize,
    );
    weights[3].1 = weights[3].1.saturating_add(
        ((persona.agreeableness + persona.conscientiousness) * 420.0 + complexity * 50.0) as usize,
    );
    weights[4].1 = weights[4].1.saturating_add(
        ((persona.controversy_affinity + persona.neuroticism) * 360.0 + post_title_bias as f32)
            as usize,
    );
    weights[5].1 = weights[5].1.saturating_add(
        ((persona.posting_frequency + persona.extroversion) * 350.0) as usize
            + question_density * 45,
    );
    weights[6].1 = weights[6].1.saturating_add(
        ((1.0 - persona.agreeableness) * 360.0
            + complexity * 75.0
            + style.question_bias as usize as f32 * 120.0) as usize,
    );
    weights[7].1 = weights[7]
        .1
        .saturating_add(((persona.humor_affinity + persona.meme_affinity) * 520.0) as usize);
    weights[8].1 = weights[8].1.saturating_add(
        (persona.controversy_affinity * 460.0 + style.sarcasm_bias as f32) as usize
            + style.question_bias as usize * 20,
    );

    select_intent(
        deterministic_index(config, "comments", comment_id, "intent", 10_000),
        &weights,
    )
}

fn render_intent_draft(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    draft: &LanguageDraft,
) -> String {
    let template = match draft.kind {
        UtteranceKind::Post => {
            choose_intent_template(config, namespace, entity_id, draft.intent, DraftKind::Post)
        }
        UtteranceKind::Comment => choose_intent_template(
            config,
            namespace,
            entity_id,
            draft.intent,
            DraftKind::Comment,
        ),
    };

    template_replace(template, draft)
}

#[derive(Clone, Copy)]
enum DraftKind {
    Post,
    Comment,
}

fn choose_intent_template(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    intent: SemanticIntent,
    kind: DraftKind,
) -> &'static str {
    match kind {
        DraftKind::Post => choose(
            config,
            namespace,
            entity_id,
            "post_intent_template",
            post_template_bank(intent),
        ),
        DraftKind::Comment => choose(
            config,
            namespace,
            entity_id,
            "comment_intent_template",
            comment_template_bank(intent),
        ),
    }
}

fn post_template_bank(intent: SemanticIntent) -> &'static [&'static str] {
    match intent {
        SemanticIntent::Frustration => POST_FRUSTRATION_TEMPLATES,
        SemanticIntent::Excitement => POST_EXCITEMENT_TEMPLATES,
        SemanticIntent::TechnicalAdvice => POST_TECHNICAL_ADVICE_TEMPLATES,
        SemanticIntent::Agreement => POST_AGREEMENT_TEMPLATES,
        SemanticIntent::Disagreement => POST_DISAGREEMENT_TEMPLATES,
        SemanticIntent::Question => POST_QUESTION_TEMPLATES,
        SemanticIntent::Clarification => POST_CLARIFICATION_TEMPLATES,
        SemanticIntent::Humor => POST_HUMOR_TEMPLATES,
        SemanticIntent::Sarcasm => POST_SARCASTIC_TEMPLATES,
    }
}

fn comment_template_bank(intent: SemanticIntent) -> &'static [&'static str] {
    match intent {
        SemanticIntent::Frustration => COMMENT_FRUSTRATION_TEMPLATES,
        SemanticIntent::Excitement => COMMENT_EXCITEMENT_TEMPLATES,
        SemanticIntent::TechnicalAdvice => COMMENT_TECHNICAL_ADVICE_TEMPLATES,
        SemanticIntent::Agreement => COMMENT_AGREEMENT_TEMPLATES,
        SemanticIntent::Disagreement => COMMENT_DISAGREEMENT_TEMPLATES,
        SemanticIntent::Question => COMMENT_QUESTION_TEMPLATES,
        SemanticIntent::Clarification => COMMENT_CLARIFICATION_TEMPLATES,
        SemanticIntent::Humor => COMMENT_HUMOR_TEMPLATES,
        SemanticIntent::Sarcasm => COMMENT_SARCASTIC_TEMPLATES,
    }
}

fn template_replace(template: &str, draft: &LanguageDraft) -> String {
    template
        .replace("{author}", &draft.author_name)
        .replace("{community}", &draft.community_name)
        .replace("{topic}", &draft.topic)
        .replace(
            "{post_title}",
            draft
                .referenced_post_title
                .as_deref()
                .unwrap_or(&draft.topic),
        )
        .replace(
            "{post_topic}",
            draft
                .referenced_post_topic
                .as_deref()
                .unwrap_or(&draft.topic),
        )
}

fn select_intent(sample: usize, weights: &[(SemanticIntent, usize); 9]) -> SemanticIntent {
    let total = weights.iter().map(|(_, weight)| weight).sum::<usize>();
    if total == 0 {
        return weights[0].0;
    }

    let mut target = sample % total;
    for (intent, weight) in weights {
        if target < *weight {
            return *intent;
        }
        target -= *weight;
    }

    weights[0].0
}

fn style_profile_for(persona: &Persona) -> StyleProfile {
    let detail_signal =
        (persona.technical_depth + persona.conscientiousness + persona.agreeableness) / 3.0;
    let detail_bias = ((detail_signal * 68.0) + (persona.posting_frequency * 14.0)) as usize;
    let abbreviation_bias = ((1.0 - persona.conscientiousness) * 72.0) as usize;
    let humor_bias = ((persona.humor_affinity + persona.meme_affinity) * 54.0) as usize;
    let sarcasm_bias = ((persona.controversy_affinity
        + persona.neuroticism
        + if matches!(persona.activity_pattern, ActivityPattern::Lurker) {
            0.25
        } else {
            0.0
        })
        * 45.0) as usize;
    let typo_bias = ((1.0 - persona.conscientiousness) * 26.0) as usize;
    let emoji_bias =
        ((persona.humor_affinity * 40.0 + persona.meme_affinity * 32.0) * 0.9) as usize;
    let question_bias = matches!(
        persona.activity_pattern,
        ActivityPattern::Regular | ActivityPattern::PowerUser
    ) || matches!(persona.verbosity, Verbosity::Terse | Verbosity::Balanced);

    StyleProfile {
        verbosity: persona.verbosity,
        detail_bias: detail_bias.min(100),
        abbreviation_bias: abbreviation_bias.min(100),
        humor_bias: humor_bias.min(100),
        sarcasm_bias: sarcasm_bias.min(100),
        typo_bias: typo_bias.min(100),
        emoji_bias: emoji_bias.min(100),
        question_bias,
    }
}

fn apply_style_pipeline(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    persona: &Persona,
    draft: &LanguageDraft,
    rendered: String,
) -> String {
    let profile = style_profile_for(persona);
    let mut text = apply_verbosity_profile(config, namespace, entity_id, &profile, draft, rendered);
    if !matches!(profile.verbosity, Verbosity::Terse) {
        text = apply_structural_variants(config, namespace, entity_id, &profile, draft, text);
        text = apply_synonym_mutation(config, namespace, entity_id, &draft.kind, text);
    }
    text = normalize_punctuation_and_cadence(config, namespace, entity_id, &profile, draft, text);
    text = apply_abbreviation_variants(config, namespace, entity_id, draft, &profile, text);
    text = apply_style_tone_helpers(config, namespace, entity_id, draft, &profile, text);
    text = apply_humor_and_sarcasm_variants(config, namespace, entity_id, draft, &profile, text);
    text = apply_typos_and_emoji(config, namespace, entity_id, &profile, draft, text);

    text
}

fn apply_verbosity_profile(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    profile: &StyleProfile,
    _draft: &LanguageDraft,
    rendered: String,
) -> String {
    match profile.verbosity {
        Verbosity::Terse => {
            let compact = choose(
                config,
                namespace,
                entity_id,
                "compact_prefix",
                &["", "In short:", "TL;DR:", "Quickly:"],
            );
            if compact.is_empty() {
                rendered
            } else {
                format!("{compact} {rendered}")
            }
        }
        Verbosity::Balanced => rendered,
        Verbosity::Detailed => {
            let detail = choose(
                config,
                namespace,
                entity_id,
                "detail_append",
                &[
                    "I can follow with a concrete check list next.",
                    "From a reproducible standpoint, a small pilot is the safest next step.",
                    "The next move is to verify tradeoffs and publish a practical sequence.",
                ],
            );
            format!("{rendered} {detail}")
        }
    }
}

fn apply_structural_variants(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    profile: &StyleProfile,
    _draft: &LanguageDraft,
    mut text: String,
) -> String {
    let opener = choose(
        config,
        namespace,
        entity_id,
        "opener",
        if profile.detail_bias >= 55 {
            OPENER_TEMPLATES
        } else {
            &OPENER_TEMPLATES[1..]
        },
    );
    if !opener.is_empty() {
        text = format!("{opener} {text}");
    }

    let rewrite = choose(
        config,
        namespace,
        entity_id,
        "structural_rewrite",
        STRUCTURAL_REWRITES,
    );
    let connector = choose(
        config,
        namespace,
        entity_id,
        "connector",
        &["so", "therefore", "that said", "for context"],
    );
    text = rewrite
        .replace("{body}", &text)
        .replace("{connector}", connector);

    let closer = choose(config, namespace, entity_id, "closer", CLOSER_TEMPLATES);
    if !closer.is_empty() {
        text = format!("{text} {closer}");
    }

    text
}

fn normalize_punctuation_and_cadence(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    profile: &StyleProfile,
    draft: &LanguageDraft,
    rendered: String,
) -> String {
    let mut text = rendered.trim().to_string();
    let cadence = deterministic_index(config, namespace, entity_id, "cadence", 6);
    let base = if matches!(
        draft.intent,
        SemanticIntent::Question | SemanticIntent::Clarification
    ) || profile.question_bias
    {
        '?'
    } else {
        match cadence {
            0 => '.',
            1 => '.',
            2 => '.',
            3 => '!',
            _ => '.',
        }
    };

    while matches!(text.chars().last(), Some('.' | '!' | '?')) {
        text.pop();
    }
    text.push(base);

    text
}

fn apply_abbreviation_variants(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    _draft: &LanguageDraft,
    profile: &StyleProfile,
    mut rendered: String,
) -> String {
    let count = (profile.abbreviation_bias / 45).min(2);
    for slot in 0..count {
        let source_field = format!("abbr_source_{slot}");
        let index = deterministic_index(
            config,
            namespace,
            entity_id,
            source_field.as_str(),
            ABBREVIATION_PAIRS.len(),
        );
        let (source, replacement) = ABBREVIATION_PAIRS[index];
        let replaced = replace_word(&rendered, source, replacement);
        if replaced != rendered {
            rendered = replaced;
        }
    }

    rendered
}

fn apply_style_tone_helpers(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    draft: &LanguageDraft,
    profile: &StyleProfile,
    mut rendered: String,
) -> String {
    if profile.detail_bias > 60 {
        rendered = match draft.intent {
            SemanticIntent::Agreement => {
                rendered
                    + " "
                    + choose(
                        config,
                        namespace,
                        entity_id,
                        "agreement",
                        AGREEMENT_VARIANTS,
                    )
            }
            SemanticIntent::TechnicalAdvice => {
                rendered + " " + choose(config, namespace, entity_id, "advice", ADVICE_VARIANTS)
            }
            SemanticIntent::Frustration | SemanticIntent::Disagreement => {
                rendered + " " + choose(config, namespace, entity_id, "concern", CONCERN_VARIANTS)
            }
            _ => rendered,
        };
    }

    rendered
}

fn apply_humor_and_sarcasm_variants(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    draft: &LanguageDraft,
    profile: &StyleProfile,
    mut rendered: String,
) -> String {
    if profile.humor_bias > 45
        && matches!(
            draft.intent,
            SemanticIntent::Humor | SemanticIntent::Excitement
        )
    {
        rendered = format!(
            "{rendered} {}",
            choose(config, namespace, entity_id, "humor", HUMOR_VARIANTS)
        );
    }

    if profile.sarcasm_bias > 55
        && matches!(
            draft.intent,
            SemanticIntent::Sarcasm | SemanticIntent::Disagreement | SemanticIntent::Frustration,
        )
    {
        rendered = format!(
            "{rendered} {}",
            choose(config, namespace, entity_id, "sarcasm", SARCASTIC_VARIANTS)
        );
    }

    rendered
}

fn apply_typos_and_emoji(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    profile: &StyleProfile,
    _draft: &LanguageDraft,
    mut rendered: String,
) -> String {
    let typo_gate = deterministic_index(config, namespace, entity_id, "typo_gate", 100);
    if typo_gate < profile.typo_bias {
        let typo = TYPO_PAIRS
            [deterministic_index(config, namespace, entity_id, "typo_pick", TYPO_PAIRS.len())];
        let replaced = replace_word(&rendered, typo.0, typo.1);
        if replaced != rendered {
            rendered = replaced;
        }
    }

    let emoji_gate = deterministic_index(config, namespace, entity_id, "emoji_gate", 100);
    if emoji_gate < profile.emoji_bias {
        rendered = format!(
            "{rendered} {}",
            choose(
                config,
                namespace,
                entity_id,
                "emoji",
                &[":)", ":D", ";)", "*smile*", "<3"],
            )
        );
    }

    rendered
}

fn apply_synonym_mutation(
    config: &ForumScenarioConfig,
    namespace: &str,
    entity_id: &str,
    kind: &UtteranceKind,
    mut rendered: String,
) -> String {
    let max_mutations = if matches!(kind, UtteranceKind::Post) {
        2
    } else {
        1
    };
    let mut applied = 0usize;

    for slot in 0..max_mutations {
        let group = SYNONYM_GROUPS[deterministic_index(
            config,
            namespace,
            entity_id,
            &format!("synonym_group_{slot}"),
            SYNONYM_GROUPS.len(),
        )];
        if group.len() < 2 {
            continue;
        }
        let source = group[deterministic_index(
            config,
            namespace,
            entity_id,
            &format!("synonym_source_{slot}"),
            group.len(),
        )];
        let replacement = group[deterministic_index(
            config,
            namespace,
            entity_id,
            &format!("synonym_target_{slot}"),
            group.len(),
        )];

        if source == replacement {
            continue;
        }
        let replaced = replace_word(&rendered, source, replacement);
        if replaced != rendered {
            rendered = replaced;
            applied += 1;
        }

        if applied >= max_mutations {
            break;
        }
    }

    rendered
}

fn replace_word(value: &str, source: &str, replacement: &str) -> String {
    let spaced_source = format!(" {} ", source);
    if let Some(index) = value.find(&spaced_source) {
        let start = index + 1;
        let end = start + source.len();
        let mut output = String::with_capacity(value.len());
        output.push_str(&value[..index]);
        output.push(' ');
        output.push_str(replacement);
        output.push(' ');
        output.push_str(&value[end + 1..]);
        return output;
    }

    if value.starts_with(source) {
        return value.replacen(source, replacement, 1);
    }

    if value.ends_with(source) {
        return value
            .strip_suffix(source)
            .expect("suffix should exist")
            .to_string()
            + replacement;
    }

    value.to_string()
}

fn topic_contains(topic: &str, tokens: &[&str]) -> usize {
    let haystack = topic.to_lowercase();
    tokens
        .iter()
        .filter(|token| haystack.contains(**token))
        .count()
}

fn topic_for_post_title(post: &Post) -> String {
    if let Some(title) = post.title.as_deref() {
        title
            .split_once(": ")
            .map(|(_, topic)| topic)
            .unwrap_or(title)
            .to_string()
    } else {
        post.body.clone()
    }
}

fn post_topic_from_title(post: &Post) -> String {
    topic_for_post_title(post)
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
const ACTIVITY_SIMULATION_DAYS: usize = 366 * 4;
const ACTIVITY_SIMULATION_MINUTES: usize = ACTIVITY_SIMULATION_DAYS * MINUTES_PER_DAY;

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
    let minutes = advance_activity_candidate_after_minimum(candidate, minimum);

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

    advance_activity_candidate_after_minimum(scheduled, minimum)
}

fn advance_activity_candidate_after_minimum(candidate: usize, minimum: usize) -> usize {
    if candidate >= minimum {
        candidate
    } else {
        candidate
            + ((minimum - candidate) / ACTIVITY_SIMULATION_MINUTES + 1)
                * ACTIVITY_SIMULATION_MINUTES
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

    (anchor
        + cluster * cadence
        + if matches!(
            persona.activity_pattern,
            ActivityPattern::Bursty | ActivityPattern::PowerUser
        ) {
            burst_day
        } else {
            cluster_day
        })
        % ACTIVITY_SIMULATION_DAYS
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
        assert_eq!(dataset.interests.len(), INTEREST_CATALOG_LEN);
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
    fn language_intents_cover_all_required_categories() {
        let config = forum_config_with("intent-distribution", 160, 800, 800);
        let dataset = generate_forum_dataset(&config).expect("dataset should build");
        let mut post_counts = [0usize; 9];
        let mut comment_counts = [0usize; 9];

        for post in &dataset.posts {
            let persona = dataset
                .personas
                .iter()
                .find(|persona| persona.user_id == post.author_id)
                .expect("post author persona should exist");
            let author = dataset
                .users
                .iter()
                .find(|user| user.id == post.author_id)
                .expect("post author should exist");
            let community = dataset
                .communities
                .iter()
                .find(|community| Some(&community.id) == post.community_id.as_ref())
                .expect("post community should exist");
            let intent = post_intent_for(
                &config,
                post.id.as_str(),
                &post_topic_from_title(post),
                &community.name,
                &author.display_name,
                persona,
            );

            post_counts[intent_index(intent)] += 1;
        }

        for comment in &dataset.comments {
            let post = dataset
                .posts
                .iter()
                .find(|post| post.id == comment.post_id)
                .expect("comment should reference a post");
            let intent = comment_intent_for(
                &config,
                comment.id.as_str(),
                post,
                "",
                dataset
                    .personas
                    .iter()
                    .find(|persona| persona.user_id == comment.author_id)
                    .expect("comment author persona should exist"),
            );

            comment_counts[intent_index(intent)] += 1;
        }

        assert!(post_counts.iter().all(|count| *count > 0));
        assert!(comment_counts.iter().all(|count| *count > 0));
    }

    #[test]
    fn technical_depth_bias_increases_technical_advice_for_posts() {
        let config = small_forum_config();
        let mut technical = affinity_persona("tech-intent", SleepPhase::Daytime, 0.80, 0.25, 0.90);
        technical.technical_depth = 0.98;
        technical.humor_affinity = 0.08;
        technical.meme_affinity = 0.05;
        let mut social = affinity_persona("social-intent", SleepPhase::Daytime, 0.80, 0.40, 0.20);
        social.technical_depth = 0.12;
        social.humor_affinity = 0.60;
        social.meme_affinity = 0.80;

        let topic = "bug triage";
        let community = "general";
        let mut tech_advice = 0usize;
        let mut social_advice = 0usize;

        for index in 0..240 {
            let id = format!("post-{index:06}");
            let technical_intent =
                post_intent_for(&config, &id, topic, community, "Alex", &technical);
            let social_intent = post_intent_for(&config, &id, topic, community, "Alex", &social);

            if technical_intent == SemanticIntent::TechnicalAdvice {
                tech_advice += 1;
            }
            if social_intent == SemanticIntent::TechnicalAdvice {
                social_advice += 1;
            }
        }

        assert!(tech_advice > social_advice * 2);
    }

    #[test]
    fn verbosity_profile_affects_output_length() {
        let config = small_forum_config();
        let topic = "bug triage".to_string();
        let community = "general".to_string();
        let base = "A concrete rollout sequence for this topic in the team thread.".to_string();
        let mut persona_template =
            affinity_persona("verbosity", SleepPhase::Daytime, 0.80, 0.20, 0.80);
        persona_template.user_id = user_id(9).expect("valid test user");
        let mut terse = persona_template.clone();
        terse.verbosity = Verbosity::Terse;
        let mut balanced = persona_template.clone();
        balanced.verbosity = Verbosity::Balanced;
        let mut detailed = persona_template;
        detailed.verbosity = Verbosity::Detailed;

        let mut terse_lengths = Vec::new();
        let mut balanced_lengths = Vec::new();
        let mut detailed_lengths = Vec::new();

        for slot in 0..20 {
            let draft = LanguageDraft {
                intent: SemanticIntent::TechnicalAdvice,
                kind: UtteranceKind::Post,
                author_name: "Alex".to_string(),
                community_name: community.clone(),
                topic: topic.clone(),
                referenced_post_title: None,
                referenced_post_topic: None,
            };
            terse_lengths.push(
                apply_style_pipeline(
                    &config,
                    "style-length",
                    &format!("terse-{slot}"),
                    &terse,
                    &draft,
                    base.clone(),
                )
                .len(),
            );
            balanced_lengths.push(
                apply_style_pipeline(
                    &config,
                    "style-length",
                    &format!("balanced-{slot}"),
                    &balanced,
                    &draft,
                    base.clone(),
                )
                .len(),
            );
            detailed_lengths.push(
                apply_style_pipeline(
                    &config,
                    "style-length",
                    &format!("detailed-{slot}"),
                    &detailed,
                    &draft,
                    base.clone(),
                )
                .len(),
            );
        }

        let terse_avg =
            terse_lengths.iter().copied().sum::<usize>() as f64 / terse_lengths.len() as f64;
        let balanced_avg =
            balanced_lengths.iter().copied().sum::<usize>() as f64 / balanced_lengths.len() as f64;
        let detailed_avg =
            detailed_lengths.iter().copied().sum::<usize>() as f64 / detailed_lengths.len() as f64;

        assert!(terse_avg < balanced_avg);
        assert!(detailed_avg > balanced_avg);
    }

    #[test]
    fn generated_content_maintains_bounded_duplicate_rates() {
        let config = ForumScenarioConfig {
            seed: "duplicate-smoke".to_string(),
            population: PopulationConfig { users: 180 },
            communities: vec![
                "general".to_string(),
                "support".to_string(),
                "programming".to_string(),
                "gaming".to_string(),
            ],
            content: ContentConfig {
                posts: 1_000,
                comments: 1_000,
            },
            output_format: OutputFormat::Jsonl,
        };
        let dataset = generate_forum_dataset(&config).expect("dataset should build");
        let post_bodies = dataset
            .posts
            .iter()
            .map(|post| post.body.clone())
            .collect::<Vec<_>>();
        let comment_bodies = dataset
            .comments
            .iter()
            .map(|comment| comment.body.clone())
            .collect::<Vec<_>>();

        assert!(duplicate_rate(&post_bodies) < 0.22);
        assert!(duplicate_rate(&comment_bodies) < 0.22);
        assert!(normalized_duplicate_rate(&post_bodies) < 0.12);
        assert!(normalized_duplicate_rate(&comment_bodies) < 0.12);
    }

    #[test]
    #[ignore = "large corpus duplicate smoke test"]
    fn generated_comments_maintain_low_duplicate_rate_at_100k() {
        let config = ForumScenarioConfig {
            seed: "duplicate-smoke-100k".to_string(),
            population: PopulationConfig { users: 400 },
            communities: vec![
                "general".to_string(),
                "support".to_string(),
                "programming".to_string(),
                "gaming".to_string(),
                "linux".to_string(),
            ],
            content: ContentConfig {
                posts: 2_000,
                comments: 100_000,
            },
            output_format: OutputFormat::Jsonl,
        };
        let dataset = generate_forum_dataset(&config).expect("dataset should build");
        let comment_bodies = dataset
            .comments
            .iter()
            .map(|comment| comment.body.clone())
            .collect::<Vec<_>>();

        assert!(normalized_duplicate_rate(&comment_bodies) < 0.10);
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
        assert!(!dataset.posts[0].body.trim().is_empty());
        assert!(!dataset.posts[0].body.chars().all(char::is_whitespace));
        assert_eq!(dataset.comments[0].id.as_str(), "comment-000001");
        assert_eq!(dataset.comments[0].post_id.as_str(), "post-000005");
        assert!(!dataset.comments[0].body.trim().is_empty());
        assert!(!dataset.comments[0].body.chars().all(char::is_whitespace));
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
    fn max_content_activity_schedule_stays_parseable_and_sortable() {
        let config = ForumScenarioConfig {
            seed: "temporal-large-window".to_string(),
            population: PopulationConfig { users: 1 },
            communities: vec!["general".to_string()],
            content: ContentConfig {
                posts: MAX_CONTENT_ITEMS,
                comments: MAX_CONTENT_ITEMS,
            },
            output_format: OutputFormat::Jsonl,
        };
        config
            .validate()
            .expect("max content config should be valid");
        let user = test_user("user-temporal-large", "en-US", &[]);
        let mut persona =
            affinity_persona("persona-temporal-large", SleepPhase::Daytime, 0.1, 0.1, 0.8);
        persona.user_id = user.id.clone();
        persona.activity_pattern = ActivityPattern::Lurker;

        let first = scheduled_post_timestamp(
            &config,
            MAX_CONTENT_ITEMS - 2,
            "post-999999",
            &user,
            &persona,
        )
        .expect("timestamp should build");
        let second = scheduled_post_timestamp(
            &config,
            MAX_CONTENT_ITEMS - 1,
            "post-1000000",
            &user,
            &persona,
        )
        .expect("timestamp should build");
        let repeated = scheduled_post_timestamp(
            &config,
            MAX_CONTENT_ITEMS - 1,
            "post-1000000",
            &user,
            &persona,
        )
        .expect("timestamp should build");

        assert_eq!(second, repeated);
        for timestamp in [&first, &second] {
            assert_eq!(timestamp.as_str().len(), 20);
            let minutes =
                parse_timestamp_minutes(timestamp.as_str()).expect("timestamp should parse");
            assert_eq!(timestamp.as_str(), timestamp_from_minutes(minutes));
        }

        let mut events = vec![
            ActivityEvent {
                id: ActivityEventId::new("activity-000002").expect("activity ID should be valid"),
                kind: ActivityEventKind::PostCreated,
                actor_id: Some(user.id.clone()),
                object: ActivityObject::Post(
                    PostId::new("post-1000000").expect("post ID should be valid"),
                ),
                occurred_at: second,
            },
            ActivityEvent {
                id: ActivityEventId::new("activity-000001").expect("activity ID should be valid"),
                kind: ActivityEventKind::PostCreated,
                actor_id: Some(user.id.clone()),
                object: ActivityObject::Post(
                    PostId::new("post-999999").expect("post ID should be valid"),
                ),
                occurred_at: first,
            },
        ];

        sort_activity_events(&mut events);

        let ordered_minutes = events
            .iter()
            .map(|event| {
                parse_timestamp_minutes(event.occurred_at.as_str())
                    .expect("sorted timestamp should parse")
            })
            .collect::<Vec<_>>();
        assert!(ordered_minutes.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn validator_reports_malformed_timestamps_without_panicking() {
        let mut dataset =
            generate_forum_dataset(&small_forum_config()).expect("dataset should build");
        dataset.users[0].created_at =
            ModelTimestamp::new("10000-01-01T00:00:00Z").expect("timestamp is non-empty");

        let error = validate_forum_temporal_consistency(&dataset)
            .expect_err("malformed timestamp should be reported");

        assert!(error.message().contains("unstable timestamp"));
        assert!(error.message().contains("10000-01-01T00:00:00Z"));
    }

    #[test]
    fn validator_checks_user_persona_and_community_activity_events() {
        let mut dataset =
            generate_forum_dataset(&small_forum_config()).expect("dataset should build");
        let user_id = dataset.users[0].id.clone();
        let persona_id = dataset.personas[0].id.clone();
        let community_id = dataset.communities[0].id.clone();
        let shifted_user_timestamp = timestamp_plus_minutes(&dataset.users[0].created_at, 1);
        let shifted_persona_timestamp = timestamp_plus_minutes(&dataset.personas[0].created_at, 1);
        let shifted_community_timestamp =
            timestamp_plus_minutes(&dataset.communities[0].created_at, 1);
        let extra_community_timestamp =
            timestamp_plus_minutes(&dataset.communities[0].created_at, 10);

        dataset
            .activity_events
            .iter_mut()
            .find(|event| event.kind == ActivityEventKind::UserCreated)
            .expect("user activity should exist")
            .occurred_at = shifted_user_timestamp;
        dataset
            .activity_events
            .iter_mut()
            .find(|event| event.kind == ActivityEventKind::PersonaCreated)
            .expect("persona activity should exist")
            .occurred_at = shifted_persona_timestamp;
        dataset
            .activity_events
            .iter_mut()
            .find(|event| event.kind == ActivityEventKind::CommunityJoined)
            .expect("community activity should exist")
            .occurred_at = shifted_community_timestamp;
        dataset.activity_events.push(ActivityEvent {
            id: ActivityEventId::new("activity-extra").expect("activity ID should be valid"),
            kind: ActivityEventKind::CommunityJoined,
            actor_id: Some(user_id),
            object: ActivityObject::Persona(persona_id),
            occurred_at: extra_community_timestamp,
        });

        let error = validate_forum_temporal_consistency(&dataset)
            .expect_err("activity mismatches should be reported");

        assert!(error.message().contains("user activity"));
        assert!(error.message().contains("persona activity"));
        assert!(error.message().contains("community activity"));
        assert!(error.message().contains("kind/object mismatch"));
        assert!(error.message().contains(community_id.as_str()));
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

        assert_eq!(dataset.interests.len(), INTEREST_CATALOG_LEN);
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
            persona_interest_score(&technical, &programming)
                > persona_interest_score(&social, &programming)
        );
        assert!(
            persona_interest_score(&technical, &linux) > persona_interest_score(&culture, &linux)
        );
        assert!(
            persona_interest_score(&social, &community)
                > persona_interest_score(&technical, &community)
        );
        assert!(
            persona_interest_score(&social, &events) > persona_interest_score(&culture, &events)
        );
        assert!(
            persona_interest_score(&culture, &gaming) > persona_interest_score(&social, &gaming)
        );
        assert!(
            persona_interest_score(&culture, &memes) > persona_interest_score(&technical, &memes)
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

    fn timestamp_plus_minutes(timestamp: &ModelTimestamp, minutes: usize) -> ModelTimestamp {
        let base = parse_timestamp_minutes(timestamp.as_str()).expect("timestamp should parse");
        ModelTimestamp::new(timestamp_from_minutes(base + minutes)).expect("timestamp should build")
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

    fn catalog_interest(id: &str) -> InterestSpec {
        interest_specs()
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

    fn intent_index(intent: SemanticIntent) -> usize {
        match intent {
            SemanticIntent::Frustration => 0,
            SemanticIntent::Excitement => 1,
            SemanticIntent::TechnicalAdvice => 2,
            SemanticIntent::Agreement => 3,
            SemanticIntent::Disagreement => 4,
            SemanticIntent::Question => 5,
            SemanticIntent::Clarification => 6,
            SemanticIntent::Humor => 7,
            SemanticIntent::Sarcasm => 8,
        }
    }

    fn duplicate_rate(values: &[String]) -> f64 {
        if values.len() <= 1 {
            return 0.0;
        }

        let mut unique = HashSet::new();
        let mut repeated = 0usize;

        for value in values {
            if !unique.insert(value.as_str()) {
                repeated += 1;
            }
        }

        repeated as f64 / values.len() as f64
    }

    fn normalized_duplicate_rate(values: &[String]) -> f64 {
        let normalized = values
            .iter()
            .map(|value| normalize_for_duplicate(value))
            .collect::<Vec<_>>();
        duplicate_rate(&normalized)
    }

    fn normalize_for_duplicate(value: &str) -> String {
        let mut normalized = String::with_capacity(value.len());
        let mut space_pending = false;

        for ch in value.to_lowercase().chars() {
            if ch.is_ascii_alphanumeric() {
                if space_pending && !normalized.is_empty() {
                    normalized.push(' ');
                    space_pending = false;
                }
                normalized.push(ch);
                continue;
            }

            if ch.is_whitespace() || ch.is_ascii_punctuation() {
                space_pending = true;
                continue;
            }

            // Strip emoji and other symbols to keep normalized metrics stable.
            space_pending = true;
        }

        while normalized.ends_with(' ') {
            normalized.pop();
        }

        normalized
    }
}
