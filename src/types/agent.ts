// mercury4win-linux/src/types/agent.ts

export interface ProviderProfile {
  id: string;
  name: string;
  base_url: string;
  display_name: string | null;
  is_default: boolean;
  is_enabled: boolean;
  is_archived: boolean;
  created_at: string;
}

export interface ModelProfile {
  id: string;
  provider_id: string;
  name: string;
  model_name: string;
  temperature: number | null;
  top_p: number | null;
  max_tokens: number | null;
  is_streaming: boolean;
  supports_summary: boolean;
  supports_translation: boolean;
  supports_tagging: boolean;
  is_default: boolean;
  is_enabled: boolean;
  is_archived: boolean;
  last_tested_at: string | null;
}

export interface AgentProfile {
  agent_type: string;
  primary_model_id: string | null;
  fallback_model_id: string | null;
  target_language: string | null;
  detail_level: string | null;
  prompt_strategy: string | null;
  concurrency_degree: number | null;
}

export interface AgentStatus {
  agent_type: string;
  state: "idle" | "running" | "waiting";
  current_entry_id: number | null;
  queue_depth: number;
}

export type TaskType = "summary" | "translation" | "tagging";

export interface ProviderConfig {
  name: string;
  base_url: string;
  display_name?: string;
  api_key?: string;
}
