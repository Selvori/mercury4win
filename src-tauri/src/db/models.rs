// mercury4win-linux/src-tauri/db/models.rs
// Rust structs mirroring the original Mercury database schema

use serde::{Deserialize, Serialize};

// ── Feed ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    pub id: i64,
    pub title: Option<String>,
    pub feed_url: String,
    pub site_url: Option<String>,
    pub feed_parser_version: Option<i32>,
    pub last_fetched_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedWithCounts {
    pub id: i64,
    pub title: Option<String>,
    pub feed_url: String,
    pub site_url: Option<String>,
    pub last_fetched_at: Option<String>,
    pub created_at: String,
    pub unread_count: i64,
    pub total_count: i64,
}

// ── Entry ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: i64,
    pub feed_id: i64,
    pub guid: Option<String>,
    pub url: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub summary: Option<String>,
    pub is_read: bool,
    pub is_starred: bool,
    pub is_deleted: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPage {
    pub entries: Vec<Entry>,
    pub next_cursor: Option<String>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryDetail {
    pub entry: Entry,
    pub feed_title: Option<String>,
    pub tags: Vec<EntryTagInfo>,
    pub has_note: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryTagInfo {
    pub tag_id: i64,
    pub name: String,
    pub source: String,
}

// ── Content ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub entry_id: i64,
    pub html: Option<String>,
    pub cleaned_html: Option<String>,
    pub readability_title: Option<String>,
    pub readability_byline: Option<String>,
    pub readability_version: Option<i32>,
    pub markdown: Option<String>,
    pub markdown_version: Option<i32>,
    pub display_mode: String,
    pub document_base_url: Option<String>,
    pub pipeline_type: String,
    pub resolved_intermediate_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedReaderHtml {
    pub theme_id: String,
    pub entry_id: i64,
    pub html: String,
    pub reader_render_version: Option<i32>,
    pub updated_at: String,
}

// ── Tag ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub normalized_name: String,
    pub is_provisional: bool,
    pub usage_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagAlias {
    pub id: i64,
    pub tag_id: i64,
    pub alias_normalized: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryTag {
    pub entry_id: i64,
    pub tag_id: i64,
    pub source: String,
    pub confidence: Option<f64>,
}

// ── Entry Note ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryNote {
    pub entry_id: i64,
    pub markdown: String,
    pub updated_at: String,
}

// ── Agent Provider ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProviderProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub display_name: Option<String>,
    pub is_default: bool,
    pub is_enabled: bool,
    pub is_archived: bool,
    pub created_at: String,
}

// ── Agent Model ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentModelProfile {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub model_name: String,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<i64>,
    pub is_streaming: bool,
    pub supports_summary: bool,
    pub supports_translation: bool,
    pub supports_tagging: bool,
    pub is_default: bool,
    pub is_enabled: bool,
    pub is_archived: bool,
    pub last_tested_at: Option<String>,
}

// ── Agent Profile ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub agent_type: String,
    pub primary_model_id: Option<String>,
    pub fallback_model_id: Option<String>,
    pub target_language: Option<String>,
    pub detail_level: Option<String>,
    pub prompt_strategy: Option<String>,
    pub concurrency_degree: Option<i32>,
}

// ── Agent Task Run ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskRun {
    pub id: i64,
    pub entry_id: i64,
    pub task_type: String,
    pub status: String,
    pub agent_profile_id: Option<String>,
    pub provider_profile_id: Option<String>,
    pub model_profile_id: Option<String>,
    pub target_language: Option<String>,
    pub duration_ms: Option<i64>,
    pub created_at: String,
}

// ── LLM Usage Event ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUsageEvent {
    pub id: i64,
    pub task_run_id: Option<i64>,
    pub provider_name: Option<String>,
    pub model_name: Option<String>,
    pub agent_type: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub request_status: Option<String>,
    pub created_at: String,
}

// ── Translation ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub entry_id: i64,
    pub target_language: String,
    pub source_content_hash: String,
    pub segmenter_version: String,
    pub run_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationSegment {
    pub id: i64,
    pub entry_id: i64,
    pub target_language: String,
    pub source_segment_id: String,
    pub order_index: i64,
    pub source_text_snapshot: String,
    pub translated_text: Option<String>,
}

// ── Summary ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResult {
    pub entry_id: i64,
    pub target_language: String,
    pub detail_level: String,
    pub text: String,
    pub output_language: Option<String>,
    pub created_at: String,
}

// ── Content extraction result ──

#[derive(Debug, Clone)]
pub struct DecruftOutput {
    pub content: String,
    pub title: Option<String>,
    pub byline: Option<String>,
    pub site_name: Option<String>,
    pub language: Option<String>,
}

impl DecruftOutput {
    /// Check if the extraction result needs a Readability.js fallback.
    pub fn needs_fallback(&self) -> bool {
        self.content.is_empty()
            || self.content.len() < 200
            || self.title.as_ref().map_or(true, |t| t.is_empty())
    }
}

// ── Sync result ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub feed_id: i64,
    pub new_entries: usize,
    pub updated_entries: usize,
    pub error: Option<String>,
}

// ── Import/Export ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub feeds_added: usize,
    pub feeds_skipped: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent_type: String,
    pub state: String,
    pub current_entry_id: Option<i64>,
    pub queue_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTagResult {
    pub entries_processed: usize,
    pub tags_assigned: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub display_name: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub from: String,
    pub to: String,
}
