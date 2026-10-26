import type { EntryFilter } from "@/types";

export function search_filter_error(filter: EntryFilter): string | null {
  const query = filter.search ?? "";
  if ([...query].length > 1000 || query.trim().split(/\s+/u).filter(Boolean).length > 24) return "search.queryLimit";
  if (filter.recent_days != null && (!Number.isInteger(filter.recent_days) || filter.recent_days < 1 || filter.recent_days > 36500)) return "search.invalidDays";
  if (filter.date_from && filter.date_to && filter.date_from > filter.date_to) return "search.invalidDates";
  return null;
}
