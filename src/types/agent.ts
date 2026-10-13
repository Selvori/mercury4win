// mercury4win/src/types/agent.ts

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
  state: "idle" | "running" | "running_waiting" | "cancelling";
  current_entry_id: number | null;
  current_request_id: string | null;
  waiting_entry_id: number | null;
  waiting_request_id: string | null;
  queue_depth: number;
}

export type TaskType = "summary" | "translation" | "tagging";

export interface ProviderConfig {
  name: string;
  base_url: string;
  display_name?: string;
  api_key?: string;
}

export interface AgentResult {
  source_hash: string;
  text: string | null;
  segments: (string | null)[];
  complete: boolean;
  model_name: string;
  segment_ids?: string[];
  segment_errors?: (string | null)[];
  request_route?: RequestRoute | null;
  segment_routes?: (RequestRoute | null)[];
}

export interface SourceSegment {
  source_segment_id: string;
  order_index: number;
  segment_type: "p" | "ul" | "ol";
  source_html: string;
  source_text: string;
}
export interface TranslationDocument {
  source_hash: string;
  segmenter_version: string;
  html: string;
  source_segments: SourceSegment[];
  result: AgentResult | null;
}

export interface RequestRoute {
  provider_id: string;
  provider_name: string;
  model_id: string;
  model_name: string;
}

export interface UsageFilter { provider_id?: string; model_id?: string; agent_type?: string }
export interface UsageMetrics {
  request_count: number; succeeded: number; failed: number; cancelled: number; timed_out: number;
  missing_usage: number; prompt_tokens: number; completion_tokens: number; total_tokens: number;
  legacy_events: number; legacy_tokens: number;
}
export interface UsageGroup { id: string; label: string; archived: boolean; metrics: UsageMetrics }
export interface UsageReport { from_utc: string; to_utc: string; totals: UsageMetrics; groups: UsageGroup[]; daily: UsageGroup[]; persistence_failures: number }
export interface UsageContext { dimension: "provider" | "model" | "agent"; filter: UsageFilter; label?: string }
