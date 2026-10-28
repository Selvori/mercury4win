import type { EntryFilter } from "./entry";
import type { SearchFilterError } from "./search_filter";

export interface TopicSearchPlan {
  intent: string;
  entities: string[];
  routes: { language: string; groups: string[][] }[];
}
export interface TopicSearchSession {
  preliminary?: number;
  scan?: TopicSearchProgress | null;
  id: string;
  sentence: string;
  input_key: string;
  plan: TopicSearchPlan;
  filter: EntryFilter;
  manual: EntryFilter;
  checked: number;
  total: number;
  pending: number;
  matches: number;
  stale: boolean;
}
export interface TopicSearchRun { session: TopicSearchSession; error: SearchFilterError | null; cached: boolean }

export interface TopicSearchProgress {
  started_at: string;
  scan_id: string;
  status: "running" | "paused" | "cancelled" | "failed" | "complete";
  phase: "understanding" | "titles" | "previews" | "full" | "finalizing" | "done";
  total: number;
  title_screened: number;
  title_model_reviewed: number;
  title_fallback?: number;
  title_excluded: number;
  preview_screened: number;
  further_pending: number;
  full_verified: number;
  summary_only: number;
  note_verified: number;
  excluded: number;
  insufficient: number;
  changed: number;
  failed: number;
  pending: number;
  matches: number;
  full_parts_done: number;
  full_parts_total: number;
  cache_hits: number;
  snapshot_revision: number;
}
export interface TopicSearchResume {
  sentence: string;
  manual: EntryFilter;
  progress: TopicSearchProgress;
  session: TopicSearchSession | null;
}
