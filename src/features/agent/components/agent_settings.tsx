// mercury4win-linux/src/features/agent/components/agent_settings.tsx
// Agent settings panel — provider/model/profile management

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Cpu, Plus, Trash2, Play, Layers, Settings2, BarChart3, FileText } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  list_providers,
  add_provider,
  delete_provider,
  test_provider_connection,
  list_models,
  add_model,
  update_model,
  delete_model,
  get_agent_profile,
  update_agent_profile,
  save_custom_template,
  load_custom_template,
} from "@/lib/tauri_bindings";
import type { ModelProfile, AgentProfile } from "@/types";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { UsageStats } from "@/features/usage/components/usage_stats";

type Tab = "providers" | "models" | "profiles" | "usage" | "prompts";

// Key lookup for agent type display labels
const AGENT_TYPE_LABEL_KEYS: Record<string, string> = {
  summary: "settings.agentTypeSummary",
  translation: "settings.agentTypeTranslation",
  tagging: "settings.agentTypeTagging",
};

export function AgentSettings() {
  const { t } = useTranslation();
  const [tab, set_tab] = useState<Tab>("providers");

  const tabs = [
    { value: "providers" as Tab, key: "settings.providers", icon: Cpu },
    { value: "models" as Tab, key: "settings.models", icon: Layers },
    { value: "profiles" as Tab, key: "settings.profiles", icon: Settings2 },
    { value: "usage" as Tab, key: "settings.usage", icon: BarChart3 },
    { value: "prompts" as Tab, key: "settings.prompts", icon: FileText },
  ];

  return (
    <div className="max-h-[85vh] flex flex-col">
      {/* Tab navigation */}
      <div className="flex gap-1 border-b border-border px-6 pt-4">
        {tabs.map((opt) => (
          <Button
            key={opt.value}
            variant="ghost"
            size="sm"
            className={cn(
              "rounded-b-none border-b-2 border-transparent px-4",
              tab === opt.value && "border-primary text-primary",
            )}
            onClick={() => set_tab(opt.value)}
          >
            <opt.icon className="mr-1.5 h-4 w-4" />
            {t(opt.key)}
          </Button>
        ))}
      </div>

      <div className="overflow-y-auto p-6">
        {tab === "providers" && <ProvidersTab />}
        {tab === "models" && <ModelsTab />}
        {tab === "profiles" && <ProfilesTab />}
        {tab === "usage" && <UsageStats />}
        {tab === "prompts" && <PromptsTab />}
      </div>
    </div>
  );
}

// ── Providers Tab ──

function ProvidersTab() {
  const { t } = useTranslation();
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
    mutationFn: () => add_provider({ name: new_name, base_url: new_url, api_key: new_key || undefined }),
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
    mutationFn: ({ id, model }: { id: string; model: string }) => test_provider_connection(id, model),
    onSuccess: (result) => set_test_result(result),
  });

  return (
    <div className="max-w-2xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold text-foreground">{t("settings.providers")}</h3>
          <p className="text-sm text-muted-foreground mt-1">{t("settings.description")}</p>
        </div>
        <Button size="sm" onClick={() => set_show_add(!show_add)}>
          <Plus className="mr-1 h-4 w-4" />{t("settings.addProvider")}
        </Button>
      </div>

      {show_add && (
        <div className="mb-6 rounded-lg border border-border bg-card p-4 space-y-3">
          <Input placeholder={t("settings.providerName")} value={new_name} onChange={(e) => set_new_name(e.target.value)} />
          <Input placeholder={t("settings.baseUrl")} value={new_url} onChange={(e) => set_new_url(e.target.value)} />
          <Input type="password" placeholder={t("settings.apiKey")} value={new_key} onChange={(e) => set_new_key(e.target.value)} />
          <div className="flex gap-2">
            <Button size="sm" onClick={() => add_mutation.mutate()} disabled={add_mutation.isPending || !new_name || !new_url}>
              {add_mutation.isPending ? t("settings.saving") : t("settings.save")}
            </Button>
            <Button size="sm" variant="outline" onClick={() => set_show_add(false)}>{t("settings.cancel")}</Button>
          </div>
        </div>
      )}

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
            <div key={p.id} className={cn("flex items-center gap-3 rounded-lg border border-border bg-card p-3", !p.is_enabled && "opacity-50")}>
              <Cpu className="h-5 w-5 text-primary shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium">{p.display_name || p.name}</p>
                <p className="text-xs text-muted-foreground truncate">{p.base_url}</p>
              </div>
              <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => test_mutation.mutate({ id: p.id, model: "gpt-4o-mini" })} title={t("settings.testConnection")}>
                <Play className="h-3.5 w-3.5" />
              </Button>
              <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => delete_mutation.mutate(p.id)}>
                <Trash2 className="h-3.5 w-3.5 text-destructive" />
              </Button>
            </div>
          ))}
        </div>
      )}

      {test_result !== null && (
        <div className="mt-4 rounded-lg border border-border bg-card p-3">
          <p className="text-xs font-medium text-foreground">{t("settings.connectionResult")}</p>
          <p className="text-xs text-muted-foreground mt-1">{test_result}</p>
        </div>
      )}
    </div>
  );
}

// ── Models Tab ──

function ModelsTab() {
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
    mutationFn: () => add_model(selected_provider!, form_name, form_model),
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["models", selected_provider] });
      reset_form();
    },
  });

  const update_mutation = useMutation({
    mutationFn: (model: ModelProfile) =>
      update_model(model.id, form_name, form_model, form_supports_summary, form_supports_translation, form_supports_tagging),
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["models", selected_provider] });
      reset_form();
    },
  });

  const delete_mutation = useMutation({
    mutationFn: (id: string) => delete_model(id),
    onSuccess: () => query_client.invalidateQueries({ queryKey: ["models", selected_provider] }),
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
          className="h-8 rounded-md border border-border bg-background px-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          value={selected_provider ?? ""}
          onChange={(e) => { set_selected_provider(e.target.value || null); reset_form(); }}
        >
          <option value="">{t("settings.selectProvider")}</option>
          {(providers ?? []).map((p) => <option key={p.id} value={p.id}>{p.display_name || p.name}</option>)}
        </select>
      </div>

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
              <Input placeholder={t("settings.displayName")} value={form_name} onChange={(e) => set_form_name(e.target.value)} />
              <Input placeholder={t("settings.modelId")} value={form_model} onChange={(e) => set_form_model(e.target.value)} />
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
                <div key={m.id} className={cn("flex items-center gap-3 rounded-lg border border-border bg-card p-3", !m.is_enabled && "opacity-50")}>
                  <Layers className="h-4 w-4 text-muted-foreground shrink-0" />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium">{m.name}</p>
                    <div className="flex flex-wrap gap-1 mt-0.5">
                      {m.supports_summary && <span className="text-[10px] rounded bg-blue-100 dark:bg-blue-900/30 px-1 py-0.5 text-blue-700 dark:text-blue-300">{t("settings.agentTypeSummary")}</span>}
                      {m.supports_translation && <span className="text-[10px] rounded bg-green-100 dark:bg-green-900/30 px-1 py-0.5 text-green-700 dark:text-green-300">{t("settings.agentTypeTranslation")}</span>}
                      {m.supports_tagging && <span className="text-[10px] rounded bg-purple-100 dark:bg-purple-900/30 px-1 py-0.5 text-purple-700 dark:text-purple-300">{t("settings.agentTypeTagging")}</span>}
                    </div>
                  </div>
                  <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => start_edit(m)} title={t("settings.edit")}>
                    <Play className="h-3.5 w-3.5" />
                  </Button>
                  <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => delete_mutation.mutate(m.id)}>
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

// ── Profiles Tab ──

const AGENT_TYPES = ["summary", "translation", "tagging"] as const;

function ProfilesTab() {
  const { t } = useTranslation();
  return (
    <div className="max-w-2xl mx-auto">
      <h3 className="text-lg font-semibold text-foreground mb-4">{t("settings.agentProfiles")}</h3>
      <p className="text-sm text-muted-foreground mb-6">{t("settings.profilesDescription")}</p>
      <div className="space-y-4">
        {AGENT_TYPES.map((agent_type) => (
          <ProfileSection key={agent_type} agent_type={agent_type} />
        ))}
      </div>
    </div>
  );
}

function ProfileSection({ agent_type }: { agent_type: string }) {
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
        data.primary_model_id ?? profile?.primary_model_id ?? null,
        data.fallback_model_id ?? profile?.fallback_model_id ?? null,
        data.target_language ?? profile?.target_language ?? null,
        data.detail_level ?? profile?.detail_level ?? null,
        data.prompt_strategy ?? profile?.prompt_strategy ?? null,
        data.concurrency_degree ?? profile?.concurrency_degree ?? null,
      ),
    onSuccess: () => query_client.invalidateQueries({ queryKey: ["agent_profile", agent_type] }),
  });

  const [primary, set_primary] = useState(profile?.primary_model_id ?? "");
  const [target_lang, set_target_lang] = useState(profile?.target_language ?? "en");
  const [detail_level, set_detail_level] = useState(profile?.detail_level ?? "medium");

  // Sync state from query
  if (profile && primary === "" && profile.primary_model_id) {
    set_primary(profile.primary_model_id);
  }

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
      <div className="grid grid-cols-2 gap-3 text-sm">
        <div>
          <label className="text-xs text-muted-foreground block mb-1">{t("settings.primaryModel")}</label>
          <select
            className="w-full h-8 rounded-md border border-border bg-background px-2 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
            value={primary}
            onChange={(e) => {
              set_primary(e.target.value);
              update_mutation.mutate({ primary_model_id: e.target.value || null });
            }}
          >
            <option value="">{t("settings.auto")}</option>
            {(models ?? []).map((m) => (
              <option key={m.id} value={m.id}>{m.name} ({m.model_name})</option>
            ))}
          </select>
        </div>
        {agent_type !== "tagging" && (
          <div>
            <label className="text-xs text-muted-foreground block mb-1">{t("settings.targetLanguage")}</label>
            <select
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
            <select
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
            <select
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

  const { data: template, isLoading } = useQuery({
    queryKey: ["custom_template", agent_type],
    queryFn: () => load_custom_template(agent_type),
  });

  const [status, set_status] = useState<string | null>(null);

  async function handle_upload() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        filters: [{ name: "YAML", extensions: ["yaml", "yml"] }],
        multiple: false,
      });
      if (selected) {
        await save_custom_template(agent_type, selected as string);
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

      {status && (
        <p className="mt-2 text-xs text-muted-foreground">{status}</p>
      )}
    </div>
  );
}
