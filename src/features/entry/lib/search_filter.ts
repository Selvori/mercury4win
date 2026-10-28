import type { EntryFilter } from "@/types";

// Parents merge defaults/navigation into filters. Object key order, omitted
// defaults and tag order must not invalidate the same effective conditions.
export function search_filter_identity(filter: EntryFilter): string {
  return JSON.stringify({
    ai_search_id: filter.ai_search_id ?? null,
    feed_id: filter.feed_id ?? null,
    search: filter.search ?? "",
    tag_ids: [...new Set(filter.tag_ids ?? [])].sort((a, b) => a - b),
    tag_mode: filter.tag_mode ?? "any",
    is_read: filter.is_read ?? (filter.unread_only ? false : null),
    starred_only: filter.starred_only ?? false,
    has_note: filter.has_note ?? null,
    date_from: filter.date_from || null,
    date_to: filter.date_to || null,
    recent_days: filter.recent_days ?? null,
    search_field: filter.search_field ?? "all",
    exclude_title: filter.exclude_title ?? false,
    sort: filter.sort ?? "newest",
  });
}

export function search_filter_error(filter: EntryFilter): string | null {
  const query = filter.search ?? "";
  if ([...query].length > 1000 || query.trim().split(/\s+/u).filter(Boolean).length > 24) return "search.queryLimit";
  if (filter.recent_days != null && (!Number.isInteger(filter.recent_days) || filter.recent_days < 1 || filter.recent_days > 36500)) return "search.invalidDays";
  if (filter.date_from && filter.date_to && filter.date_from > filter.date_to) return "search.invalidDates";
  return null;
}
