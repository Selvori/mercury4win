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
}

export interface EntryPage {
  entries: Entry[];
  next_cursor: EntryCursor | null;
  total: number;
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
  feed_id?: number;
  unread_only?: boolean;
  starred_only?: boolean;
  search?: string;
  tag_ids?: number[];
  tag_mode?: TagMatchMode;
}
