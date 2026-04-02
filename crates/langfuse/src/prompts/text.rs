//! Text prompt client with `{{variable}}` template compilation.

use std::collections::HashMap;

use langfuse_core::error::LangfuseError;

/// A compiled text prompt. Holds the raw template and supports `{{variable}}` substitution.
#[derive(Debug, Clone)]
pub struct TextPromptClient {
    /// Prompt name.
    pub name: String,
    /// Prompt version.
    pub version: i32,
    /// Raw template string containing `{{variable}}` placeholders.
    pub template: String,
    /// Arbitrary configuration attached to the prompt.
    pub config: serde_json::Value,
    /// Labels associated with this prompt version (e.g. `["production"]`).
    pub labels: Vec<String>,
    /// Tags for categorisation.
    pub tags: Vec<String>,
    /// Whether this prompt was served from an expired cache entry (fallback).
    pub is_fallback: bool,
}

impl TextPromptClient {
    /// Compile the prompt by replacing `{{variable}}` placeholders with values from `variables`.
    ///
    /// Returns [`LangfuseError::PromptCompilation`] if a placeholder references a variable
    /// that is not present in the map.
    pub fn compile(&self, variables: &HashMap<String, String>) -> Result<String, LangfuseError> {
        compile_template(&self.template, variables)
    }
}

/// Shared template compilation logic.
///
/// Scans `template` for `{{name}}` patterns and replaces each with the corresponding
/// value from `variables`. Whitespace inside the braces is trimmed so `{{ name }}` works
/// identically to `{{name}}`.
pub(crate) fn compile_template(
    template: &str,
    variables: &HashMap<String, String>,
) -> Result<String, LangfuseError> {
    let mut result = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        // Push everything before the opening braces.
        result.push_str(&rest[..start]);

        let after_open = &rest[start + 2..];
        let end = after_open
            .find("}}")
            .ok_or_else(|| LangfuseError::PromptCompilation {
                variable: "unclosed {{".into(),
            })?;

        let var_name = after_open[..end].trim();
        let value = variables
            .get(var_name)
            .ok_or_else(|| LangfuseError::PromptCompilation {
                variable: var_name.to_owned(),
            })?;

        result.push_str(value);
        rest = &after_open[end + 2..];
    }

    // Append any remaining text after the last placeholder.
    result.push_str(rest);
    Ok(result)
}
