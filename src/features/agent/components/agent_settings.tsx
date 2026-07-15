// mercury4win-linux/src/features/agent/components/agent_settings.tsx
// Agent settings panel — provider/model/profile management

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Cpu, Plus, Trash2, Play } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  list_providers,
  add_provider,
  delete_provider,
  test_provider_connection,
} from "@/lib/tauri_bindings";
import { useState } from "react";

export function AgentSettings() {
  const query_client = useQueryClient();
  const [show_add, set_show_add] = useState(false);
  const [new_name, set_new_name] = useState("");
  const [new_url, set_new_url] = useState("");
  const [new_key, set_new_key] = useState("");
  const [test_result, set_test_result] = useState<string | null>(null);

  const { data: providers, isLoading } = useQuery({
    queryKey: ["providers"],
    queryFn: list_providers,
  });

  const add_mutation = useMutation({
    mutationFn: () =>
      add_provider({
        name: new_name,
        base_url: new_url,
        api_key: new_key || undefined,
      }),
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["providers"] });
      set_show_add(false);
      set_new_name("");
      set_new_url("");
      set_new_key("");
    },
  });

  const delete_mutation = useMutation({
    mutationFn: (id: string) => delete_provider(id),
    onSuccess: () => query_client.invalidateQueries({ queryKey: ["providers"] }),
  });

  const test_mutation = useMutation({
    mutationFn: ({ id, model }: { id: string; model: string }) =>
      test_provider_connection(id, model),
    onSuccess: (result) => set_test_result(result),
  });

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-xl font-semibold text-foreground">AI Providers</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Configure OpenAI-compatible API endpoints for summaries, translations, and tagging.
          </p>
        </div>
        <Button
          size="sm"
          onClick={() => set_show_add(!show_add)}
        >
          <Plus className="mr-1 h-4 w-4" />
          Add Provider
        </Button>
      </div>

      {/* Add form */}
      {show_add && (
        <div className="mb-6 rounded-lg border border-border bg-card p-4 space-y-3">
          <Input
            placeholder="Provider name (e.g., OpenAI)"
            value={new_name}
            onChange={(e) => set_new_name(e.target.value)}
          />
          <Input
            placeholder="Base URL (e.g., https://api.openai.com/v1)"
            value={new_url}
            onChange={(e) => set_new_url(e.target.value)}
          />
          <Input
            type="password"
            placeholder="API Key"
            value={new_key}
            onChange={(e) => set_new_key(e.target.value)}
          />
          <div className="flex gap-2">
            <Button
              size="sm"
              onClick={() => add_mutation.mutate()}
              disabled={add_mutation.isPending || !new_name || !new_url}
            >
              {add_mutation.isPending ? "Saving..." : "Save"}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => set_show_add(false)}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}

      {/* Provider list */}
      {isLoading ? (
        <p className="text-sm text-muted-foreground">Loading...</p>
      ) : !providers?.length ? (
        <div className="rounded-lg border border-border bg-card p-8 text-center">
          <Cpu className="mx-auto h-8 w-8 text-muted-foreground/50" />
          <p className="mt-2 text-sm text-muted-foreground">No AI providers configured</p>
          <p className="text-xs text-muted-foreground mt-1">
            Add a provider like OpenAI, Anthropic, or any OpenAI-compatible API.
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {providers.map((p) => (
            <div
              key={p.id}
              className={cn(
                "flex items-center gap-3 rounded-lg border border-border bg-card p-3",
                !p.is_enabled && "opacity-50",
              )}
            >
              <Cpu className="h-5 w-5 text-primary shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium">{p.display_name || p.name}</p>
                <p className="text-xs text-muted-foreground truncate">{p.base_url}</p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={() =>
                  test_mutation.mutate({ id: p.id, model: "gpt-4o-mini" })
                }
                title="Test connection"
              >
                <Play className="h-3.5 w-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={() => delete_mutation.mutate(p.id)}
              >
                <Trash2 className="h-3.5 w-3.5 text-destructive" />
              </Button>
            </div>
          ))}
        </div>
      )}

      {test_result !== null && (
        <div className="mt-4 rounded-lg border border-border bg-card p-3">
          <p className="text-xs font-medium text-foreground">Connection test result:</p>
          <p className="text-xs text-muted-foreground mt-1">{test_result}</p>
        </div>
      )}
    </div>
  );
}
