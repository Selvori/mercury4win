// mercury4win-linux/src/types/entry.ts

export interface Entry {
  id: number;
  feed_id: number;
  guid: string | null;
  url: string | null;
  title: string | null;
  author: string | null;
  published_at: string | null;
  summary: string | null;
  is_read: boolean;
  is_starred: boolean;
  is_deleted: boolean;
  created_at: string;
}

export interface EntryPage {
  entries: Entry[];
  next_cursor: string | null;
  total: number;
}

export interface EntryDetail extends Entry {
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
