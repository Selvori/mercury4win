// mercury4win/src/types/entry.ts

export interface Entry {
  id: number;
  feed_id: number;
  guid: string | null;
  url: string | null;
  title: string | null;
  author: string | null;
  published_at: string | null;
  summary: string | null;
  feed_content: string | null;
  feed_content_type: string | null;
  is_read: boolean;
  is_starred: boolean;
  is_deleted: boolean;
  created_at: string;
}

export interface EntryCursor {
  published_at: string | null;
  id: number;
  score?: number | null;
  anchor?: string | null;
}

export interface EntryPage {
  entries: Entry[];
  next_cursor: EntryCursor | null;
  total: number;
  search_info?: SearchInfo[];
}

export interface EntryDetail {
  entry: Entry;
  feed_title: string | null;
  tags: EntryTagInfo[];
  has_note: boolean;
}

export interface EntryTagInfo {
  tag_id: number;
  name: string;
  source: string;
}

export interface EntryNote {
  entry_id: number;
  markdown: string;
  updated_at: string;
}

export type TagMatchMode = "any" | "all";

export interface EntryFilter {
  feed_id?: number | null;
  unread_only?: boolean;
  starred_only?: boolean;
  search?: string | null;
  tag_ids?: number[] | null;
  tag_mode?: TagMatchMode;
  is_read?: boolean | null;
  has_note?: boolean | null;
  date_from?: string | null;
  date_to?: string | null;
  recent_days?: number | null;
  search_field?: "all" | "title" | "summary" | "body" | "note";
  exclude_title?: boolean;
  sort?: "newest" | "oldest" | "relevance";
}

export interface SearchPart { text: string; matched: boolean }
export interface SearchHit { field: "title" | "summary" | "body" | "note"; parts: SearchPart[] }
export interface SearchInfo { preview?: string; entry_id: number; feed_title: string; body_available: boolean; hits: SearchHit[] }
export interface SmartView { id: number; name: string; filter: EntryFilter; updated_at: string }
