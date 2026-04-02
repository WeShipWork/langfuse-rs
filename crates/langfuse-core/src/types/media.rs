use serde::{Deserialize, Serialize};

/// Content type and raw bytes for media.
#[derive(Debug, Clone)]
pub struct MediaContent {
    /// MIME content type (e.g. "image/png").
    pub content_type: String,
    /// Raw bytes of the media content.
    pub data: Vec<u8>,
}

/// Reference token format used in Langfuse for inline media.
/// Format: `@@@langfuseMedia:type=<content_type>|id=<media_id>|source=<source>@@@`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaReference {
    /// Unique media identifier.
    pub media_id: String,
    /// MIME content type.
    pub content_type: String,
    /// Source identifier (e.g. "base64_data_uri").
    pub source: String,
}
