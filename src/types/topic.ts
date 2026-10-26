import type { EntryFilter } from "./entry";
export interface Topic { id: number; name: string; description: string; archived: boolean; rule: EntryFilter | null; rule_name: string | null; notes: string; revision: number }
export interface TopicSource { entry_id: number; title: string; url: string | null; feed: string; body: string; content_version: string; group_name: string }
export interface TopicInput { topic_id: number; name: string; description: string; revision: number; sources: TopicSource[]; baseline_id: number | null; hash: string }
export interface TopicCitation { entry_id: number; quote: string; content_version: string; start: number; end: number }
export interface TopicClaim { kind: string; text: string; citations: TopicCitation[] }
export interface TopicBrief { id: number; baseline_id: number | null; input: TopicInput; result: { claims: TopicClaim[]; warnings: string[] }; model: { route?: { provider_name: string; model_name: string }; model: string; language: string }; created_at: string }
export interface TopicDetail { topic: Topic; input: TopicInput; history: TopicBrief[]; runs: { request_id: string; status: string; error: string | null; created_at: string }[]; changed: boolean }
