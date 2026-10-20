//! Resource discovery outputs, shared by normalization and schema generation.
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contact {
    #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
    pub user_id: String,
    #[schemars(regex(pattern = "^private:[1-9][0-9]{0,31}$"))]
    pub conversation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub remark: Option<String>,
}
#[derive(Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Group {
    #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
    pub group_id: String,
    #[schemars(regex(pattern = "^group:[1-9][0-9]{0,31}$"))]
    pub conversation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub remark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub member_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub max_member_count: Option<u64>,
}
#[derive(Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Member {
    #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub card: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub role: Option<String>,
}
#[derive(Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recent {
    #[schemars(regex(pattern = "^(private|group):[1-9][0-9]{0,31}$"))]
    pub conversation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub remark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    #[schemars(range(max = 253402300799_u64))]
    pub last_activity_seconds: Option<u64>,
}
#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum Row {
    Contact(Contact),
    Group(Group),
    Member(Member),
    Recent(Recent),
}
impl Row {
    pub fn conversation(&self) -> Option<&str> {
        match self {
            Self::Contact(v) => Some(&v.conversation),
            Self::Group(v) => Some(&v.conversation),
            Self::Recent(v) => Some(&v.conversation),
            Self::Member(_) => None,
        }
    }
    pub fn id(&self) -> &str {
        match self {
            Self::Contact(v) => &v.user_id,
            Self::Group(v) => &v.group_id,
            Self::Member(v) => &v.user_id,
            Self::Recent(v) => &v.conversation,
        }
    }
    pub fn matches(&self, query: &str) -> bool {
        let fields: Vec<&str> = match self {
            Self::Contact(v) => vec![
                Some(v.user_id.as_str()),
                v.nickname.as_deref(),
                v.remark.as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Self::Group(v) => vec![
                Some(v.group_id.as_str()),
                v.name.as_deref(),
                v.remark.as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Self::Member(v) => vec![
                Some(v.user_id.as_str()),
                v.nickname.as_deref(),
                v.card.as_deref(),
                v.role.as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Self::Recent(v) => vec![v.name.as_deref(), v.remark.as_deref()]
                .into_iter()
                .flatten()
                .collect(),
        };
        fields.iter().any(|f| f.to_lowercase().contains(query))
    }
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    Friends,
    Groups,
    Members,
    Recent,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum End {
    BackendEnd,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Limit {
    Limit,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum CollectionPage {
    End {
        complete: super::scalar::Yes,
        #[serde(rename = "stopReason")]
        stop_reason: End,
        coverage: Coverage,
        #[serde(rename = "nextCursor")]
        next_cursor: (),
    },
    More {
        complete: super::scalar::No,
        #[serde(rename = "stopReason")]
        stop_reason: Limit,
        coverage: Coverage,
        #[serde(rename = "nextCursor")]
        #[schemars(regex(pattern = "^c1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$"))]
        next_cursor: String,
    },
}
impl CollectionPage {
    pub fn new(coverage: Coverage, next: Option<String>) -> Self {
        match next {
            Some(next_cursor) => Self::More {
                complete: super::scalar::No,
                stop_reason: Limit::Limit,
                coverage,
                next_cursor,
            },
            None => Self::End {
                complete: super::scalar::Yes,
                stop_reason: End::BackendEnd,
                coverage,
                next_cursor: (),
            },
        }
    }
}
