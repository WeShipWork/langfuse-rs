//! Langfuse-specific OTel span attribute key constants.
//!
//! These mirror the `LangfuseOtelSpanAttributes` enum defined in the Langfuse
//! server (`packages/shared/src/server/otel/attributes.ts`) and the JS SDK
//! (`packages/core/src/constants.ts`).

// ── Observation attributes ──────────────────────────────────────────────

/// The observation type (span, generation, event, etc.)
pub const LANGFUSE_OBSERVATION_TYPE: &str = "langfuse.observation.type";
/// Observation input data (JSON-serialized).
pub const LANGFUSE_INPUT: &str = "langfuse.observation.input";
/// Observation output data (JSON-serialized).
pub const LANGFUSE_OUTPUT: &str = "langfuse.observation.output";
/// Observation metadata (JSON-serialized).
pub const LANGFUSE_METADATA: &str = "langfuse.observation.metadata";
/// Observation severity level.
pub const LANGFUSE_LEVEL: &str = "langfuse.observation.level";
/// Observation status message.
pub const LANGFUSE_STATUS_MESSAGE: &str = "langfuse.observation.status_message";

// ── Generation-specific attributes ──────────────────────────────────────

/// Model name (for generations).
pub const LANGFUSE_MODEL: &str = "langfuse.observation.model.name";
/// Model parameters (JSON-serialized).
pub const LANGFUSE_MODEL_PARAMETERS: &str = "langfuse.observation.model.parameters";
/// Usage details (JSON-serialized `UsageDetails`).
pub const LANGFUSE_USAGE: &str = "langfuse.observation.usage_details";
/// Cost details (JSON-serialized `CostDetails`).
pub const LANGFUSE_COST: &str = "langfuse.observation.cost_details";
/// Completion start time (ISO 8601).
pub const LANGFUSE_COMPLETION_START_TIME: &str = "langfuse.observation.completion_start_time";
/// Tool calls extracted from model response (JSON-serialized array).
pub const LANGFUSE_TOOL_CALLS: &str = "langfuse.observation.tool_calls";
/// Prompt name (for prompt linking).
pub const LANGFUSE_PROMPT_NAME: &str = "langfuse.observation.prompt.name";
/// Prompt version (for prompt linking).
pub const LANGFUSE_PROMPT_VERSION: &str = "langfuse.observation.prompt.version";

// ── Trace-level attributes ──────────────────────────────────────────────

/// Trace name.
pub const LANGFUSE_TRACE_NAME: &str = "langfuse.trace.name";
/// Trace-level input (JSON-serialized).
pub const LANGFUSE_TRACE_INPUT: &str = "langfuse.trace.input";
/// Trace-level output (JSON-serialized).
pub const LANGFUSE_TRACE_OUTPUT: &str = "langfuse.trace.output";
/// Trace tags (JSON-serialized array).
pub const LANGFUSE_TAGS: &str = "langfuse.trace.tags";
/// Trace public flag.
pub const LANGFUSE_TRACE_PUBLIC: &str = "langfuse.trace.public";
/// Trace-level metadata (JSON-serialized).
pub const LANGFUSE_TRACE_METADATA: &str = "langfuse.trace.metadata";

// ── Identity attributes (compat aliases — primary keys are `session.id` / `user.id`) ─

/// User ID (trace-level). Compat alias; primary is `user.id`.
pub const LANGFUSE_USER_ID: &str = "langfuse.user.id";
/// Session ID (trace-level). Compat alias; primary is `session.id`.
pub const LANGFUSE_SESSION_ID: &str = "langfuse.session.id";

// ── General attributes ──────────────────────────────────────────────────

/// Version tag.
pub const LANGFUSE_VERSION: &str = "langfuse.version";
/// Release tag.
pub const LANGFUSE_RELEASE: &str = "langfuse.release";
/// Environment tag.
pub const LANGFUSE_ENVIRONMENT: &str = "langfuse.environment";
