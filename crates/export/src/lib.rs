use serde::Serialize;
use serde_json::{json, Value};
use std::fmt;
use synthetic_pop_core::{
    ActivityEvent, ActivityObject, Comment, Community, Interest, Persona, Post, Relationship,
    RelationshipEndpoint, User,
};
use synthetic_pop_scenarios::ForumDataset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportState {
    Implemented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Jsonl,
    Csv,
    SqliteSql,
    PostgresSql,
    PrismaSeed,
}

#[derive(Debug)]
pub enum ExportError {
    Json(serde_json::Error),
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "failed to serialize export payload: {error}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<serde_json::Error> for ExportError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[must_use]
pub const fn current_export_state() -> ExportState {
    ExportState::Implemented
}

#[must_use]
pub const fn export_enabled() -> bool {
    true
}

pub fn export_forum_dataset(
    dataset: &ForumDataset,
    format: ExportFormat,
) -> Result<String, ExportError> {
    match format {
        ExportFormat::Json => Ok(serde_json::to_string_pretty(dataset)?),
        ExportFormat::Jsonl => export_jsonl(dataset),
        ExportFormat::Csv => export_csv(dataset),
        ExportFormat::SqliteSql => export_sqlite_sql(dataset),
        ExportFormat::PostgresSql => export_postgres_sql(dataset),
        ExportFormat::PrismaSeed => export_prisma_seed(dataset),
    }
}

fn export_jsonl(dataset: &ForumDataset) -> Result<String, ExportError> {
    let mut lines = Vec::new();

    push_jsonl_records(&mut lines, "user", &dataset.users)?;
    push_jsonl_records(&mut lines, "persona", &dataset.personas)?;
    push_jsonl_records(&mut lines, "interest", &dataset.interests)?;
    push_jsonl_records(&mut lines, "community", &dataset.communities)?;
    push_jsonl_records(&mut lines, "post", &dataset.posts)?;
    push_jsonl_records(&mut lines, "comment", &dataset.comments)?;
    push_jsonl_records(&mut lines, "relationship", &dataset.relationships)?;
    push_jsonl_records(&mut lines, "activity_event", &dataset.activity_events)?;

    Ok(lines.join("\n"))
}

fn push_jsonl_records<T: Serialize>(
    lines: &mut Vec<String>,
    entity_type: &str,
    records: &[T],
) -> Result<(), ExportError> {
    for record in records {
        lines.push(serde_json::to_string(&json!({
            "type": entity_type,
            "data": record,
        }))?);
    }

    Ok(())
}

fn export_csv(dataset: &ForumDataset) -> Result<String, ExportError> {
    let mut output = String::new();

    push_csv_section(
        &mut output,
        "users",
        &[
            "id",
            "username",
            "display_name",
            "locale",
            "bio",
            "status",
            "profile_id",
            "persona_id",
            "interest_ids",
            "community_ids",
            "organization_ids",
            "created_at",
        ],
        dataset.users.iter().map(user_csv_row),
    )?;
    push_csv_section(
        &mut output,
        "personas",
        &[
            "id",
            "user_id",
            "summary",
            "traits",
            "openness",
            "extroversion",
            "conscientiousness",
            "agreeableness",
            "neuroticism",
            "posting_frequency",
            "controversy_affinity",
            "humor_affinity",
            "technical_depth",
            "meme_affinity",
            "verbosity",
            "sleep_phase",
            "activity_pattern",
            "created_at",
        ],
        dataset.personas.iter().map(persona_csv_row),
    )?;
    push_csv_section(
        &mut output,
        "interests",
        &["id", "label", "category"],
        dataset.interests.iter().map(interest_csv_row),
    )?;
    push_csv_section(
        &mut output,
        "communities",
        &[
            "id",
            "name",
            "description",
            "owner_id",
            "organization_id",
            "created_at",
        ],
        dataset.communities.iter().map(community_csv_row),
    )?;
    push_csv_section(
        &mut output,
        "posts",
        &[
            "id",
            "author_id",
            "community_id",
            "title",
            "body",
            "created_at",
            "updated_at",
        ],
        dataset.posts.iter().map(post_csv_row),
    )?;
    push_csv_section(
        &mut output,
        "comments",
        &[
            "id",
            "post_id",
            "author_id",
            "parent_comment_id",
            "body",
            "created_at",
            "updated_at",
        ],
        dataset.comments.iter().map(comment_csv_row),
    )?;
    push_csv_section(
        &mut output,
        "relationships",
        &[
            "id",
            "source_type",
            "source_id",
            "target_type",
            "target_id",
            "kind",
            "created_at",
        ],
        dataset.relationships.iter().map(relationship_csv_row),
    )?;
    push_csv_section(
        &mut output,
        "activity_events",
        &[
            "id",
            "kind",
            "actor_id",
            "object_type",
            "object_id",
            "occurred_at",
        ],
        dataset.activity_events.iter().map(activity_event_csv_row),
    )?;

    Ok(output)
}

fn push_csv_section<I>(
    output: &mut String,
    name: &str,
    headers: &[&str],
    rows: I,
) -> Result<(), ExportError>
where
    I: IntoIterator<Item = Result<Vec<String>, ExportError>>,
{
    if !output.is_empty() {
        output.push('\n');
    }

    output.push('[');
    output.push_str(name);
    output.push_str("]\n");
    output.push_str(&headers.join(","));
    output.push('\n');

    for row in rows {
        output.push_str(
            &row?
                .into_iter()
                .map(|field| csv_escape(&field))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }

    Ok(())
}

fn user_csv_row(user: &User) -> Result<Vec<String>, ExportError> {
    Ok(vec![
        user.id.to_string(),
        user.username.clone(),
        user.display_name.clone(),
        user.locale.to_string(),
        optional_string(&user.bio),
        optional_string(&user.status),
        user.profile_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        user.persona_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        json_string(&user.interest_ids)?,
        json_string(&user.community_ids)?,
        json_string(&user.organization_ids)?,
        user.created_at.to_string(),
    ])
}

fn persona_csv_row(persona: &Persona) -> Result<Vec<String>, ExportError> {
    Ok(vec![
        persona.id.to_string(),
        persona.user_id.to_string(),
        persona.summary.clone(),
        json_string(&persona.traits)?,
        score_string(persona.openness),
        score_string(persona.extroversion),
        score_string(persona.conscientiousness),
        score_string(persona.agreeableness),
        score_string(persona.neuroticism),
        score_string(persona.posting_frequency),
        score_string(persona.controversy_affinity),
        score_string(persona.humor_affinity),
        score_string(persona.technical_depth),
        score_string(persona.meme_affinity),
        json_scalar(&persona.verbosity)?,
        json_scalar(&persona.sleep_phase)?,
        json_scalar(&persona.activity_pattern)?,
        persona.created_at.to_string(),
    ])
}

fn interest_csv_row(interest: &Interest) -> Result<Vec<String>, ExportError> {
    Ok(vec![
        interest.id.to_string(),
        interest.label.clone(),
        optional_string(&interest.category),
    ])
}

fn community_csv_row(community: &Community) -> Result<Vec<String>, ExportError> {
    Ok(vec![
        community.id.to_string(),
        community.name.clone(),
        optional_string(&community.description),
        community
            .owner_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        community
            .organization_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        community.created_at.to_string(),
    ])
}

fn post_csv_row(post: &Post) -> Result<Vec<String>, ExportError> {
    Ok(vec![
        post.id.to_string(),
        post.author_id.to_string(),
        post.community_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        optional_string(&post.title),
        post.body.clone(),
        post.created_at.to_string(),
        post.updated_at
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
    ])
}

fn comment_csv_row(comment: &Comment) -> Result<Vec<String>, ExportError> {
    Ok(vec![
        comment.id.to_string(),
        comment.post_id.to_string(),
        comment.author_id.to_string(),
        comment
            .parent_comment_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        comment.body.clone(),
        comment.created_at.to_string(),
        comment
            .updated_at
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
    ])
}

fn relationship_csv_row(relationship: &Relationship) -> Result<Vec<String>, ExportError> {
    let (source_type, source_id) = relationship_endpoint_parts(&relationship.source);
    let (target_type, target_id) = relationship_endpoint_parts(&relationship.target);

    Ok(vec![
        relationship.id.to_string(),
        source_type.to_string(),
        source_id,
        target_type.to_string(),
        target_id,
        json_scalar(&relationship.kind)?,
        relationship.created_at.to_string(),
    ])
}

fn activity_event_csv_row(event: &ActivityEvent) -> Result<Vec<String>, ExportError> {
    let (object_type, object_id) = activity_object_parts(&event.object);

    Ok(vec![
        event.id.to_string(),
        json_scalar(&event.kind)?,
        event
            .actor_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        object_type.to_string(),
        object_id,
        event.occurred_at.to_string(),
    ])
}

fn export_sqlite_sql(dataset: &ForumDataset) -> Result<String, ExportError> {
    let mut output = String::from("BEGIN TRANSACTION;\n");
    push_sqlite_tables(&mut output);
    push_sql_rows(&mut output, dataset, SqlDialect::Sqlite)?;
    output.push_str("COMMIT;\n");
    Ok(output)
}

fn export_postgres_sql(dataset: &ForumDataset) -> Result<String, ExportError> {
    let mut output = String::from("BEGIN;\n");
    push_postgres_tables(&mut output);
    push_sql_rows(&mut output, dataset, SqlDialect::Postgres)?;
    output.push_str("COMMIT;\n");
    Ok(output)
}

fn push_sqlite_tables(output: &mut String) {
    output.push_str(
        "CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY, username TEXT NOT NULL, display_name TEXT NOT NULL, locale TEXT NOT NULL, bio TEXT, status TEXT, profile_id TEXT, persona_id TEXT, interest_ids TEXT NOT NULL, community_ids TEXT NOT NULL, organization_ids TEXT NOT NULL, created_at TEXT NOT NULL);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS personas (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, summary TEXT NOT NULL, traits TEXT NOT NULL, openness REAL NOT NULL, extroversion REAL NOT NULL, conscientiousness REAL NOT NULL, agreeableness REAL NOT NULL, neuroticism REAL NOT NULL, posting_frequency REAL NOT NULL, controversy_affinity REAL NOT NULL, humor_affinity REAL NOT NULL, technical_depth REAL NOT NULL, meme_affinity REAL NOT NULL, verbosity TEXT NOT NULL, sleep_phase TEXT NOT NULL, activity_pattern TEXT NOT NULL, created_at TEXT NOT NULL);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS interests (id TEXT PRIMARY KEY, label TEXT NOT NULL, category TEXT);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS communities (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, owner_id TEXT, organization_id TEXT, created_at TEXT NOT NULL);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS posts (id TEXT PRIMARY KEY, author_id TEXT NOT NULL, community_id TEXT, title TEXT, body TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS comments (id TEXT PRIMARY KEY, post_id TEXT NOT NULL, author_id TEXT NOT NULL, parent_comment_id TEXT, body TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS relationships (id TEXT PRIMARY KEY, source_type TEXT NOT NULL, source_id TEXT NOT NULL, target_type TEXT NOT NULL, target_id TEXT NOT NULL, kind TEXT NOT NULL, created_at TEXT NOT NULL);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS activity_events (id TEXT PRIMARY KEY, kind TEXT NOT NULL, actor_id TEXT, object_type TEXT NOT NULL, object_id TEXT NOT NULL, occurred_at TEXT NOT NULL);\n",
    );
}

fn push_postgres_tables(output: &mut String) {
    output.push_str(
        "CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY, username TEXT NOT NULL, display_name TEXT NOT NULL, locale TEXT NOT NULL, bio TEXT, status TEXT, profile_id TEXT, persona_id TEXT, interest_ids JSONB NOT NULL, community_ids JSONB NOT NULL, organization_ids JSONB NOT NULL, created_at TEXT NOT NULL);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS personas (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, summary TEXT NOT NULL, traits JSONB NOT NULL, openness REAL NOT NULL, extroversion REAL NOT NULL, conscientiousness REAL NOT NULL, agreeableness REAL NOT NULL, neuroticism REAL NOT NULL, posting_frequency REAL NOT NULL, controversy_affinity REAL NOT NULL, humor_affinity REAL NOT NULL, technical_depth REAL NOT NULL, meme_affinity REAL NOT NULL, verbosity TEXT NOT NULL, sleep_phase TEXT NOT NULL, activity_pattern TEXT NOT NULL, created_at TEXT NOT NULL);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS interests (id TEXT PRIMARY KEY, label TEXT NOT NULL, category TEXT);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS communities (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, owner_id TEXT, organization_id TEXT, created_at TEXT NOT NULL);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS posts (id TEXT PRIMARY KEY, author_id TEXT NOT NULL, community_id TEXT, title TEXT, body TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS comments (id TEXT PRIMARY KEY, post_id TEXT NOT NULL, author_id TEXT NOT NULL, parent_comment_id TEXT, body TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS relationships (id TEXT PRIMARY KEY, source_type TEXT NOT NULL, source_id TEXT NOT NULL, target_type TEXT NOT NULL, target_id TEXT NOT NULL, kind TEXT NOT NULL, created_at TEXT NOT NULL);\n",
    );
    output.push_str(
        "CREATE TABLE IF NOT EXISTS activity_events (id TEXT PRIMARY KEY, kind TEXT NOT NULL, actor_id TEXT, object_type TEXT NOT NULL, object_id TEXT NOT NULL, occurred_at TEXT NOT NULL);\n",
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SqlDialect {
    Sqlite,
    Postgres,
}

fn push_sql_rows(
    output: &mut String,
    dataset: &ForumDataset,
    dialect: SqlDialect,
) -> Result<(), ExportError> {
    for user in &dataset.users {
        push_insert(
            output,
            "users",
            &[
                "id",
                "username",
                "display_name",
                "locale",
                "bio",
                "status",
                "profile_id",
                "persona_id",
                "interest_ids",
                "community_ids",
                "organization_ids",
                "created_at",
            ],
            &[
                SqlValue::text(user.id.to_string()),
                SqlValue::text(&user.username),
                SqlValue::text(&user.display_name),
                SqlValue::text(user.locale.to_string()),
                SqlValue::optional_text(&user.bio),
                SqlValue::optional_text(&user.status),
                SqlValue::optional_display(&user.profile_id),
                SqlValue::optional_display(&user.persona_id),
                SqlValue::json(&user.interest_ids)?,
                SqlValue::json(&user.community_ids)?,
                SqlValue::json(&user.organization_ids)?,
                SqlValue::text(user.created_at.to_string()),
            ],
            dialect,
        );
    }

    for persona in &dataset.personas {
        push_insert(
            output,
            "personas",
            &[
                "id",
                "user_id",
                "summary",
                "traits",
                "openness",
                "extroversion",
                "conscientiousness",
                "agreeableness",
                "neuroticism",
                "posting_frequency",
                "controversy_affinity",
                "humor_affinity",
                "technical_depth",
                "meme_affinity",
                "verbosity",
                "sleep_phase",
                "activity_pattern",
                "created_at",
            ],
            &[
                SqlValue::text(persona.id.to_string()),
                SqlValue::text(persona.user_id.to_string()),
                SqlValue::text(&persona.summary),
                SqlValue::json(&persona.traits)?,
                SqlValue::number(persona.openness),
                SqlValue::number(persona.extroversion),
                SqlValue::number(persona.conscientiousness),
                SqlValue::number(persona.agreeableness),
                SqlValue::number(persona.neuroticism),
                SqlValue::number(persona.posting_frequency),
                SqlValue::number(persona.controversy_affinity),
                SqlValue::number(persona.humor_affinity),
                SqlValue::number(persona.technical_depth),
                SqlValue::number(persona.meme_affinity),
                SqlValue::text(json_scalar(&persona.verbosity)?),
                SqlValue::text(json_scalar(&persona.sleep_phase)?),
                SqlValue::text(json_scalar(&persona.activity_pattern)?),
                SqlValue::text(persona.created_at.to_string()),
            ],
            dialect,
        );
    }

    for interest in &dataset.interests {
        push_insert(
            output,
            "interests",
            &["id", "label", "category"],
            &[
                SqlValue::text(interest.id.to_string()),
                SqlValue::text(&interest.label),
                SqlValue::optional_text(&interest.category),
            ],
            dialect,
        );
    }

    for community in &dataset.communities {
        push_insert(
            output,
            "communities",
            &[
                "id",
                "name",
                "description",
                "owner_id",
                "organization_id",
                "created_at",
            ],
            &[
                SqlValue::text(community.id.to_string()),
                SqlValue::text(&community.name),
                SqlValue::optional_text(&community.description),
                SqlValue::optional_display(&community.owner_id),
                SqlValue::optional_display(&community.organization_id),
                SqlValue::text(community.created_at.to_string()),
            ],
            dialect,
        );
    }

    for post in &dataset.posts {
        push_insert(
            output,
            "posts",
            &[
                "id",
                "author_id",
                "community_id",
                "title",
                "body",
                "created_at",
                "updated_at",
            ],
            &[
                SqlValue::text(post.id.to_string()),
                SqlValue::text(post.author_id.to_string()),
                SqlValue::optional_display(&post.community_id),
                SqlValue::optional_text(&post.title),
                SqlValue::text(&post.body),
                SqlValue::text(post.created_at.to_string()),
                SqlValue::optional_display(&post.updated_at),
            ],
            dialect,
        );
    }

    for comment in &dataset.comments {
        push_insert(
            output,
            "comments",
            &[
                "id",
                "post_id",
                "author_id",
                "parent_comment_id",
                "body",
                "created_at",
                "updated_at",
            ],
            &[
                SqlValue::text(comment.id.to_string()),
                SqlValue::text(comment.post_id.to_string()),
                SqlValue::text(comment.author_id.to_string()),
                SqlValue::optional_display(&comment.parent_comment_id),
                SqlValue::text(&comment.body),
                SqlValue::text(comment.created_at.to_string()),
                SqlValue::optional_display(&comment.updated_at),
            ],
            dialect,
        );
    }

    for relationship in &dataset.relationships {
        let (source_type, source_id) = relationship_endpoint_parts(&relationship.source);
        let (target_type, target_id) = relationship_endpoint_parts(&relationship.target);
        push_insert(
            output,
            "relationships",
            &[
                "id",
                "source_type",
                "source_id",
                "target_type",
                "target_id",
                "kind",
                "created_at",
            ],
            &[
                SqlValue::text(relationship.id.to_string()),
                SqlValue::text(source_type),
                SqlValue::text(source_id),
                SqlValue::text(target_type),
                SqlValue::text(target_id),
                SqlValue::text(json_scalar(&relationship.kind)?),
                SqlValue::text(relationship.created_at.to_string()),
            ],
            dialect,
        );
    }

    for event in &dataset.activity_events {
        let (object_type, object_id) = activity_object_parts(&event.object);
        push_insert(
            output,
            "activity_events",
            &[
                "id",
                "kind",
                "actor_id",
                "object_type",
                "object_id",
                "occurred_at",
            ],
            &[
                SqlValue::text(event.id.to_string()),
                SqlValue::text(json_scalar(&event.kind)?),
                SqlValue::optional_display(&event.actor_id),
                SqlValue::text(object_type),
                SqlValue::text(object_id),
                SqlValue::text(event.occurred_at.to_string()),
            ],
            dialect,
        );
    }

    Ok(())
}

enum SqlValue {
    Null,
    Text(String),
    Json(String),
    Number(String),
}

impl SqlValue {
    fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    fn optional_text(value: &Option<String>) -> Self {
        value
            .as_ref()
            .map_or(Self::Null, |value| Self::Text(value.clone()))
    }

    fn optional_display<T: ToString>(value: &Option<T>) -> Self {
        value
            .as_ref()
            .map_or(Self::Null, |value| Self::Text(value.to_string()))
    }

    fn json<T: Serialize>(value: &T) -> Result<Self, ExportError> {
        Ok(Self::Json(serde_json::to_string(value)?))
    }

    fn number(value: f32) -> Self {
        Self::Number(score_string(value))
    }
}

fn push_insert(
    output: &mut String,
    table: &str,
    columns: &[&str],
    values: &[SqlValue],
    dialect: SqlDialect,
) {
    output.push_str("INSERT INTO ");
    output.push_str(table);
    output.push_str(" (");
    output.push_str(&columns.join(", "));
    output.push_str(") VALUES (");
    output.push_str(
        &values
            .iter()
            .map(|value| sql_literal(value, dialect))
            .collect::<Vec<_>>()
            .join(", "),
    );
    output.push_str(");\n");
}

fn sql_literal(value: &SqlValue, dialect: SqlDialect) -> String {
    match value {
        SqlValue::Null => "NULL".to_string(),
        SqlValue::Text(value) => format!("'{}'", sql_escape(value)),
        SqlValue::Json(value) if dialect == SqlDialect::Postgres => {
            format!("'{}'::jsonb", sql_escape(value))
        }
        SqlValue::Json(value) => format!("'{}'", sql_escape(value)),
        SqlValue::Number(value) => value.clone(),
    }
}

fn export_prisma_seed(dataset: &ForumDataset) -> Result<String, ExportError> {
    let mut output = String::from(
        "import { PrismaClient } from '@prisma/client';\n\nconst prisma = new PrismaClient();\n\nasync function main() {\n",
    );

    push_prisma_create_many(&mut output, "user", users_json(dataset)?)?;
    push_prisma_create_many(&mut output, "persona", personas_json(dataset)?)?;
    push_prisma_create_many(&mut output, "interest", interests_json(dataset)?)?;
    push_prisma_create_many(&mut output, "community", communities_json(dataset)?)?;
    push_prisma_create_many(&mut output, "post", posts_json(dataset)?)?;
    push_prisma_create_many(&mut output, "comment", comments_json(dataset)?)?;
    push_prisma_create_many(&mut output, "relationship", relationships_json(dataset)?)?;
    push_prisma_create_many(&mut output, "activityEvent", activity_events_json(dataset)?)?;

    output.push_str(
        "}\n\nmain()\n  .finally(async () => {\n    await prisma.$disconnect();\n  });\n",
    );

    Ok(output)
}

fn push_prisma_create_many(
    output: &mut String,
    model: &str,
    data: Value,
) -> Result<(), ExportError> {
    output.push_str("  await prisma.");
    output.push_str(model);
    output.push_str(".createMany({ data: ");
    output.push_str(&serde_json::to_string_pretty(&data)?.replace('\n', "\n  "));
    output.push_str(" });\n");
    Ok(())
}

fn users_json(dataset: &ForumDataset) -> Result<Value, ExportError> {
    Ok(Value::Array(
        dataset
            .users
            .iter()
            .map(|user| {
                json!({
                    "id": user.id,
                    "username": user.username,
                    "displayName": user.display_name,
                    "locale": user.locale,
                    "bio": user.bio,
                    "status": user.status,
                    "profileId": user.profile_id,
                    "personaId": user.persona_id,
                    "interestIds": user.interest_ids,
                    "communityIds": user.community_ids,
                    "organizationIds": user.organization_ids,
                    "createdAt": user.created_at,
                })
            })
            .collect(),
    ))
}

fn personas_json(dataset: &ForumDataset) -> Result<Value, ExportError> {
    Ok(Value::Array(
        dataset
            .personas
            .iter()
            .map(|persona| {
                json!({
                    "id": persona.id,
                    "userId": persona.user_id,
                    "summary": persona.summary,
                    "traits": persona.traits,
                    "openness": persona.openness,
                    "extroversion": persona.extroversion,
                    "conscientiousness": persona.conscientiousness,
                    "agreeableness": persona.agreeableness,
                    "neuroticism": persona.neuroticism,
                    "postingFrequency": persona.posting_frequency,
                    "controversyAffinity": persona.controversy_affinity,
                    "humorAffinity": persona.humor_affinity,
                    "technicalDepth": persona.technical_depth,
                    "memeAffinity": persona.meme_affinity,
                    "verbosity": persona.verbosity,
                    "sleepPhase": persona.sleep_phase,
                    "activityPattern": persona.activity_pattern,
                    "createdAt": persona.created_at,
                })
            })
            .collect(),
    ))
}

fn interests_json(dataset: &ForumDataset) -> Result<Value, ExportError> {
    Ok(Value::Array(
        dataset
            .interests
            .iter()
            .map(|interest| {
                json!({
                    "id": interest.id,
                    "label": interest.label,
                    "category": interest.category,
                })
            })
            .collect(),
    ))
}

fn communities_json(dataset: &ForumDataset) -> Result<Value, ExportError> {
    Ok(Value::Array(
        dataset
            .communities
            .iter()
            .map(|community| {
                json!({
                    "id": community.id,
                    "name": community.name,
                    "description": community.description,
                    "ownerId": community.owner_id,
                    "organizationId": community.organization_id,
                    "createdAt": community.created_at,
                })
            })
            .collect(),
    ))
}

fn posts_json(dataset: &ForumDataset) -> Result<Value, ExportError> {
    Ok(Value::Array(
        dataset
            .posts
            .iter()
            .map(|post| {
                json!({
                    "id": post.id,
                    "authorId": post.author_id,
                    "communityId": post.community_id,
                    "title": post.title,
                    "body": post.body,
                    "createdAt": post.created_at,
                    "updatedAt": post.updated_at,
                })
            })
            .collect(),
    ))
}

fn comments_json(dataset: &ForumDataset) -> Result<Value, ExportError> {
    Ok(Value::Array(
        dataset
            .comments
            .iter()
            .map(|comment| {
                json!({
                    "id": comment.id,
                    "postId": comment.post_id,
                    "authorId": comment.author_id,
                    "parentCommentId": comment.parent_comment_id,
                    "body": comment.body,
                    "createdAt": comment.created_at,
                    "updatedAt": comment.updated_at,
                })
            })
            .collect(),
    ))
}

fn relationships_json(dataset: &ForumDataset) -> Result<Value, ExportError> {
    Ok(Value::Array(
        dataset
            .relationships
            .iter()
            .map(|relationship| {
                let (source_type, source_id) = relationship_endpoint_parts(&relationship.source);
                let (target_type, target_id) = relationship_endpoint_parts(&relationship.target);
                json!({
                    "id": relationship.id,
                    "sourceType": source_type,
                    "sourceId": source_id,
                    "targetType": target_type,
                    "targetId": target_id,
                    "kind": relationship.kind,
                    "createdAt": relationship.created_at,
                })
            })
            .collect(),
    ))
}

fn activity_events_json(dataset: &ForumDataset) -> Result<Value, ExportError> {
    Ok(Value::Array(
        dataset
            .activity_events
            .iter()
            .map(|event| {
                let (object_type, object_id) = activity_object_parts(&event.object);
                json!({
                    "id": event.id,
                    "kind": event.kind,
                    "actorId": event.actor_id,
                    "objectType": object_type,
                    "objectId": object_id,
                    "occurredAt": event.occurred_at,
                })
            })
            .collect(),
    ))
}

fn optional_string(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

fn json_string<T: Serialize>(value: &T) -> Result<String, ExportError> {
    Ok(serde_json::to_string(value)?)
}

fn json_scalar<T: Serialize>(value: &T) -> Result<String, ExportError> {
    match serde_json::to_value(value)? {
        Value::String(value) => Ok(value),
        value => Ok(value.to_string()),
    }
}

fn score_string(value: f32) -> String {
    format!("{value:.3}")
}

fn relationship_endpoint_parts(endpoint: &RelationshipEndpoint) -> (&'static str, String) {
    match endpoint {
        RelationshipEndpoint::User(id) => ("user", id.to_string()),
        RelationshipEndpoint::Community(id) => ("community", id.to_string()),
        RelationshipEndpoint::Organization(id) => ("organization", id.to_string()),
    }
}

fn activity_object_parts(object: &ActivityObject) -> (&'static str, String) {
    match object {
        ActivityObject::User(id) => ("user", id.to_string()),
        ActivityObject::Profile(id) => ("profile", id.to_string()),
        ActivityObject::Username(id) => ("username", id.to_string()),
        ActivityObject::Persona(id) => ("persona", id.to_string()),
        ActivityObject::Interest(id) => ("interest", id.to_string()),
        ActivityObject::Status(id) => ("status", id.to_string()),
        ActivityObject::Post(id) => ("post", id.to_string()),
        ActivityObject::Comment(id) => ("comment", id.to_string()),
        ActivityObject::Reaction(id) => ("reaction", id.to_string()),
        ActivityObject::Relationship(id) => ("relationship", id.to_string()),
        ActivityObject::Community(id) => ("community", id.to_string()),
        ActivityObject::Organization(id) => ("organization", id.to_string()),
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn sql_escape(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use synthetic_pop_core::{
        ActivityEventId, ActivityEventKind, ActivityPattern, CommentId, CommunityId, Interest,
        InterestId, Locale, ModelTimestamp, Persona, PersonaId, PostId, RelationshipId,
        RelationshipKind, SleepPhase, UserId, Verbosity,
    };

    fn timestamp() -> ModelTimestamp {
        ModelTimestamp::new("2026-05-12T18:30:00Z").expect("timestamp should be valid")
    }

    fn sample_dataset() -> ForumDataset {
        let user_id = UserId::new("user-000001").expect("user ID should be valid");
        let community_id =
            CommunityId::new("community-000001").expect("community ID should be valid");
        let persona_id = PersonaId::new("persona-000001").expect("persona ID should be valid");
        let interest_id =
            InterestId::new("interest-programming").expect("interest ID should be valid");
        let post_id = PostId::new("post-000001").expect("post ID should be valid");
        let comment_id = CommentId::new("comment-000001").expect("comment ID should be valid");
        let relationship_id =
            RelationshipId::new("relationship-000001").expect("relationship ID should be valid");
        let occurred_at = timestamp();

        let mut user = User::new(
            user_id.clone(),
            "alex",
            "Alex \"A\" Morgan",
            Locale::new("en-US").expect("locale should be valid"),
            occurred_at.clone(),
        )
        .expect("user should be valid");
        user.bio = Some("Builder, writer\nmentor".to_string());
        user.persona_id = Some(persona_id.clone());
        user.interest_ids = vec![interest_id.clone()];
        user.community_ids = vec![community_id.clone()];

        let persona = Persona {
            id: persona_id,
            user_id: user_id.clone(),
            summary: "balanced technical regular contributor".to_string(),
            traits: vec!["technical".to_string(), "supportive".to_string()],
            openness: 0.61,
            extroversion: 0.42,
            conscientiousness: 0.77,
            agreeableness: 0.68,
            neuroticism: 0.18,
            posting_frequency: 0.53,
            controversy_affinity: 0.09,
            humor_affinity: 0.45,
            technical_depth: 0.82,
            meme_affinity: 0.23,
            verbosity: Verbosity::Balanced,
            sleep_phase: SleepPhase::Daytime,
            activity_pattern: ActivityPattern::Regular,
            created_at: occurred_at.clone(),
        };

        let community = Community {
            id: community_id.clone(),
            name: "General, Help".to_string(),
            description: Some("Owner's picks".to_string()),
            owner_id: Some(user_id.clone()),
            organization_id: None,
            created_at: occurred_at.clone(),
        };

        let interest = Interest {
            id: interest_id,
            label: "Programming".to_string(),
            category: Some("technical".to_string()),
        };

        let post = Post {
            id: post_id.clone(),
            author_id: user_id.clone(),
            community_id: Some(community_id.clone()),
            title: Some("What's new?".to_string()),
            body: "It's deterministic".to_string(),
            created_at: occurred_at.clone(),
            updated_at: None,
        };

        let comment = Comment {
            id: comment_id.clone(),
            post_id: post_id.clone(),
            author_id: user_id.clone(),
            parent_comment_id: None,
            body: "Looks \"good\", ship it".to_string(),
            created_at: occurred_at.clone(),
            updated_at: None,
        };

        let relationship = Relationship {
            id: relationship_id.clone(),
            source: RelationshipEndpoint::User(user_id.clone()),
            target: RelationshipEndpoint::Community(community_id.clone()),
            kind: RelationshipKind::MemberOf,
            created_at: occurred_at.clone(),
        };

        let activity = ActivityEvent {
            id: ActivityEventId::new("activity-000001").expect("activity ID should be valid"),
            kind: ActivityEventKind::CommentCreated,
            actor_id: Some(user_id),
            object: ActivityObject::Comment(comment_id),
            occurred_at,
        };

        ForumDataset {
            users: vec![user],
            personas: vec![persona],
            interests: vec![interest],
            communities: vec![community],
            posts: vec![post],
            comments: vec![comment],
            relationships: vec![relationship],
            activity_events: vec![activity],
        }
    }

    #[test]
    fn export_is_claimed_as_implemented() {
        assert_eq!(current_export_state(), ExportState::Implemented);
        assert!(export_enabled());
    }

    #[test]
    fn json_and_jsonl_include_all_entity_types() {
        let dataset = sample_dataset();
        let json = export_forum_dataset(&dataset, ExportFormat::Json).expect("json export");
        let jsonl = export_forum_dataset(&dataset, ExportFormat::Jsonl).expect("jsonl export");

        assert!(json.contains("\"users\""));
        assert!(json.contains("\"personas\""));
        assert!(json.contains("\"interests\""));
        assert!(json.contains("\"activity_events\""));
        assert!(jsonl.contains("\"type\":\"user\""));
        assert!(jsonl.contains("\"type\":\"persona\""));
        assert!(jsonl.contains("\"type\":\"interest\""));
        assert!(jsonl.contains("\"type\":\"activity_event\""));
        assert_eq!(jsonl.lines().count(), 8);
    }

    #[test]
    fn csv_uses_stable_headers_and_escapes_fields() {
        let csv = export_forum_dataset(&sample_dataset(), ExportFormat::Csv).expect("csv export");

        assert!(csv.contains("[users]\nid,username,display_name,locale,bio,status,profile_id,persona_id,interest_ids,community_ids,organization_ids,created_at\n"));
        assert!(csv.contains("[personas]\nid,user_id,summary,traits,openness,extroversion,conscientiousness,agreeableness,neuroticism,posting_frequency,controversy_affinity,humor_affinity,technical_depth,meme_affinity,verbosity,sleep_phase,activity_pattern,created_at\n"));
        assert!(csv.contains("[interests]\nid,label,category\n"));
        assert!(csv.contains(
            "[relationships]\nid,source_type,source_id,target_type,target_id,kind,created_at\n"
        ));
        assert!(csv.contains("\"Alex \"\"A\"\" Morgan\""));
        assert!(csv.contains("\"Builder, writer\nmentor\""));
    }

    #[test]
    fn sql_scripts_escape_quotes_and_include_relationships_and_activity() {
        let sqlite =
            export_forum_dataset(&sample_dataset(), ExportFormat::SqliteSql).expect("sqlite sql");
        let postgres = export_forum_dataset(&sample_dataset(), ExportFormat::PostgresSql)
            .expect("postgres sql");

        assert!(sqlite.starts_with("BEGIN TRANSACTION;\n"));
        assert!(sqlite.contains("INSERT INTO relationships"));
        assert!(sqlite.contains("INSERT INTO personas"));
        assert!(sqlite.contains("INSERT INTO interests"));
        assert!(sqlite.contains("INSERT INTO activity_events"));
        assert!(sqlite.contains("'Owner''s picks'"));
        assert!(postgres.contains("'[\"community-000001\"]'::jsonb"));
    }

    #[test]
    fn prisma_seed_is_deterministic_script_text_with_escaped_strings() {
        let seed =
            export_forum_dataset(&sample_dataset(), ExportFormat::PrismaSeed).expect("seed export");

        assert!(seed.contains("const prisma = new PrismaClient();"));
        assert!(seed.contains("await prisma.persona.createMany"));
        assert!(seed.contains("await prisma.interest.createMany"));
        assert!(seed.contains("await prisma.relationship.createMany"));
        assert!(seed.contains("await prisma.activityEvent.createMany"));
        assert!(seed.contains("Alex \\\"A\\\" Morgan"));
    }
}
