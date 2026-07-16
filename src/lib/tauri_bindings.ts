// mercury4win-linux/src/lib/tauri_bindings.ts
// Type-safe Tauri invoke wrappers

import { invoke } from "@tauri-apps/api/core";
import type { Channel } from "@tauri-apps/api/core";
import type {
  Feed,
  FeedWithCounts,
  SyncResult,
  ImportResult,
  EntryPage,
  EntryDetail,
  EntryNote,
  Tag,
  BatchTagResult,
  TimeRange,
  ProviderProfile,
  AgentStatus,
  ProviderConfig,
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

// ── Entry ──

export interface LoadEntriesParams {
  feed_id?: number;
  unread_only?: boolean;
  search?: string;
  tag_ids?: number[];
  tag_mode?: string;
  cursor?: string;
  limit?: number;
}

export async function load_entries(
  params: LoadEntriesParams,
): Promise<EntryPage> {
  return invoke("load_entries", { ...params });
}

export async function mark_read(
  entry_ids: number[],
  is_read: boolean,
): Promise<void> {
  return invoke("mark_read", { entryIds: entry_ids, isRead: is_read });
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
): Promise<string> {
  return invoke("build_reader_content", { entryId: entry_id });
}

export async function get_cached_reader_content(
  entry_id: number,
): Promise<ReaderContent | null> {
  return invoke("get_cached_reader_content", { entryId: entry_id });
}

// ── Agent ──

export async function list_providers(): Promise<ProviderProfile[]> {
  return invoke("list_providers");
}

export async function add_provider(
  config: ProviderConfig,
): Promise<ProviderProfile> {
  return invoke("add_provider", { config });
}

export async function delete_provider(id: string): Promise<void> {
  return invoke("delete_provider", { id });
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
): Promise<void> {
  return invoke("run_summary", {
    entryId: entry_id,
    targetLanguage: target_language,
    detailLevel: detail_level,
    onEvent: on_event,
  });
}

export async function run_translation(
  entry_id: number,
  target_language: string,
  on_event: Channel<string>,
): Promise<void> {
  return invoke("run_translation", {
    entryId: entry_id,
    targetLanguage: target_language,
    onEvent: on_event,
  });
}

export async function run_tagging(entry_id: number): Promise<string[]> {
  return invoke("run_tagging", { entryId: entry_id });
}

export async function cancel_agent_task(task_type: string): Promise<void> {
  return invoke("cancel_agent_task", { taskType: task_type });
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

export async function batch_tag(
  time_range: TimeRange,
): Promise<BatchTagResult> {
  return invoke("batch_tag", { timeRange: time_range });
}

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
): Promise<string> {
  return invoke("generate_digest", {
    entryId: entry_id,
    templateName: template_name,
  });
}

export async function export_digest(
  entry_id: number,
  template_name: string,
  export_path: string,
): Promise<void> {
  return invoke("export_digest", {
    entryId: entry_id,
    templateName: template_name,
    exportPath: export_path,
  });
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

export async function get_usage_report(
  window: string,
): Promise<[number, number, number]> {
  return invoke("get_usage_report", { window });
}
