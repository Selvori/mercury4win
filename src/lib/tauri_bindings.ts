// mercury4win/src/lib/tauri_bindings.ts
// Type-safe Tauri invoke wrappers

import { invoke } from "@tauri-apps/api/core";
import type { Channel } from "@tauri-apps/api/core";
import type {
  Feed,
  FeedWithCounts,
  SyncResult,
  ImportResult,
  ImportProgress,
  EntryPage,
  EntryFilter,
  EntryCursor,
  EntryDetail,
  EntryNote,
  Tag,
  TagAlias,
  BatchScope,
  BatchSnapshot,
  BatchApplyResult,
  Entry,
  ProviderProfile,
  ModelProfile,
  AgentProfile,
  AgentStatus,
  AgentResult,
  ProviderConfig,
  DigestOptions,
  DigestPreview,
} from "@/types";

// ── Feed ──

export async function load_feeds(): Promise<FeedWithCounts[]> {
  return invoke("load_feeds");
}

export async function add_feed(url: string): Promise<Feed> {
  return invoke("add_feed", { url });
}

export async function update_feed(
  id: number,
  title?: string,
): Promise<Feed> {
  return invoke("update_feed", { id, title: title ?? null });
}

export async function delete_feed(id: number): Promise<void> {
  return invoke("delete_feed", { id });
}

export async function sync_feed(id: number): Promise<SyncResult> {
  return invoke("sync_feed", { id });
}

export async function sync_all_feeds(): Promise<SyncResult[]> {
  return invoke("sync_all_feeds");
}

export async function sync_all_feeds_stream(
  on_progress: Channel<ImportProgress>,
): Promise<void> {
  return invoke("sync_all_feeds_stream", { onProgress: on_progress });
}

// ── Entry ──

export interface LoadEntriesParams extends EntryFilter {
  cursor?: EntryCursor;
  limit?: number;
}

function entry_filter_args(params: EntryFilter) {
  return {
    feedId: params.feed_id ?? null,
    tagIds: params.tag_ids ?? null,
    unreadOnly: params.unread_only ?? false,
    starredOnly: params.starred_only ?? false,
    search: params.search ?? null,
    tagMode: params.tag_mode ?? "any",
  };
}

export async function load_entries(
  params: LoadEntriesParams,
): Promise<EntryPage> {
  return invoke("load_entries", {
    ...entry_filter_args(params),
    cursor: params.cursor ?? null,
    limit: params.limit ?? null,
  });
}

export async function mark_read(
  entry_ids: number[],
  is_read: boolean,
): Promise<void> {
  return invoke("mark_read", { entryIds: entry_ids, isRead: is_read });
}

export async function mark_all_read_in_scope(filter: EntryFilter): Promise<void> {
  return invoke("mark_all_read_in_scope", entry_filter_args(filter));
}

export async function mark_starred(
  entry_id: number,
  is_starred: boolean,
): Promise<void> {
  return invoke("mark_starred", { entryId: entry_id, isStarred: is_starred });
}

export async function delete_entry(entry_id: number): Promise<void> {
  return invoke("delete_entry", { entryId: entry_id });
}

export async function load_entry_detail(
  entry_id: number,
): Promise<EntryDetail> {
  return invoke("load_entry_detail", { entryId: entry_id });
}

// ── OPML ──

export async function import_opml(file_path: string): Promise<ImportResult> {
  return invoke("import_opml", { filePath: file_path });
}

export async function import_opml_stream(
  file_path: string,
  on_progress: Channel<ImportProgress>,
): Promise<void> {
  return invoke("import_opml_stream", { filePath: file_path, onProgress: on_progress });
}

export async function export_opml(
  file_path: string,
  feed_ids?: number[],
): Promise<void> {
  return invoke("export_opml", { filePath: file_path, feedIds: feed_ids ?? null });
}

// ── Reader ──

export interface ReaderContent {
  entry_id: number;
  html: string;
  title: string;
  byline: string | null;
  theme_id: string;
}

export async function build_reader_content(
  entry_id: number,
  theme_id?: string,
  font_family?: string,
  font_size?: number,
): Promise<string> {
  return invoke("build_reader_content", {
    entryId: entry_id,
    themeId: theme_id ?? null,
    fontFamily: font_family ?? null,
    fontSize: font_size ?? null,
  });
}

export async function get_cached_reader_content(
  entry_id: number,
  theme_id?: string,
  font_family?: string,
  font_size?: number,
): Promise<ReaderContent | null> {
  return invoke("get_cached_reader_content", {
    entryId: entry_id,
    themeId: theme_id ?? null,
    fontFamily: font_family ?? null,
    fontSize: font_size ?? null,
  });
}

// ── Agent ──

export async function list_providers(): Promise<ProviderProfile[]> {
  return invoke("list_providers");
}

export async function add_provider(
  config: ProviderConfig,
): Promise<ProviderProfile> {
  return invoke("add_provider", {
    config: {
      name: config.name,
      baseUrl: config.base_url,
      displayName: config.display_name ?? null,
      apiKey: config.api_key ?? null,
    },
  });
}

export async function delete_provider(id: string): Promise<void> {
  return invoke("delete_provider", { id });
}

export async function update_provider(
  id: string,
  config: ProviderConfig,
): Promise<void> {
  return invoke("update_provider", {
    id,
    config: {
      name: config.name,
      baseUrl: config.base_url,
      displayName: config.display_name ?? null,
      apiKey: config.api_key ?? null,
    },
  });
}

export async function test_provider_connection(
  provider_id: string,
  model_name: string,
): Promise<string> {
  return invoke("test_provider_connection", {
    providerId: provider_id,
    modelName: model_name,
  });
}

export async function test_provider_config(
  provider_id: string | null,
  config: ProviderConfig,
  model_name: string,
): Promise<string> {
  return invoke("test_provider_config", {
    providerId: provider_id,
    config: {
      name: config.name,
      baseUrl: config.base_url,
      displayName: config.display_name ?? null,
      apiKey: config.api_key ?? null,
    },
    modelName: model_name,
  });
}

export async function get_agent_status(
  agent_type: string,
): Promise<AgentStatus> {
  return invoke("get_agent_status", { agentType: agent_type });
}

export async function run_summary(
  entry_id: number,
  target_language: string,
  detail_level: string,
  on_event: Channel<string>,
  request_id: string = crypto.randomUUID(),
): Promise<void> {
  return invoke("run_summary", {
    entryId: entry_id,
    targetLanguage: target_language,
    detailLevel: detail_level,
    requestId: request_id,
    onEvent: on_event,
  });
}

export async function run_translation(
  entry_id: number,
  target_language: string,
  on_event: Channel<string>,
  request_id: string = crypto.randomUUID(),
  source_segment_id?: string,
): Promise<void> {
  return invoke("run_translation", {
    entryId: entry_id,
    targetLanguage: target_language,
    requestId: request_id,
    sourceSegmentId: source_segment_id ?? null,
    onEvent: on_event,
  });
}

export async function run_tagging(
  entry_id: number,
  on_event: Channel<string>,
  request_id: string = crypto.randomUUID(),
): Promise<string[]> {
  return invoke("run_tagging", { entryId: entry_id, onEvent: on_event, requestId: request_id });
}

export async function cancel_agent_task(task_type: string, request_id: string): Promise<void> {
  return invoke("cancel_agent_task", { taskType: task_type, requestId: request_id });
}

export async function abandon_agent_waiting(task_type: string, request_id: string): Promise<void> {
  return invoke("abandon_agent_waiting", { taskType: task_type, requestId: request_id });
}

// ── Model management ──

export async function list_models(
  provider_id: string,
): Promise<ModelProfile[]> {
  return invoke("list_models", { providerId: provider_id });
}

export async function add_model(
  provider_id: string,
  name: string,
  model_name: string,
  supports_summary?: boolean,
  supports_translation?: boolean,
  supports_tagging?: boolean,
): Promise<void> {
  return invoke("add_model", { providerId: provider_id, name, modelName: model_name, supportsSummary: supports_summary, supportsTranslation: supports_translation, supportsTagging: supports_tagging });
}

export async function update_model(
  id: string,
  name: string,
  model_name: string,
  supports_summary: boolean,
  supports_translation: boolean,
  supports_tagging: boolean,
): Promise<void> {
  return invoke("update_model", {
    id, name, modelName: model_name,
    supportsSummary: supports_summary,
    supportsTranslation: supports_translation,
    supportsTagging: supports_tagging,
  });
}

export async function delete_model(id: string): Promise<void> {
  return invoke("delete_model", { id });
}

// ── Agent profile ──

export async function get_agent_profile(
  agent_type: string,
): Promise<AgentProfile | null> {
  return invoke("get_agent_profile", { agentType: agent_type });
}

export async function update_agent_profile(
  agent_type: string,
  primary_model_id: string | null,
  fallback_model_id: string | null,
  target_language: string | null,
  detail_level: string | null,
  prompt_strategy: string | null,
  concurrency_degree: number | null,
): Promise<void> {
  return invoke("update_agent_profile", {
    agentType: agent_type,
    primaryModelId: primary_model_id,
    fallbackModelId: fallback_model_id,
    targetLanguage: target_language,
    detailLevel: detail_level,
    promptStrategy: prompt_strategy,
    concurrencyDegree: concurrency_degree,
  });
}

// ── Custom prompt templates ──

export interface PromptTemplateInfo {
  version: number;
  system: string | null;
  user: string;
}

export async function save_custom_template(
  agent_type: string,
  source_path: string,
): Promise<void> {
  return invoke("save_custom_template", { agentType: agent_type, sourcePath: source_path });
}

export async function load_custom_template(
  agent_type: string,
): Promise<string | null> {
  return invoke("load_custom_template", { agentType: agent_type });
}

// ── Tags ──

export async function list_tags(search?: string): Promise<Tag[]> {
  return invoke("list_tags", { search: search ?? null });
}

export async function add_tag(
  entry_id: number,
  name: string,
): Promise<Tag> {
  return invoke("add_tag", { entryId: entry_id, name });
}

export async function remove_tag(
  entry_id: number,
  tag_id: number,
): Promise<void> {
  return invoke("remove_tag", { entryId: entry_id, tagId: tag_id });
}

export async function batch_tag(scope: BatchScope, request_id: string, confirmed_large: boolean, concurrency: number, on_event: Channel<string>): Promise<BatchSnapshot> {
  return invoke("batch_tag", { scope, requestId: request_id, confirmedLarge: confirmed_large, concurrency, onEvent: on_event });
}
export const count_tag_batch = (scope: BatchScope): Promise<number> => invoke("count_tag_batch", { scope });
export const load_tag_batch = (run_id?: string): Promise<BatchSnapshot | null> => invoke("load_tag_batch", { runId: run_id ?? null });
export const apply_tag_batch = (run_id: string, kept: string[], discarded: string[]): Promise<BatchApplyResult> => invoke("apply_tag_batch", { runId: run_id, kept, discarded });
export const discard_tag_batch = (run_id: string): Promise<void> => invoke("discard_tag_batch", { runId: run_id });
export const list_tag_aliases = (tag_id: number): Promise<TagAlias[]> => invoke("list_tag_aliases", { tagId: tag_id });
export const add_tag_alias = (tag_id: number, alias: string): Promise<void> => invoke("add_tag_alias", { tagId: tag_id, alias });
export const delete_tag_alias = (alias_id: number): Promise<void> => invoke("delete_tag_alias", { aliasId: alias_id });
export const promote_tag = (tag_id: number): Promise<void> => invoke("promote_tag", { tagId: tag_id });
export const cleanup_unused_tags = (): Promise<number> => invoke("cleanup_unused_tags");
export const local_tag_suggestions = (entry_id: number): Promise<string[]> => invoke("local_tag_suggestions", { entryId: entry_id });
export const related_entries = (entry_id: number): Promise<Entry[]> => invoke("related_entries", { entryId: entry_id });

export async function rename_tag(
  tag_id: number,
  new_name: string,
): Promise<void> {
  return invoke("rename_tag", { tagId: tag_id, newName: new_name });
}

export async function delete_tag(tag_id: number): Promise<void> {
  return invoke("delete_tag", { tagId: tag_id });
}

export async function merge_tags(
  source_id: number,
  target_id: number,
): Promise<void> {
  return invoke("merge_tags", { sourceId: source_id, targetId: target_id });
}

// ── Digest ──

export async function generate_digest(
  entry_id: number,
  template_name: string,
  options?: DigestOptions,
): Promise<string> {
  return invoke("generate_digest", {
    entryId: entry_id,
    templateName: template_name,
    options,
  });
}

export async function export_digest(
  entry_id: number,
  template_name: string,
  export_path: string,
  options?: DigestOptions,
  expected_content?: string,
): Promise<void> {
  return invoke("export_digest", {
    entryId: entry_id,
    templateName: template_name,
    exportPath: export_path,
    options,
    expectedContent: expected_content,
  });
}

export function preview_digest(entry_ids: number[], template_name: string, options: DigestOptions): Promise<DigestPreview> {
  return invoke("preview_digest", { entryIds: entry_ids, templateName: template_name, options });
}

export function export_multi_digest(entry_ids: number[], template_name: string, export_path: string, options: DigestOptions, expected_content: string): Promise<void> {
  return invoke("export_multi_digest", { entryIds: entry_ids, templateName: template_name, exportPath: export_path, options, expectedContent: expected_content });
}

// ── Settings ──

export async function get_setting(key: string): Promise<string | null> {
  return invoke("get_setting", { key });
}

export async function set_setting(key: string, value: string): Promise<void> {
  return invoke("set_setting", { key, value });
}

// ── Notes ──

export async function get_note(
  entry_id: number,
): Promise<EntryNote | null> {
  return invoke("get_note", { entryId: entry_id });
}

export async function save_note(
  entry_id: number,
  markdown: string,
): Promise<void> {
  return invoke("save_note", { entryId: entry_id, markdown });
}

export async function delete_note(entry_id: number): Promise<void> {
  return invoke("delete_note", { entryId: entry_id });
}

// ── Usage ──

export async function get_usage_report(window: string, dimension = "provider", filter: import("@/types/agent").UsageFilter = {}): Promise<import("@/types/agent").UsageReport> {
  return invoke("get_usage_report", { window, dimension, filter });
}
export async function set_configuration_enabled(id: string, entity: "provider" | "model", enabled: boolean): Promise<void> {
  return invoke("set_configuration_enabled", { id, entity, enabled });
}

// Durable AI results; language/parameters/source identity is checked in Rust.
export function load_summary_result(entry_id: number, target_language: string, detail_level: string): Promise<AgentResult | null> {
  return invoke("load_summary_result", { entryId: entry_id, targetLanguage: target_language, detailLevel: detail_level });
}
export function load_translation_result(entry_id: number, target_language: string): Promise<AgentResult | null> {
  return invoke("load_translation_result", { entryId: entry_id, targetLanguage: target_language });
}

export function load_translation_document(entry_id: number, target_language: string): Promise<import("@/types/agent").TranslationDocument> {
  return invoke("load_translation_document", { entryId: entry_id, targetLanguage: target_language });
}

export function get_agent_fingerprint(agent_type: string): Promise<string> {
  return invoke("get_agent_fingerprint", { agentType: agent_type });
}
export function load_latest_summary(entry_id: number): Promise<{ target_language: string; detail_level: string; configuration_fingerprint: string | null; result: AgentResult } | null> {
  return invoke("load_latest_summary", { entryId: entry_id });
}
