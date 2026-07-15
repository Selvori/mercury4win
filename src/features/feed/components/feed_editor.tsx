// mercury4win-linux/src/features/feed/components/feed_editor.tsx
// Add/edit feed dialog

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { add_feed } from "@/lib/tauri_bindings";

interface Props {
  open: boolean;
  on_close: () => void;
}

export function FeedEditor({ open, on_close }: Props) {
  const [url, set_url] = useState("");
  const query_client = useQueryClient();

  const mutation = useMutation({
    mutationFn: (feed_url: string) => add_feed(feed_url),
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["feeds"] });
      query_client.invalidateQueries({ queryKey: ["entries"] });
      set_url("");
      on_close();
    },
  });

  function handle_submit(e: React.FormEvent) {
    e.preventDefault();
    if (url.trim()) {
      mutation.mutate(url.trim());
    }
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && on_close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add Feed</DialogTitle>
        </DialogHeader>
        <form onSubmit={handle_submit}>
          <div className="space-y-3 py-4">
            <Input
              placeholder="https://example.com/feed.xml"
              value={url}
              onChange={(e) => set_url(e.target.value)}
              autoFocus
            />
            {mutation.isError && (
              <p className="text-xs text-destructive">{String(mutation.error)}</p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" type="button" onClick={on_close}>
              Cancel
            </Button>
            <Button type="submit" disabled={mutation.isPending || !url.trim()}>
              {mutation.isPending ? "Adding..." : "Add Feed"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
