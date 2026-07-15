// mercury4win-linux/src/types/feed.ts

export interface Feed {
  id: number;
  title: string | null;
  feed_url: string;
  site_url: string | null;
  feed_parser_version: number | null;
  last_fetched_at: string | null;
  created_at: string;
}

export interface FeedWithCounts extends Feed {
  unread_count: number;
  total_count: number;
}

export interface SyncResult {
  feed_id: number;
  new_entries: number;
  updated_entries: number;
  error: string | null;
}

export interface ImportResult {
  feeds_added: number;
  feeds_skipped: number;
  errors: string[];
}
