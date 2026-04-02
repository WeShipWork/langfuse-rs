//! Media types for the Langfuse SDK.

use std::path::Path;

use langfuse_core::error::LangfuseError;

/// A media object that can be uploaded to Langfuse.
#[derive(Debug, Clone)]
pub struct LangfuseMedia {
    /// MIME content type (e.g., `"image/png"`).
    pub content_type: String,
    /// Raw binary data.
    pub data: Vec<u8>,
}

impl LangfuseMedia {
    /// Create from a base64-encoded data URI (e.g., `"data:image/png;base64,..."`).
    pub fn from_data_uri(data_uri: &str) -> Result<Self, LangfuseError> {
        // Parse the data URI format: data:<content_type>;base64,<data>
        let parts: Vec<&str> = data_uri.splitn(2, ',').collect();
        if parts.len() != 2 {
            return Err(LangfuseError::Media("Invalid data URI".into()));
        }
        let header = parts[0]; // "data:image/png;base64"
        let base64_data = parts[1];

        let content_type = header
            .strip_prefix("data:")
            .and_then(|s| s.strip_suffix(";base64"))
            .ok_or_else(|| LangfuseError::Media("Invalid data URI format".into()))?;

        use base64::Engine as _;
        let data = base64::engine::general_purpose::STANDARD
            .decode(base64_data)
            .map_err(|e| LangfuseError::Media(format!("Base64 decode error: {e}")))?;

        Ok(Self {
            content_type: content_type.to_string(),
            data,
        })
    }

    /// Create from raw bytes.
    pub fn from_bytes(content_type: &str, data: Vec<u8>) -> Self {
        Self {
            content_type: content_type.to_string(),
            data,
        }
    }

    /// Create from a file path.
    pub fn from_file(content_type: &str, path: impl AsRef<Path>) -> Result<Self, LangfuseError> {
        let data = std::fs::read(path.as_ref())
            .map_err(|e| LangfuseError::Media(format!("File read error: {e}")))?;
        Ok(Self {
            content_type: content_type.to_string(),
            data,
        })
    }

    /// Create from a file path asynchronously.
    ///
    /// # Errors
    ///
    /// Returns [`LangfuseError::Media`] if the file cannot be read.
    pub async fn from_file_async(
        content_type: &str,
        path: impl AsRef<Path>,
    ) -> Result<Self, LangfuseError> {
        let data = tokio::fs::read(path.as_ref())
            .await
            .map_err(|e| LangfuseError::Media(format!("File read error: {e}")))?;
        Ok(Self {
            content_type: content_type.to_string(),
            data,
        })
    }

    /// Size in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Regex pattern for Langfuse media reference tokens.
/// Format: `@@@langfuseMedia:type=<content_type>|id=<media_id>|source=<source>@@@`
pub const MEDIA_REFERENCE_PATTERN: &str =
    r"@@@langfuseMedia:type=([^|]+)\|id=([^|]+)\|source=([^@]+)@@@";

/// A parsed media reference extracted from text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMediaReference {
    /// MIME content type.
    pub content_type: String,
    /// Langfuse media ID.
    pub media_id: String,
    /// Source descriptor (e.g., `"base64_data_uri"`).
    pub source: String,
}

/// Parse media reference tokens from a string.
///
/// Finds all occurrences of `@@@langfuseMedia:type=...|id=...|source=...@@@`
/// and returns the parsed components.
pub fn parse_media_references(text: &str) -> Vec<ParsedMediaReference> {
    let mut refs = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("@@@langfuseMedia:") {
        let after = &remaining[start + 17..]; // skip "@@@langfuseMedia:"
        if let Some(end) = after.find("@@@") {
            let inner = &after[..end];
            // Parse type=X|id=Y|source=Z
            let mut content_type = None;
            let mut media_id = None;
            let mut source = None;
            for part in inner.split('|') {
                if let Some(val) = part.strip_prefix("type=") {
                    content_type = Some(val.to_string());
                } else if let Some(val) = part.strip_prefix("id=") {
                    media_id = Some(val.to_string());
                } else if let Some(val) = part.strip_prefix("source=") {
                    source = Some(val.to_string());
                }
            }
            if let (Some(ct), Some(id), Some(src)) = (content_type, media_id, source) {
                refs.push(ParsedMediaReference {
                    content_type: ct,
                    media_id: id,
                    source: src,
                });
            }
            remaining = &after[end + 3..];
        } else {
            break;
        }
    }
    refs
}
