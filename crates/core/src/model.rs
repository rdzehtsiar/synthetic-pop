use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelValidationError {
    EmptyValue { value_type: &'static str },
}

impl fmt::Display for ModelValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { value_type } => {
                write!(formatter, "{value_type} must not be empty")
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
    ProfileUpdated,
    StatusChanged,
    PostCreated,
    CommentCreated,
    ReactionCreated,
    RelationshipCreated,
    CommunityJoined,
    OrganizationJoined,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
