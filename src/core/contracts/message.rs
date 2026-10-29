//! Actual normalized message, recursive segment and media output contracts.
use super::scalar::{No, Yes};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Message {
    #[schemars(regex(pattern = "^m1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$"))]
    pub message_ref: String,
    #[schemars(regex(pattern = "^-?[0-9]+$"))]
    pub id: String,
    #[schemars(regex(pattern = "^(private|group):[1-9][0-9]{0,31}$"))]
    pub conversation: String,
    pub sender: Sender,
    #[schemars(range(min = 0, max = 253402300799_i64))]
    pub time_seconds: i64,
    #[schemars(length(max = 1000))]
    pub segments: Vec<Segment>,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sender {
    #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Segment {
    Known(KnownSegment),
    Unsupported {
        #[serde(rename = "type")]
        kind: UnsupportedKind,
        supported: No,
    },
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum KnownSegment {
    Text {
        text: String,
    },
    Mention {
        #[serde(flatten)]
        target: Mention,
    },
    Reply {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            with = "String",
            regex(pattern = "^m1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$")
        )]
        message_ref: Option<String>,
    },
    Image {
        #[serde(flatten)]
        media: Media,
    },
    Record {
        #[serde(flatten)]
        media: Media,
    },
    File {
        #[serde(flatten)]
        media: Media,
    },
    Forward {
        expanded: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Vec<Node>", length(max = 100))]
        nodes: Option<Vec<Node>>,
        #[serde(skip)]
        resource_id: Option<String>,
    },
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Mention {
    User {
        #[serde(rename = "userId")]
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        user_id: String,
    },
    Unsupported {
        supported: No,
    },
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Media {
    #[serde(flatten)]
    pub availability: Availability,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub size_bytes: Option<u64>,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    #[schemars(length(max = 1000))]
    pub segments: Vec<Segment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String", regex(pattern = "^-?[0-9]+$"))]
    pub sender_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
}
impl Segment {
    pub fn media(&self) -> Option<&Media> {
        match self {
            Self::Known(
                KnownSegment::Image { media }
                | KnownSegment::File { media }
                | KnownSegment::Record { media },
            ) => Some(media),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Availability {
    Available {
        downloadable: Yes,
        #[serde(rename = "mediaRef")]
        #[schemars(regex(pattern = "^f1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$"))]
        media_ref: String,
    },
    Unavailable {
        downloadable: No,
        #[serde(rename = "mediaRef", skip_serializing_if = "Option::is_none")]
        #[schemars(
            with = "String",
            regex(pattern = "^f1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$")
        )]
        media_ref: Option<String>,
    },
}
impl Media {
    pub fn media_ref(&self) -> Option<&str> {
        match &self.availability {
            Availability::Available { media_ref, .. } => Some(media_ref),
            Availability::Unavailable { media_ref, .. } => media_ref.as_deref(),
        }
    }
    pub fn reference(&mut self, media_ref: String, downloadable: bool) {
        self.availability = if downloadable {
            Availability::Available {
                downloadable: Yes,
                media_ref,
            }
        } else {
            Availability::Unavailable {
                downloadable: No,
                media_ref: Some(media_ref),
            }
        };
    }
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct UnsupportedKind(
    #[schemars(extend("not" = {"enum":["text","mention","reply","image","file","record","forward"]}))]
     String,
);
impl UnsupportedKind {
    pub fn new(value: &str) -> Self {
        Self(value.into())
    }
}
