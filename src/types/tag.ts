// mercury4win-linux/src/types/tag.ts

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
