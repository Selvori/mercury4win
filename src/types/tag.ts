// mercury4win/src/types/tag.ts

export interface Tag {
  id: number;
  name: string;
  normalized_name: string;
  is_provisional: boolean;
  usage_count: number;
}

export interface TagAlias {
  id: number;
  tag_id: number;
  alias_normalized: string;
  alias: string;
}

export interface TagSuggestion {
  name: string;
  confidence: number;
}

export interface BatchTagResult {
  entries_processed: number;
  tags_assigned: number;
}

export interface TimeRange {
  from: string;
  to: string;
}

export interface BatchScope {
  from: string | null;
  to: string | null;
  unread_only: boolean;
  skip_tagged: boolean;
  skip_applied: boolean;
}
export interface BatchProposal { name: string; normalized_name: string; hits: number }
export interface BatchApplyResult extends BatchTagResult { skipped_deleted: number; skipped_changed: number }
export interface BatchSnapshot {
  id: string;
  status: "running" | "ready" | "done" | "aborted";
  scope: BatchScope;
  total: number;
  processed: number;
  succeeded: number;
  failures: { entry_id: number; error: string }[];
  proposals: BatchProposal[];
  result: BatchApplyResult | null;
}
