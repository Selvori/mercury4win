import type { QueryClient } from "@tanstack/react-query";
export function invalidate_tag_queries(client: QueryClient, entry_id?: number) {
  for (const key of ["tags", "tag_aliases", "entries", "feeds", "related_entries", "tag_batch_count"]) {
    void client.invalidateQueries({ queryKey: [key] });
  }
  for (const key of ["entry_detail", "local_tag_suggestions"]) {
    void client.invalidateQueries({ queryKey: entry_id === undefined ? [key] : [key, entry_id] });
  }
}
