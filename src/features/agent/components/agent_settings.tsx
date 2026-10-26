// mercury4win/src/features/agent/components/agent_settings.tsx
// Agent settings panel — provider/model/profile management

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Cpu, Plus, Trash2, Layers, Settings2, Pencil, BarChart3, FileText } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  list_providers,
  add_provider,
  update_provider,
  delete_provider,
  test_provider_config,
  list_models,
  add_model,
  update_model,
  delete_model,
  set_configuration_enabled,
  get_agent_profile,
  update_agent_profile,
  save_custom_template,
  load_custom_template,
} from "@/lib/tauri_bindings";
import type { UsageContext } from "@/types/agent";
import type { ModelProfile, AgentProfile, ProviderProfile, ProviderConfig } from "@/types";
import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { TagLibrary } from "@/features/tags/components/tag_library";
import { UsageStats } from "@/features/usage/components/usage_stats";
import { LanguageSelect } from "./language_select";

type OpenReport = (context: UsageContext) => void;

type Tab = "providers" | "models" | "profiles" | "usage" | "prompts" | "tags";

// Key lookup for agent type display labels
const AGENT_TYPE_LABEL_KEYS: Record<string, string> = {
  summary: "settings.agentTypeSummary",
  translation: "settings.agentTypeTranslation",
  tagging: "settings.agentTypeTagging",
};

export function AgentSettings({ on_tags_changed, on_batch_active_change }: { on_tags_changed?: (removed: number[], target?: number) => void; on_batch_active_change?: (active: boolean) => void } = {}) {
  const { t } = useTranslation();
  const [tab, set_tab] = useState<Tab>("providers");
  const [usage_context, set_usage_context] = useState<UsageContext>();
  const open_report: OpenReport = (context) => { set_usage_context(context); set_tab("usage"); };
  const [batch_active, set_batch_active] = useState(false);
  const handle_batch_activity = useCallback((active: boolean) => { set_batch_active(active); on_batch_active_change?.(active); }, [on_batch_active_change]);

  const tabs = [
    { value: "providers" as Tab, key: "settings.providers", icon: Cpu },
    { value: "models" as Tab, key: "settings.models", icon: Layers },
    { value: "profiles" as Tab, key: "settings.profiles", icon: Settings2 },
    { value: "usage" as Tab, key: "settings.usage", icon: BarChart3 },
    { value: "prompts" as Tab, key: "settings.prompts", icon: FileText },
    { value: "tags" as Tab, key: "tagging.library", icon: Layers },
  ];

  return (
    <div className="flex min-h-0 flex-col">
      <div className="shrink-0 border-b border-border px-6 py-4">
        <LanguageSelect />
      </div>
      {/* Tab navigation */}
      <div className="flex shrink-0 flex-nowrap gap-1 overflow-x-auto overflow-y-hidden border-b border-border px-6 pt-4">
        {tabs.map((opt) => (
          <Button
            key={opt.value}
            aria-pressed={tab === opt.value}
            disabled={batch_active && opt.value !== "tags"}
            variant="ghost"
            size="sm"
            className={cn(
              "rounded-b-none border-b-2 border-transparent px-3",
              tab === opt.value && "border-primary text-primary",
            )}
            onClick={() => { set_tab(opt.value); if (opt.value === "usage") set_usage_context(undefined); }}
          >
            <opt.icon className="mr-1.5 h-4 w-4" />
            {t(opt.key)}
          </Button>
        ))}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-6">
        {tab === "providers" && <ProvidersTab open_report={open_report} />}
        {tab === "models" && <ModelsTab open_report={open_report} />}
        {tab === "profiles" && <ProfilesTab open_report={open_report} />}
        {tab === "usage" && <UsageStats key={JSON.stringify(usage_context)} context={usage_context} on_clear={() => set_usage_context(undefined)} />}
        {tab === "prompts" && <PromptsTab />}
        {tab === "tags" && <TagLibrary on_tags_changed={on_tags_changed} on_batch_active_change={handle_batch_activity} />}
      </div>
    </div>
  );
}

// ── Providers Tab ──

function ProvidersTab({ open_report }: { open_report: OpenReport }) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const [show_add, set_show_add] = useState(false);
  const [editing_provider, set_editing_provider] = useState<ProviderProfile | null>(null);
  const [new_name, set_new_name] = useState("");
  const [new_url, set_new_url] = useState("");
  const [new_key, set_new_key] = useState("");
  const [test_model, set_test_model] = useState("gpt-4o-mini");
  const [feedback, set_feedback] = useState<{ kind: "success" | "error"; message: string } | null>(null);

  const { data: providers, isLoading } = useQuery({
    queryKey: ["providers"],
    queryFn: list_providers,
  });

  const add_mutation = useMutation({
    mutationFn: () => add_provider({ name: new_name, base_url: new_url, api_key: new_key || undefined }),
    onSuccess: () => {
      invalidate_configuration(query_client);
      query_client.invalidateQueries({ queryKey: ["providers"] });
      reset_form();
    },
    onError: (error) => set_feedback({ kind: "error", message: String(error) }),
  });

  const update_mutation = useMutation({
    mutationFn: ({ id, config }: { id: string; config: ProviderConfig }) =>
      update_provider(id, config),
    onSuccess: () => {
      invalidate_configuration(query_client);
      query_client.invalidateQueries({ queryKey: ["providers"] });
      reset_form();
    },
    onError: (error) => set_feedback({ kind: "error", message: String(error) }),
  });

  const [archive_target, set_archive_target] = useState<{ provider: ProviderProfile; models: string[] }>();
  const enable_mutation = useMutation({ mutationFn: (p: ProviderProfile) => set_configuration_enabled(p.id, "provider", !p.is_enabled), onSuccess: () => { invalidate_configuration(query_client); void query_client.invalidateQueries({ queryKey: ["providers"] }); }, onError: error => set_feedback({kind:"error",message:String(error)}) });
  const request_archive = async (provider: ProviderProfile) => { try { const models = await list_models(provider.id); set_archive_target({ provider, models: models.map(m => m.name) }); } catch(error) { set_feedback({kind:"error",message:String(error)}); } };
  const delete_mutation = useMutation({
    mutationFn: (id: string) => delete_provider(id),
    onMutate: () => set_feedback(null),
    onSuccess: () => { set_archive_target(undefined); invalidate_configuration(query_client); return query_client.invalidateQueries({ queryKey: ["providers"] }); },
    onError: (error) => set_feedback({ kind: "error", message: String(error) }),
  });

  const test_mutation = useMutation({
    mutationFn: ({ provider_id, config, model }: { provider_id: string | null; config: ProviderConfig; model: string }) =>
      test_provider_config(provider_id, config, model),
    onMutate: () => set_feedback(null),
    onSuccess: (result) => set_feedback({ kind: "success", message: result || t("settings.connectionSuccess") }),
    onError: (error) => set_feedback({ kind: "error", message: `${t("settings.connectionFailed")}: ${String(error)}` }),
  });

  function reset_form() {
    set_show_add(false);
    set_editing_provider(null);
    set_new_name("");
    set_new_url("");
    set_new_key("");
    set_test_model("gpt-4o-mini");
    set_feedback(null);
  }

  function start_edit(provider: ProviderProfile) {
    set_editing_provider(provider);
    set_new_name(provider.display_name || provider.name);
    set_new_url(provider.base_url);
    set_new_key(""); // Don't show existing key
    set_test_model("gpt-4o-mini");
    set_feedback(null);
    set_show_add(true);
  }

  function handle_save() {
    const name = new_name.trim();
    const base_url = new_url.trim();
    if (editing_provider) {
      update_mutation.mutate({
        id: editing_provider.id,
        config: {
          name,
          base_url,
          display_name: editing_provider.display_name ? name : undefined,
          api_key: new_key || undefined,
        },
      });
    } else {
      add_mutation.mutate();
    }
  }

  function handle_test() {
    const name = new_name.trim();
    const base_url = new_url.trim();
    test_mutation.mutate({
      provider_id: editing_provider?.id ?? null,
      config: { name, base_url, api_key: new_key || undefined },
      model: test_model.trim(),
    });
  }

  return (
    <div className="max-w-2xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold text-foreground">{t("settings.providers")}</h3>
          <p className="text-sm text-muted-foreground mt-1">{t("settings.description")}</p>
        </div>
        <Button size="sm" onClick={() => { set_feedback(null); set_show_add(!show_add); }}>
          <Plus className="mr-1 h-4 w-4" />{t("settings.addProvider")}
        </Button>
      </div>

      {show_add && (
        <div className="mb-6 rounded-lg border border-border bg-card p-4 space-y-3">
          <Input aria-label={t("settings.providerName")} placeholder={t("settings.providerName")} value={new_name} onChange={(e) => set_new_name(e.target.value)} autoFocus />
          <Input aria-label={t("settings.baseUrl")} placeholder={t("settings.baseUrl")} value={new_url} onChange={(e) => set_new_url(e.target.value)} />
          <Input
            type="password"
            aria-label={t("settings.apiKey")}
            placeholder={editing_provider ? t("settings.apiKeyKeepEmpty") : t("settings.apiKey")}
            value={new_key}
            onChange={(e) => set_new_key(e.target.value)}
          />
          <Input
            aria-label={t("settings.testModel")} placeholder={t("settings.testModel")}
            value={test_model}
            onChange={(e) => set_test_model(e.target.value)}
          />
          <div className="flex gap-2">
            <Button size="sm" onClick={handle_save} disabled={add_mutation.isPending || update_mutation.isPending || !new_name.trim() || !new_url.trim()}>
              {(add_mutation.isPending || update_mutation.isPending) ? t("settings.saving") : editing_provider ? t("settings.update") : t("settings.save")}
            </Button>
            <Button size="sm" variant="outline" onClick={handle_test} disabled={test_mutation.isPending || !new_name.trim() || !new_url.trim() || !test_model.trim()}>
              {test_mutation.isPending ? t("settings.testing") : t("settings.testConnection")}
            </Button>
            <Button size="sm" variant="outline" onClick={reset_form}>{t("settings.cancel")}</Button>
          </div>

        </div>
      )}

      {feedback && <p className={cn("mb-2 text-xs", feedback.kind === "error" ? "text-destructive" : "text-green-600 dark:text-green-400")} role={feedback.kind === "error" ? "alert" : "status"}>{feedback.message}</p>}
      <Button variant="outline" size="sm" onClick={() => open_report({dimension:"provider",filter:{}})}>{t("usage.compareProviders")}</Button>
      <p className="my-2 text-xs text-muted-foreground">{t("settings.restoreProviderNote")}</p>
      {archive_target && <div role="alertdialog" aria-label={t("settings.archiveProvider")} className="mb-3 rounded border border-border p-3">
        <p>{t("settings.archiveProviderConfirm",{name:archive_target.provider.name,models:archive_target.models.join(" / ") || t("settings.none")})}</p>
        <Button size="sm" disabled={delete_mutation.isPending} onClick={() => delete_mutation.mutate(archive_target.provider.id)}>{t("settings.archive")}</Button>
        <Button size="sm" variant="outline" onClick={() => set_archive_target(undefined)}>{t("settings.cancel")}</Button>
      </div>}
      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t("settings.loading")}</p>
      ) : !providers?.length ? (
        <div className="rounded-lg border border-border bg-card p-8 text-center">
          <Cpu className="mx-auto h-8 w-8 text-muted-foreground/50" />
          <p className="mt-2 text-sm text-muted-foreground">{t("settings.noProviders")}</p>
          <p className="text-xs text-muted-foreground mt-1">{t("settings.noProvidersHint")}</p>
        </div>
      ) : (
        <div className="space-y-2">
          {providers.map((p) => (
            <div key={p.id} className={cn("flex flex-wrap items-center gap-3 rounded-lg border border-border bg-card p-3", !p.is_enabled && "opacity-50")}>
              <Cpu className="h-5 w-5 text-primary shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium">{p.display_name || p.name}</p>
                <p className="text-xs text-muted-foreground truncate">{p.base_url}</p>
              </div>
              <Button variant="outline" size="sm" disabled={enable_mutation.isPending} onClick={() => enable_mutation.mutate(p)}>{t(p.is_enabled ? "settings.disable" : "settings.enable")}</Button>
              <Button variant="outline" size="sm" onClick={() => open_report({dimension:"provider",filter:{provider_id:p.id},label:p.name})}>{t("usage.statistics")}</Button>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={() => start_edit(p)}
                title={t("settings.edit")} aria-label={t("settings.edit")}
              >
                <Settings2 className="h-3.5 w-3.5" />
              </Button>
              <Button variant="ghost" size="icon" className="h-7 w-7" aria-label={t("settings.archiveProvider")} onClick={() => request_archive(p)}>
                <Trash2 className="h-3.5 w-3.5 text-destructive" />
              </Button>
            </div>
          ))}
        </div>
      )}

    </div>
  );
}

// ── Models Tab ──

function ModelsTab({ open_report }: { open_report: OpenReport }) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const { data: providers } = useQuery({ queryKey: ["providers"], queryFn: list_providers });
  const [selected_provider, set_selected_provider] = useState<string | null>(null);
  const [show_add, set_show_add] = useState(false);
  const [editing, set_editing] = useState<ModelProfile | null>(null);
  const [form_name, set_form_name] = useState("");
  const [form_model, set_form_model] = useState("");
  const [form_supports_summary, set_form_supports_summary] = useState(true);
  const [form_supports_translation, set_form_supports_translation] = useState(true);
  const [form_supports_tagging, set_form_supports_tagging] = useState(true);

  const { data: models, isLoading } = useQuery({
    queryKey: ["models", selected_provider],
    queryFn: () => list_models(selected_provider!),
    enabled: !!selected_provider,
  });

  const add_mutation = useMutation({
    mutationFn: () => add_model(selected_provider!, form_name, form_model, form_supports_summary, form_supports_translation, form_supports_tagging),
    onSuccess: () => {
      invalidate_configuration(query_client);
      query_client.invalidateQueries({ queryKey: ["models", selected_provider] });
      reset_form();
    },
  });

  const update_mutation = useMutation({
    mutationFn: (model: ModelProfile) =>
      update_model(model.id, form_name, form_model, form_supports_summary, form_supports_translation, form_supports_tagging),
    onSuccess: () => {
      invalidate_configuration(query_client);
      query_client.invalidateQueries({ queryKey: ["models", selected_provider] });
      reset_form();
    },
  });

  const [archive_model, set_archive_model] = useState<ModelProfile>();
  const enable_mutation = useMutation({ mutationFn: (m: ModelProfile) => set_configuration_enabled(m.id,"model",!m.is_enabled), onSuccess: () => { invalidate_configuration(query_client); void query_client.invalidateQueries({queryKey:["models"]}); } });
  const delete_mutation = useMutation({
    mutationFn: (id: string) => delete_model(id),
    onSuccess: () => { set_archive_model(undefined); invalidate_configuration(query_client); return query_client.invalidateQueries({ queryKey: ["models", selected_provider] }); },
  });

  function reset_form() {
    set_show_add(false);
    set_editing(null);
    set_form_name("");
    set_form_model("");
    set_form_supports_summary(true);
    set_form_supports_translation(true);
    set_form_supports_tagging(true);
  }

  function start_edit(model: ModelProfile) {
    set_editing(model);
    set_form_name(model.name);
    set_form_model(model.model_name);
    set_form_supports_summary(model.supports_summary);
    set_form_supports_translation(model.supports_translation);
    set_form_supports_tagging(model.supports_tagging);
    set_show_add(true);
  }

  return (
    <div className="max-w-2xl mx-auto">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-foreground">{t("settings.models")}</h3>
        <select
          aria-label={t("settings.selectProvider")}
          className="h-8 rounded-md border border-border bg-background px-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          value={selected_provider ?? ""}
          onChange={(e) => { set_selected_provider(e.target.value || null); reset_form(); }}
        >
          <option value="">{t("settings.selectProvider")}</option>
          {(providers ?? []).map((p) => <option key={p.id} value={p.id}>{p.display_name || p.name}</option>)}
        </select>
      </div>

      <Button variant="outline" size="sm" onClick={() => open_report({dimension:"model",filter:{}})}>{t("usage.compareModels")}</Button>
      {[add_mutation.error,update_mutation.error,delete_mutation.error,enable_mutation.error].filter(Boolean).map((error,i) => <p role="alert" key={i} className="text-destructive">{String(error)}</p>)}
      {archive_model && <div role="alertdialog" aria-label={t("settings.archiveModel")} className="rounded border border-border p-3">
        <p>{t("settings.archiveModelConfirm",{name:archive_model.name})}</p>
        <Button size="sm" disabled={delete_mutation.isPending} onClick={() => delete_mutation.mutate(archive_model.id)}>{t("settings.archive")}</Button>
        <Button variant="outline" size="sm" onClick={() => set_archive_model(undefined)}>{t("settings.cancel")}</Button>
      </div>}
      {!selected_provider ? (
        <div className="rounded-lg border border-border bg-card p-8 text-center">
          <Layers className="mx-auto h-8 w-8 text-muted-foreground/50" />
          <p className="mt-2 text-sm text-muted-foreground">{t("settings.selectProviderToManage")}</p>
        </div>
      ) : (
        <>
          <div className="mb-4 flex justify-end">
            <Button size="sm" onClick={() => set_show_add(true)} disabled={show_add}>
              <Plus className="mr-1 h-3.5 w-3.5" />{t("settings.addModel")}
            </Button>
          </div>

          {show_add && (
            <div className="mb-4 rounded-lg border border-border bg-card p-4 space-y-3">
              <Input aria-label={t("settings.displayName")} placeholder={t("settings.displayName")} value={form_name} onChange={(e) => set_form_name(e.target.value)} autoFocus />
              <Input aria-label={t("settings.modelId")} placeholder={t("settings.modelId")} value={form_model} onChange={(e) => set_form_model(e.target.value)} />
              <div className="flex flex-wrap gap-4 text-xs">
                <label className="flex items-center gap-1.5">
                  <input type="checkbox" checked={form_supports_summary} onChange={(e) => set_form_supports_summary(e.target.checked)} />
                  {t("settings.agentTypeSummary")}
                </label>
                <label className="flex items-center gap-1.5">
                  <input type="checkbox" checked={form_supports_translation} onChange={(e) => set_form_supports_translation(e.target.checked)} />
                  {t("settings.agentTypeTranslation")}
                </label>
                <label className="flex items-center gap-1.5">
                  <input type="checkbox" checked={form_supports_tagging} onChange={(e) => set_form_supports_tagging(e.target.checked)} />
                  {t("settings.agentTypeTagging")}
                </label>
              </div>
              <div className="flex gap-2">
                <Button size="sm" onClick={() => editing ? update_mutation.mutate(editing) : add_mutation.mutate()} disabled={add_mutation.isPending || !form_name || !form_model}>
                  {add_mutation.isPending ? t("settings.saving") : editing ? t("settings.update") : t("settings.save")}
                </Button>
                <Button size="sm" variant="outline" onClick={reset_form}>{t("settings.cancel")}</Button>
              </div>
            </div>
          )}

          {isLoading ? (
            <p className="text-sm text-muted-foreground">{t("settings.loading")}</p>
          ) : !models?.length ? (
            <div className="rounded-lg border border-border bg-card p-8 text-center">
              <p className="text-sm text-muted-foreground">{t("settings.noModels")}</p>
            </div>
          ) : (
            <div className="space-y-1.5">
              {models.map((m) => (
                <div key={m.id} className={cn("flex flex-wrap items-center gap-3 rounded-lg border border-border bg-card p-3", !m.is_enabled && "opacity-50")}>
                  <Layers className="h-4 w-4 text-muted-foreground shrink-0" />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium">{m.name}</p>
                    <div className="flex flex-wrap gap-1 mt-0.5">
                      {m.supports_summary && <span className="text-[10px] rounded bg-blue-100 dark:bg-blue-900/30 px-1 py-0.5 text-blue-700 dark:text-blue-300">{t("settings.agentTypeSummary")}</span>}
                      {m.supports_translation && <span className="text-[10px] rounded bg-green-100 dark:bg-green-900/30 px-1 py-0.5 text-green-700 dark:text-green-300">{t("settings.agentTypeTranslation")}</span>}
                      {m.supports_tagging && <span className="text-[10px] rounded bg-purple-100 dark:bg-purple-900/30 px-1 py-0.5 text-purple-700 dark:text-purple-300">{t("settings.agentTypeTagging")}</span>}
                    </div>
                  </div>
                  <Button variant="outline" size="sm" disabled={enable_mutation.isPending} onClick={() => enable_mutation.mutate(m)}>{t(m.is_enabled ? "settings.disable" : "settings.enable")}</Button>
                  <Button variant="outline" size="sm" onClick={() => open_report({dimension:"model",filter:{model_id:m.id},label:m.name})}>{t("usage.statistics")}</Button>
                  <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => start_edit(m)} title={t("settings.edit")} aria-label={t("settings.edit")}>
                    <Pencil className="h-3.5 w-3.5" />
                  </Button>
                  <Button variant="ghost" size="icon" className="h-7 w-7" aria-label={t("settings.archiveModel")} onClick={() => set_archive_model(m)}>
                    <Trash2 className="h-3.5 w-3.5 text-destructive" />
                  </Button>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

function invalidate_configuration(query_client: ReturnType<typeof useQueryClient>) {
  void query_client.invalidateQueries({ queryKey: ["agent_fingerprint"] });
  void query_client.invalidateQueries({ queryKey: ["models"] });
  void query_client.invalidateQueries({ queryKey: ["usage"] });
}

// ── Profiles Tab ──

const AGENT_TYPES = ["summary", "translation", "tagging"] as const;

function ProfilesTab({ open_report }: { open_report: OpenReport }) {
  const { t } = useTranslation();
  return (
    <div className="max-w-2xl mx-auto">
      <h3 className="text-lg font-semibold text-foreground mb-4">{t("settings.agentProfiles")}</h3>
      <p className="text-sm text-muted-foreground mb-6">{t("settings.profilesDescription")}</p>
      <Button variant="outline" size="sm" onClick={() => open_report({dimension:"agent",filter:{}})}>{t("usage.compareAgents")}</Button>
      <div className="space-y-4">
        {AGENT_TYPES.map((agent_type) => (
          <ProfileSection key={agent_type} agent_type={agent_type} open_report={open_report} />
        ))}
      </div>
    </div>
  );
}

function ProfileSection({ agent_type, open_report }: { agent_type: string; open_report: OpenReport }) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const { data: providers } = useQuery({ queryKey: ["providers"], queryFn: list_providers });

  const { data: profile } = useQuery({
    queryKey: ["agent_profile", agent_type],
    queryFn: () => get_agent_profile(agent_type),
  });

  const { data: models } = useQuery({
    queryKey: ["models", "all"],
    queryFn: () => {
      if (!providers?.length) return Promise.resolve([] as ModelProfile[]);
      return Promise.all(providers.filter(p => p.is_enabled).map((p) => list_models(p.id))).then((arr) => arr.flat());
    },
    enabled: !!providers?.length,
  });

  const update_mutation = useMutation({
    mutationFn: (data: Partial<AgentProfile>) =>
      update_agent_profile(
        agent_type,
        data.primary_model_id !== undefined ? data.primary_model_id : primary || null,
        data.fallback_model_id !== undefined ? data.fallback_model_id : profile?.fallback_model_id ?? null,
        data.target_language !== undefined ? data.target_language : target_lang,
        data.detail_level !== undefined ? data.detail_level : detail_level,
        data.prompt_strategy !== undefined ? data.prompt_strategy : profile?.prompt_strategy ?? "default",
        data.concurrency_degree !== undefined ? data.concurrency_degree : profile?.concurrency_degree ?? null,
      ),
    onSuccess: () => { invalidate_configuration(query_client); return query_client.invalidateQueries({ queryKey: ["agent_profile", agent_type] }); },
  });

  const [primary, set_primary] = useState(profile?.primary_model_id ?? "");
  const [target_lang, set_target_lang] = useState(profile?.target_language ?? "en");
  const [detail_level, set_detail_level] = useState(profile?.detail_level ?? "medium");

  useEffect(() => {
    if (!profile) return;
    set_primary(profile.primary_model_id ?? "");
    set_target_lang(profile.target_language ?? "en");
    set_detail_level(profile.detail_level ?? "medium");
  }, [profile]);

  const compatible_models = (models ?? []).filter((m) => m.is_enabled && providers?.some((provider) => provider.is_enabled && provider.id === m.provider_id) && (agent_type === "summary" ? m.supports_summary : agent_type === "translation" ? m.supports_translation : m.supports_tagging));

  const detail_level_options: Record<string, string> = {
    brief: "summary.brief",
    medium: "summary.medium",
    detailed: "summary.detailed",
  };

  const prompt_strategy_options: Record<string, string> = {
    default: "settings.standard",
    "hy-mt-optimized": "settings.hyMtOptimized",
  };

  const language_options = [
    { value: "en", key: "common.languageEnglish" },
    { value: "zh-Hans", key: "common.languageChineseSimplified" },
    { value: "ja", key: "common.languageJapanese" },
    { value: "ko", key: "common.languageKorean" },
    { value: "fr", key: "common.languageFrench" },
    { value: "de", key: "common.languageGerman" },
    { value: "es", key: "common.languageSpanish" },
  ];

  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <h4 className="text-sm font-semibold mb-3">{t(AGENT_TYPE_LABEL_KEYS[agent_type] ?? agent_type)}</h4>
      <Button variant="outline" size="sm" onClick={() => open_report({dimension:"agent",filter:{agent_type},label:t(AGENT_TYPE_LABEL_KEYS[agent_type])})}>{t("usage.statistics")}</Button>
      {update_mutation.error && <p role="alert" className="mb-2 text-sm text-destructive">{String(update_mutation.error)}</p>}
      <div className="grid grid-cols-2 gap-3 text-sm">
        <div>
          <label className="text-xs text-muted-foreground block mb-1">{t("settings.primaryModel")}</label>
          <select aria-label={`${t(AGENT_TYPE_LABEL_KEYS[agent_type])} ${t("settings.primaryModel")}`}
            className="w-full h-8 rounded-md border border-border bg-background px-2 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
            value={primary}
            onChange={(e) => {
              set_primary(e.target.value);
              update_mutation.mutate({ primary_model_id: e.target.value || null });
            }}
          >
            <option value="">{t("settings.auto")}</option>
            {primary && !compatible_models.some(m => m.id === primary) && <option value={primary} disabled>{t("settings.unavailableModel")}</option>}
            {compatible_models.map((m) => (
              <option key={m.id} value={m.id}>{m.name} ({m.model_name})</option>
            ))}
          </select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground block mb-1">{t("settings.fallbackModel")}</label>
          <select aria-label={`${t(AGENT_TYPE_LABEL_KEYS[agent_type])} ${t("settings.fallbackModel")}`} className="w-full h-8 rounded-md border border-border bg-background px-2 text-xs" value={profile?.fallback_model_id ?? ""} onChange={(event) => update_mutation.mutate({ fallback_model_id: event.target.value || null })}>
            <option value="">{t("settings.noFallback")}</option>
            {profile?.fallback_model_id && !compatible_models.some(m => m.id === profile.fallback_model_id) && <option value={profile.fallback_model_id} disabled>{t("settings.unavailableModel")}</option>}
            {compatible_models.filter((model) => model.id !== primary).map((model) => <option key={model.id} value={model.id}>{model.name} ({model.model_name})</option>)}
          </select>
        </div>
        {agent_type !== "tagging" && (
          <div>
            <label className="text-xs text-muted-foreground block mb-1">{t("settings.targetLanguage")}</label>
            <select aria-label={`${t(AGENT_TYPE_LABEL_KEYS[agent_type])} ${t("settings.targetLanguage")}`}
              className="w-full h-8 rounded-md border border-border bg-background px-2 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
              value={target_lang}
              onChange={(e) => {
                set_target_lang(e.target.value);
                update_mutation.mutate({ target_language: e.target.value });
              }}
            >
              {language_options.map((opt) => (
                <option key={opt.value} value={opt.value}>{t(opt.key)}</option>
              ))}
            </select>
          </div>
        )}
        {agent_type === "summary" && (
          <div>
            <label className="text-xs text-muted-foreground block mb-1">{t("settings.detailLevel")}</label>
            <select aria-label={`${t(AGENT_TYPE_LABEL_KEYS[agent_type])} ${t("settings.detailLevel")}`}
              className="w-full h-8 rounded-md border border-border bg-background px-2 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
              value={detail_level}
              onChange={(e) => {
                set_detail_level(e.target.value);
                update_mutation.mutate({ detail_level: e.target.value });
              }}
            >
              {Object.entries(detail_level_options).map(([value, key]) => (
                <option key={value} value={value}>{t(key)}</option>
              ))}
            </select>
          </div>
        )}
        {agent_type === "translation" && (
          <div>
            <label className="text-xs text-muted-foreground block mb-1">{t("settings.promptStrategy")}</label>
            <select aria-label={`${t(AGENT_TYPE_LABEL_KEYS[agent_type])} ${t("settings.promptStrategy")}`}
              className="w-full h-8 rounded-md border border-border bg-background px-2 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
              value={profile?.prompt_strategy ?? "default"}
              onChange={(e) => update_mutation.mutate({ prompt_strategy: e.target.value })}
            >
              {Object.entries(prompt_strategy_options).map(([value, key]) => (
                <option key={value} value={value}>{t(key)}</option>
              ))}
            </select>
          </div>
        )}
      </div>
    </div>
  );
}

// ── Prompts Tab ──

const PROMPT_TYPES = ["summary", "translation", "tagging"] as const;

function PromptsTab() {
  const { t } = useTranslation();
  const query_client = useQueryClient();

  return (
    <div className="max-w-2xl mx-auto">
      <h3 className="text-lg font-semibold text-foreground mb-4">{t("settings.customPrompts")}</h3>
      <p className="text-sm text-muted-foreground mb-6">
        {t("settings.customPromptsDescription")}
      </p>
      <div className="space-y-4">
        {PROMPT_TYPES.map((agent_type) => (
          <PromptSection key={agent_type} agent_type={agent_type} query_client={query_client} />
        ))}
      </div>
    </div>
  );
}

function PromptSection({
  agent_type,
  query_client,
}: {
  agent_type: string;
  query_client: ReturnType<typeof useQueryClient>;
}) {
  const { t } = useTranslation();

  const { data: template, isLoading, error: template_error } = useQuery({
    queryKey: ["custom_template", agent_type],
    queryFn: () => load_custom_template(agent_type),
  });

  const [status, set_status] = useState<string | null>(null);

  async function handle_upload() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        title: t("settings.uploadYaml"),
        filters: [{ name: t("common.yamlFile"), extensions: ["yaml", "yml"] }],
        multiple: false,
      });
      if (selected) {
        // Handle both string (expected) and string[] (defensive)
        const path = Array.isArray(selected) ? selected[0] : selected;
        if (!path) return;
        await save_custom_template(agent_type, path);
        invalidate_configuration(query_client);
        query_client.invalidateQueries({ queryKey: ["custom_template", agent_type] });
        set_status(t("settings.templateUploaded"));
      }
    } catch (e) {
      set_status(`${t("common.error")}: ${e}`);
    }
  }

  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="flex items-center justify-between mb-2">
        <h4 className="text-sm font-semibold">{t(AGENT_TYPE_LABEL_KEYS[agent_type] ?? agent_type)}</h4>
        <button
          onClick={handle_upload}
          className="inline-flex items-center gap-1 rounded-md border border-border bg-background px-2.5 py-1 text-xs hover:bg-accent transition-colors"
        >
          <Plus className="h-3 w-3" />
          {t("settings.uploadYaml")}
        </button>
      </div>

      {isLoading ? (
        <p className="text-xs text-muted-foreground">{t("settings.loading")}</p>
      ) : template ? (
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <span className="inline-flex items-center rounded-full bg-green-100 dark:bg-green-900/30 px-2 py-0.5 text-[10px] text-green-700 dark:text-green-300">
              {t("settings.customBadge")}
            </span>
          </div>
          <p className="text-xs text-muted-foreground line-clamp-3 font-mono whitespace-pre-wrap">
            {template}
          </p>
        </div>
      ) : (
        <p className="text-xs text-muted-foreground">
          {t("settings.usingDefaultTemplate")}
        </p>
      )}

      {template_error && <p role="alert" className="text-xs text-destructive">{String(template_error)}</p>}
      {status && (
        <p className="mt-2 text-xs text-muted-foreground">{status}</p>
      )}
    </div>
  );
}
