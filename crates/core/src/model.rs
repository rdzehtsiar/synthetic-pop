use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelValidationError {
    EmptyValue {
        value_type: &'static str,
    },
    EmptyField {
        entity: &'static str,
        field: &'static str,
    },
}

impl fmt::Display for ModelValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { value_type } => {
                write!(formatter, "{value_type} must not be empty")
            }
            Self::EmptyField { entity, field } => {
                write!(formatter, "{entity}.{field} must not be empty")
            }
        }
    }
}

impl std::error::Error for ModelValidationError {}

fn validate_non_empty(
    value_type: &'static str,
    value: String,
) -> Result<String, ModelValidationError> {
    if value.trim().is_empty() {
        return Err(ModelValidationError::EmptyValue { value_type });
    }

    Ok(value)
}

fn validate_required_field(
    entity: &'static str,
    field: &'static str,
    value: String,
) -> Result<String, ModelValidationError> {
    if value.trim().is_empty() {
        return Err(ModelValidationError::EmptyField { entity, field });
    }

    Ok(value)
}

macro_rules! non_empty_string_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ModelValidationError> {
                validate_non_empty(stringify!($name), value.into()).map(Self)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = ModelValidationError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = ModelValidationError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

non_empty_string_type!(UserId);
non_empty_string_type!(ProfileId);
non_empty_string_type!(UsernameId);
non_empty_string_type!(PersonaId);
non_empty_string_type!(InterestId);
non_empty_string_type!(StatusId);
non_empty_string_type!(PostId);
non_empty_string_type!(CommentId);
non_empty_string_type!(ReactionId);
non_empty_string_type!(RelationshipId);
non_empty_string_type!(CommunityId);
non_empty_string_type!(OrganizationId);
non_empty_string_type!(ActivityEventId);
non_empty_string_type!(Locale);
non_empty_string_type!(ModelTimestamp);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    Follows,
    Friend,
    Blocks,
    MemberOf,
    WorksAt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionKind {
    Like,
    Dislike,
    Love,
    Laugh,
    Angry,
    Bookmark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEventKind {
    UserCreated,
    PersonaCreated,
    ProfileUpdated,
    StatusChanged,
    PostCreated,
    CommentCreated,
    ReactionCreated,
    RelationshipCreated,
    CommunityJoined,
    OrganizationJoined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
    Terse,
    Balanced,
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SleepPhase {
    EarlyBird,
    Daytime,
    NightOwl,
    Irregular,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityPattern {
    Lurker,
    Casual,
    Regular,
    Bursty,
    PowerUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub display_name: String,
    pub locale: Locale,
    pub bio: Option<String>,
    pub status: Option<String>,
    pub profile_id: Option<ProfileId>,
    pub persona_id: Option<PersonaId>,
    pub interest_ids: Vec<InterestId>,
    pub community_ids: Vec<CommunityId>,
    pub organization_ids: Vec<OrganizationId>,
    pub created_at: ModelTimestamp,
}

impl User {
    pub fn new(
        id: UserId,
        username: impl Into<String>,
        display_name: impl Into<String>,
        locale: Locale,
        created_at: ModelTimestamp,
    ) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            username: validate_required_field("User", "username", username.into())?,
            display_name: validate_required_field("User", "display_name", display_name.into())?,
            locale,
            bio: None,
            status: None,
            profile_id: None,
            persona_id: None,
            interest_ids: Vec::new(),
            community_ids: Vec::new(),
            organization_ids: Vec::new(),
            created_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub id: ProfileId,
    pub user_id: UserId,
    pub display_name: String,
    pub locale: Locale,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub updated_at: ModelTimestamp,
}

impl Profile {
    pub fn new(
        id: ProfileId,
        user_id: UserId,
        display_name: impl Into<String>,
        locale: Locale,
        updated_at: ModelTimestamp,
    ) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            user_id,
            display_name: validate_required_field("Profile", "display_name", display_name.into())?,
            locale,
            bio: None,
            avatar_url: None,
            updated_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Username {
    pub id: UsernameId,
    pub user_id: UserId,
    pub value: String,
    pub created_at: ModelTimestamp,
    pub retired_at: Option<ModelTimestamp>,
}

impl Username {
    pub fn new(
        id: UsernameId,
        user_id: UserId,
        value: impl Into<String>,
        created_at: ModelTimestamp,
    ) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            user_id,
            value: validate_required_field("Username", "value", value.into())?,
            created_at,
            retired_at: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Persona {
    pub id: PersonaId,
    pub user_id: UserId,
    pub summary: String,
    pub traits: Vec<String>,
    pub openness: f32,
    pub extroversion: f32,
    pub conscientiousness: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
    pub posting_frequency: f32,
    pub controversy_affinity: f32,
    pub humor_affinity: f32,
    pub technical_depth: f32,
    pub meme_affinity: f32,
    pub verbosity: Verbosity,
    pub sleep_phase: SleepPhase,
    pub activity_pattern: ActivityPattern,
    pub created_at: ModelTimestamp,
}

impl Persona {
    pub fn new(
        id: PersonaId,
        user_id: UserId,
        summary: impl Into<String>,
        created_at: ModelTimestamp,
    ) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            user_id,
            summary: validate_required_field("Persona", "summary", summary.into())?,
            traits: Vec::new(),
            openness: 0.5,
            extroversion: 0.5,
            conscientiousness: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
            posting_frequency: 0.5,
            controversy_affinity: 0.5,
            humor_affinity: 0.5,
            technical_depth: 0.5,
            meme_affinity: 0.5,
            verbosity: Verbosity::Balanced,
            sleep_phase: SleepPhase::Daytime,
            activity_pattern: ActivityPattern::Casual,
            created_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interest {
    pub id: InterestId,
    pub label: String,
    pub category: Option<String>,
}

impl Interest {
    pub fn new(id: InterestId, label: impl Into<String>) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            label: validate_required_field("Interest", "label", label.into())?,
            category: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub id: StatusId,
    pub user_id: UserId,
    pub text: String,
    pub created_at: ModelTimestamp,
    pub expires_at: Option<ModelTimestamp>,
}

impl Status {
    pub fn new(
        id: StatusId,
        user_id: UserId,
        text: impl Into<String>,
        created_at: ModelTimestamp,
    ) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            user_id,
            text: validate_required_field("Status", "text", text.into())?,
            created_at,
            expires_at: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Post {
    pub id: PostId,
    pub author_id: UserId,
    pub community_id: Option<CommunityId>,
    pub title: Option<String>,
    pub body: String,
    pub created_at: ModelTimestamp,
    pub updated_at: Option<ModelTimestamp>,
}

impl Post {
    pub fn new(
        id: PostId,
        author_id: UserId,
        body: impl Into<String>,
        created_at: ModelTimestamp,
    ) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            author_id,
            community_id: None,
            title: None,
            body: validate_required_field("Post", "body", body.into())?,
            created_at,
            updated_at: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: CommentId,
    pub post_id: PostId,
    pub author_id: UserId,
    pub parent_comment_id: Option<CommentId>,
    pub body: String,
    pub created_at: ModelTimestamp,
    pub updated_at: Option<ModelTimestamp>,
}

impl Comment {
    pub fn new(
        id: CommentId,
        post_id: PostId,
        author_id: UserId,
        body: impl Into<String>,
        created_at: ModelTimestamp,
    ) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            post_id,
            author_id,
            parent_comment_id: None,
            body: validate_required_field("Comment", "body", body.into())?,
            created_at,
            updated_at: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum ReactionTarget {
    Post(PostId),
    Comment(CommentId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reaction {
    pub id: ReactionId,
    pub actor_id: UserId,
    pub target: ReactionTarget,
    pub kind: ReactionKind,
    pub created_at: ModelTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum RelationshipEndpoint {
    User(UserId),
    Community(CommunityId),
    Organization(OrganizationId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationship {
    pub id: RelationshipId,
    pub source: RelationshipEndpoint,
    pub target: RelationshipEndpoint,
    pub kind: RelationshipKind,
    pub created_at: ModelTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Community {
    pub id: CommunityId,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Option<UserId>,
    pub organization_id: Option<OrganizationId>,
    pub created_at: ModelTimestamp,
}

impl Community {
    pub fn new(
        id: CommunityId,
        name: impl Into<String>,
        created_at: ModelTimestamp,
    ) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            name: validate_required_field("Community", "name", name.into())?,
            description: None,
            owner_id: None,
            organization_id: None,
            created_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: ModelTimestamp,
}

impl Organization {
    pub fn new(
        id: OrganizationId,
        name: impl Into<String>,
        created_at: ModelTimestamp,
    ) -> Result<Self, ModelValidationError> {
        Ok(Self {
            id,
            name: validate_required_field("Organization", "name", name.into())?,
            description: None,
            created_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum ActivityObject {
    User(UserId),
    Profile(ProfileId),
    Username(UsernameId),
    Persona(PersonaId),
    Interest(InterestId),
    Status(StatusId),
    Post(PostId),
    Comment(CommentId),
    Reaction(ReactionId),
    Relationship(RelationshipId),
    Community(CommunityId),
    Organization(OrganizationId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub id: ActivityEventId,
    pub kind: ActivityEventKind,
    pub actor_id: Option<UserId>,
    pub object: ActivityObject,
    pub occurred_at: ModelTimestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_json_roundtrip<T>(value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
    {
        let serialized = serde_json::to_string(value).expect("value should serialize");
        let deserialized: T = serde_json::from_str(&serialized).expect("value should deserialize");

        assert_eq!(&deserialized, value);
    }

    #[test]
    fn accepts_non_empty_id_values() {
        let user_id = UserId::new("user-000001").expect("user ID should be valid");
        let post_id = PostId::new("post-000001").expect("post ID should be valid");

        assert_eq!(user_id.as_str(), "user-000001");
        assert_eq!(user_id.as_ref(), "user-000001");
        assert_eq!(user_id.to_string(), "user-000001");
        assert_eq!(post_id.as_str(), "post-000001");
    }

    #[test]
    fn accepts_non_empty_value_primitives() {
        let locale = Locale::new("en-US").expect("locale should be valid");
        let timestamp =
            ModelTimestamp::new("2026-05-12T18:30:00Z").expect("timestamp should be valid");

        assert_eq!(locale.as_str(), "en-US");
        assert_eq!(timestamp.as_str(), "2026-05-12T18:30:00Z");
    }

    #[test]
    fn rejects_empty_and_whitespace_id_values() {
        assert_eq!(
            UserId::new(""),
            Err(ModelValidationError::EmptyValue {
                value_type: "UserId"
            })
        );
        assert_eq!(
            UserId::new("   "),
            Err(ModelValidationError::EmptyValue {
                value_type: "UserId"
            })
        );
        assert_eq!(
            CommentId::new("\t\n"),
            Err(ModelValidationError::EmptyValue {
                value_type: "CommentId"
            })
        );
    }

    #[test]
    fn rejects_empty_and_whitespace_value_primitives() {
        assert_eq!(
            Locale::new(""),
            Err(ModelValidationError::EmptyValue {
                value_type: "Locale"
            })
        );
        assert_eq!(
            ModelTimestamp::new("  "),
            Err(ModelValidationError::EmptyValue {
                value_type: "ModelTimestamp"
            })
        );
    }

    #[test]
    fn parses_from_str() {
        let user_id: UserId = "user-000001".parse().expect("user ID should parse");

        assert_eq!(user_id.as_str(), "user-000001");
        assert_eq!(
            " ".parse::<UserId>(),
            Err(ModelValidationError::EmptyValue {
                value_type: "UserId"
            })
        );
    }

    #[test]
    fn serializes_and_deserializes_id_values() {
        let user_id = UserId::new("user-000001").expect("user ID should be valid");
        let serialized = serde_json::to_string(&user_id).expect("user ID should serialize");
        let deserialized: UserId =
            serde_json::from_str(&serialized).expect("user ID should deserialize");

        assert_eq!(serialized, "\"user-000001\"");
        assert_eq!(deserialized, user_id);
    }

    #[test]
    fn deserialization_rejects_invalid_id_values() {
        let error =
            serde_json::from_str::<UserId>("\"   \"").expect_err("empty user ID should fail");

        assert!(error.to_string().contains("UserId must not be empty"));
    }

    #[test]
    fn serializes_and_deserializes_value_primitives() {
        let locale = Locale::new("en-US").expect("locale should be valid");
        let serialized = serde_json::to_string(&locale).expect("locale should serialize");
        let deserialized: Locale =
            serde_json::from_str(&serialized).expect("locale should deserialize");

        assert_eq!(serialized, "\"en-US\"");
        assert_eq!(deserialized, locale);
    }

    #[test]
    fn serializes_and_deserializes_shared_enums() {
        let relationship = RelationshipKind::MemberOf;
        let reaction = ReactionKind::Bookmark;
        let activity = ActivityEventKind::RelationshipCreated;
        let verbosity = Verbosity::Detailed;
        let sleep_phase = SleepPhase::NightOwl;
        let activity_pattern = ActivityPattern::PowerUser;

        assert_eq!(
            serde_json::to_string(&relationship).expect("relationship kind should serialize"),
            "\"member_of\""
        );
        assert_eq!(
            serde_json::from_str::<ReactionKind>("\"bookmark\"")
                .expect("reaction kind should deserialize"),
            reaction
        );
        assert_eq!(
            serde_json::to_string(&activity).expect("activity kind should serialize"),
            "\"relationship_created\""
        );
        assert_eq!(
            serde_json::to_string(&verbosity).expect("verbosity should serialize"),
            "\"detailed\""
        );
        assert_eq!(
            serde_json::to_string(&sleep_phase).expect("sleep phase should serialize"),
            "\"night_owl\""
        );
        assert_eq!(
            serde_json::to_string(&activity_pattern).expect("activity pattern should serialize"),
            "\"power_user\""
        );
    }

    #[test]
    fn all_primitive_types_accept_non_empty_values() {
        UserId::new("user-1").expect("valid");
        ProfileId::new("profile-1").expect("valid");
        UsernameId::new("username-1").expect("valid");
        PersonaId::new("persona-1").expect("valid");
        InterestId::new("interest-1").expect("valid");
        StatusId::new("status-1").expect("valid");
        PostId::new("post-1").expect("valid");
        CommentId::new("comment-1").expect("valid");
        ReactionId::new("reaction-1").expect("valid");
        RelationshipId::new("relationship-1").expect("valid");
        CommunityId::new("community-1").expect("valid");
        OrganizationId::new("organization-1").expect("valid");
        ActivityEventId::new("activity-1").expect("valid");
        Locale::new("en-US").expect("valid");
        ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid");
    }

    #[test]
    fn entity_constructors_preserve_valid_required_values() {
        let timestamp = ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid");
        let user_id = UserId::new("user-000001").expect("valid");

        let user = User::new(
            user_id.clone(),
            "alex.morgan",
            "Alex Morgan",
            Locale::new("en-US").expect("valid"),
            timestamp.clone(),
        )
        .expect("user should be valid");
        let profile = Profile::new(
            ProfileId::new("profile-000001").expect("valid"),
            user_id.clone(),
            "Alex Morgan",
            Locale::new("en-US").expect("valid"),
            timestamp.clone(),
        )
        .expect("profile should be valid");
        let username = Username::new(
            UsernameId::new("username-000001").expect("valid"),
            user_id.clone(),
            "alex.morgan",
            timestamp.clone(),
        )
        .expect("username should be valid");
        let persona = Persona::new(
            PersonaId::new("persona-000001").expect("valid"),
            user_id.clone(),
            "Practical community builder",
            timestamp.clone(),
        )
        .expect("persona should be valid");
        let interest = Interest::new(
            InterestId::new("interest-photography").expect("valid"),
            "Photography",
        )
        .expect("interest should be valid");
        let status = Status::new(
            StatusId::new("status-000001").expect("valid"),
            user_id.clone(),
            "active",
            timestamp.clone(),
        )
        .expect("status should be valid");
        let post = Post::new(
            PostId::new("post-000001").expect("valid"),
            user_id.clone(),
            "Looking for a compact daily carry recommendation.",
            timestamp.clone(),
        )
        .expect("post should be valid");
        let comment = Comment::new(
            CommentId::new("comment-000001").expect("valid"),
            post.id.clone(),
            user_id,
            "Try a weather-sealed body if you travel often.",
            timestamp.clone(),
        )
        .expect("comment should be valid");
        let community = Community::new(
            CommunityId::new("community-cameras").expect("valid"),
            "Camera Club",
            timestamp.clone(),
        )
        .expect("community should be valid");
        let organization = Organization::new(
            OrganizationId::new("organization-open-labs").expect("valid"),
            "Open Labs",
            timestamp,
        )
        .expect("organization should be valid");

        assert_eq!(user.username, "alex.morgan");
        assert_eq!(user.display_name, "Alex Morgan");
        assert!(user.bio.is_none());
        assert!(user.interest_ids.is_empty());
        assert_eq!(profile.display_name, "Alex Morgan");
        assert_eq!(username.value, "alex.morgan");
        assert!(username.retired_at.is_none());
        assert_eq!(persona.summary, "Practical community builder");
        assert!(persona.traits.is_empty());
        assert_eq!(interest.label, "Photography");
        assert!(interest.category.is_none());
        assert_eq!(status.text, "active");
        assert_eq!(
            post.body,
            "Looking for a compact daily carry recommendation."
        );
        assert!(post.title.is_none());
        assert_eq!(
            comment.body,
            "Try a weather-sealed body if you travel often."
        );
        assert!(comment.parent_comment_id.is_none());
        assert_eq!(community.name, "Camera Club");
        assert!(community.description.is_none());
        assert_eq!(organization.name, "Open Labs");
        assert!(organization.description.is_none());
    }

    #[test]
    fn entity_constructors_reject_empty_required_values() {
        let timestamp = ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid");
        let user_id = UserId::new("user-000001").expect("valid");

        assert_eq!(
            User::new(
                user_id.clone(),
                "",
                "Alex Morgan",
                Locale::new("en-US").expect("valid"),
                timestamp.clone(),
            ),
            Err(ModelValidationError::EmptyField {
                entity: "User",
                field: "username"
            })
        );
        assert_eq!(
            User::new(
                user_id.clone(),
                "alex.morgan",
                "   ",
                Locale::new("en-US").expect("valid"),
                timestamp.clone(),
            ),
            Err(ModelValidationError::EmptyField {
                entity: "User",
                field: "display_name"
            })
        );
        assert_eq!(
            Profile::new(
                ProfileId::new("profile-000001").expect("valid"),
                user_id.clone(),
                "\t",
                Locale::new("en-US").expect("valid"),
                timestamp.clone(),
            ),
            Err(ModelValidationError::EmptyField {
                entity: "Profile",
                field: "display_name"
            })
        );
        assert_eq!(
            Username::new(
                UsernameId::new("username-000001").expect("valid"),
                user_id.clone(),
                "\n",
                timestamp.clone(),
            ),
            Err(ModelValidationError::EmptyField {
                entity: "Username",
                field: "value"
            })
        );
        assert_eq!(
            Persona::new(
                PersonaId::new("persona-000001").expect("valid"),
                user_id.clone(),
                "",
                timestamp.clone(),
            ),
            Err(ModelValidationError::EmptyField {
                entity: "Persona",
                field: "summary"
            })
        );
        assert_eq!(
            Interest::new(
                InterestId::new("interest-photography").expect("valid"),
                "  ",
            ),
            Err(ModelValidationError::EmptyField {
                entity: "Interest",
                field: "label"
            })
        );
        assert_eq!(
            Status::new(
                StatusId::new("status-000001").expect("valid"),
                user_id.clone(),
                "",
                timestamp.clone(),
            ),
            Err(ModelValidationError::EmptyField {
                entity: "Status",
                field: "text"
            })
        );
        assert_eq!(
            Post::new(
                PostId::new("post-000001").expect("valid"),
                user_id.clone(),
                "   ",
                timestamp.clone(),
            ),
            Err(ModelValidationError::EmptyField {
                entity: "Post",
                field: "body"
            })
        );
        assert_eq!(
            Comment::new(
                CommentId::new("comment-000001").expect("valid"),
                PostId::new("post-000001").expect("valid"),
                user_id,
                "\t\n",
                timestamp.clone(),
            ),
            Err(ModelValidationError::EmptyField {
                entity: "Comment",
                field: "body"
            })
        );
        assert_eq!(
            Community::new(
                CommunityId::new("community-cameras").expect("valid"),
                "",
                timestamp.clone(),
            ),
            Err(ModelValidationError::EmptyField {
                entity: "Community",
                field: "name"
            })
        );
        assert_eq!(
            Organization::new(
                OrganizationId::new("organization-open-labs").expect("valid"),
                " ",
                timestamp,
            ),
            Err(ModelValidationError::EmptyField {
                entity: "Organization",
                field: "name"
            })
        );
    }

    #[test]
    fn entity_validation_errors_identify_entity_and_field() {
        let error = Post::new(
            PostId::new("post-000001").expect("valid"),
            UserId::new("user-000001").expect("valid"),
            "",
            ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid"),
        )
        .expect_err("post body should be required");

        assert_eq!(error.to_string(), "Post.body must not be empty");
    }

    #[test]
    fn user_entity_represents_example_json_shape_and_roundtrips() {
        let user = User {
            id: UserId::new("user-000001").expect("valid"),
            username: "alex.morgan".to_string(),
            display_name: "Alex Morgan".to_string(),
            locale: Locale::new("en-US").expect("valid"),
            bio: Some("Community moderator and weekend photographer.".to_string()),
            status: Some("active".to_string()),
            profile_id: Some(ProfileId::new("profile-000001").expect("valid")),
            persona_id: Some(PersonaId::new("persona-000001").expect("valid")),
            interest_ids: vec![InterestId::new("interest-photography").expect("valid")],
            community_ids: vec![CommunityId::new("community-cameras").expect("valid")],
            organization_ids: vec![OrganizationId::new("organization-open-labs").expect("valid")],
            created_at: ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid"),
        };

        let serialized = serde_json::to_value(&user).expect("user should serialize");

        assert_eq!(serialized["id"], "user-000001");
        assert_eq!(serialized["username"], "alex.morgan");
        assert_eq!(serialized["display_name"], "Alex Morgan");
        assert_eq!(serialized["locale"], "en-US");
        assert_eq!(
            serialized["bio"],
            "Community moderator and weekend photographer."
        );
        assert_eq!(serialized["status"], "active");
        assert_json_roundtrip(&user);
    }

    #[test]
    fn identity_support_entities_construct_and_roundtrip() {
        let timestamp = ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid");
        let user_id = UserId::new("user-000001").expect("valid");

        let profile = Profile {
            id: ProfileId::new("profile-000001").expect("valid"),
            user_id: user_id.clone(),
            display_name: "Alex Morgan".to_string(),
            locale: Locale::new("en-US").expect("valid"),
            bio: Some("Community moderator.".to_string()),
            avatar_url: Some("https://example.test/avatar/alex.png".to_string()),
            updated_at: timestamp.clone(),
        };
        let username = Username {
            id: UsernameId::new("username-000001").expect("valid"),
            user_id: user_id.clone(),
            value: "alex.morgan".to_string(),
            created_at: timestamp.clone(),
            retired_at: None,
        };
        let persona = Persona {
            id: PersonaId::new("persona-000001").expect("valid"),
            user_id: user_id.clone(),
            summary: "Practical community builder".to_string(),
            traits: vec!["helpful".to_string(), "curious".to_string()],
            openness: 0.61,
            extroversion: 0.44,
            conscientiousness: 0.78,
            agreeableness: 0.73,
            neuroticism: 0.22,
            posting_frequency: 0.56,
            controversy_affinity: 0.12,
            humor_affinity: 0.49,
            technical_depth: 0.81,
            meme_affinity: 0.31,
            verbosity: Verbosity::Balanced,
            sleep_phase: SleepPhase::Daytime,
            activity_pattern: ActivityPattern::Regular,
            created_at: timestamp.clone(),
        };
        let interest = Interest {
            id: InterestId::new("interest-photography").expect("valid"),
            label: "Photography".to_string(),
            category: Some("creative".to_string()),
        };
        let status = Status {
            id: StatusId::new("status-000001").expect("valid"),
            user_id,
            text: "active".to_string(),
            created_at: timestamp.clone(),
            expires_at: None,
        };

        assert_json_roundtrip(&profile);
        assert_json_roundtrip(&username);
        assert_json_roundtrip(&persona);
        assert_json_roundtrip(&interest);
        assert_json_roundtrip(&status);
    }

    #[test]
    fn forum_community_relationship_and_activity_entities_roundtrip() {
        let timestamp = ModelTimestamp::new("2026-05-12T18:30:00Z").expect("valid");
        let user_id = UserId::new("user-000001").expect("valid");
        let community_id = CommunityId::new("community-cameras").expect("valid");
        let organization_id = OrganizationId::new("organization-open-labs").expect("valid");

        let organization = Organization {
            id: organization_id.clone(),
            name: "Open Labs".to_string(),
            description: Some("Local research collective".to_string()),
            created_at: timestamp.clone(),
        };
        let community = Community {
            id: community_id.clone(),
            name: "Camera Club".to_string(),
            description: Some("A community for camera discussions".to_string()),
            owner_id: Some(user_id.clone()),
            organization_id: Some(organization_id.clone()),
            created_at: timestamp.clone(),
        };
        let post = Post {
            id: PostId::new("post-000001").expect("valid"),
            author_id: user_id.clone(),
            community_id: Some(community_id.clone()),
            title: Some("Best compact camera?".to_string()),
            body: "Looking for a compact daily carry recommendation.".to_string(),
            created_at: timestamp.clone(),
            updated_at: None,
        };
        let comment = Comment {
            id: CommentId::new("comment-000001").expect("valid"),
            post_id: post.id.clone(),
            author_id: user_id.clone(),
            parent_comment_id: None,
            body: "Try a weather-sealed body if you travel often.".to_string(),
            created_at: timestamp.clone(),
            updated_at: None,
        };
        let reaction = Reaction {
            id: ReactionId::new("reaction-000001").expect("valid"),
            actor_id: user_id.clone(),
            target: ReactionTarget::Comment(comment.id.clone()),
            kind: ReactionKind::Like,
            created_at: timestamp.clone(),
        };
        let relationship = Relationship {
            id: RelationshipId::new("relationship-000001").expect("valid"),
            source: RelationshipEndpoint::User(user_id.clone()),
            target: RelationshipEndpoint::Community(community_id),
            kind: RelationshipKind::MemberOf,
            created_at: timestamp.clone(),
        };
        let activity = ActivityEvent {
            id: ActivityEventId::new("activity-000001").expect("valid"),
            kind: ActivityEventKind::ReactionCreated,
            actor_id: Some(user_id),
            object: ActivityObject::Reaction(reaction.id.clone()),
            occurred_at: timestamp,
        };

        assert_json_roundtrip(&organization);
        assert_json_roundtrip(&community);
        assert_json_roundtrip(&post);
        assert_json_roundtrip(&comment);
        assert_json_roundtrip(&reaction);
        assert_json_roundtrip(&relationship);
        assert_json_roundtrip(&activity);

        let reaction_json = serde_json::to_value(&reaction).expect("reaction should serialize");
        assert_eq!(reaction_json["target"]["type"], "comment");
        assert_eq!(reaction_json["target"]["id"], "comment-000001");
    }
}
