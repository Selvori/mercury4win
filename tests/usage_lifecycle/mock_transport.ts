const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value));
import type { UsageReport } from "../../src/types/agent";
import { fixture } from "./fixture";
export class Channel<T> { onmessage = (_message: T) => {}; }
export const calls: Array<{ command: string; args: Record<string, unknown> }> = [];
export let fail_report = false;
export let fail_archive = false;
export function failure(report = false, archive = false) { fail_report = report; fail_archive = archive; }
const providers = [{ id: "primary", name: "Primary", base_url: "https://fixture.test", is_enabled: true, is_archived: false }];
const models = [{ id: "model", provider_id: "primary", name: "Model", model_name: "model", is_enabled: true, is_archived: false, supports_summary: true, supports_translation: true, supports_tagging: true }];
export function restore() { models.splice(1); providers[0].is_archived = false; providers[0].is_enabled = true; models[0].is_archived = false; models[0].is_enabled = true; calls.length = 0; persistence_failures = undefined; failure(); }
export let delayed: ((value: UsageReport) => void) | undefined;
let delay_next = false;
let persistence_failures: number | undefined;
export function set_persistence_failures(value: number | undefined) { persistence_failures = value; }
export function delay_report() { delay_next = true; }
export function release_report() { delayed?.(clone(fixture)); delayed = undefined; }
export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  calls.push({ command, args });
  if (command === "get_usage_report") {
    if (fail_report) throw new Error("fixture report failed");
    if (delay_next) { delay_next = false; return new Promise(resolve => { delayed = resolve as (value: UsageReport) => void; }); }
    const value = clone(fixture);
    if (persistence_failures !== undefined) value.persistence_failures = persistence_failures;
    if (args.dimension === "agent") value.groups = [{ id: "summary", label: "summary", archived: false, metrics: value.totals }];
    return value as T;
  }
  if (command === "list_providers") return clone(providers.filter(p => !p.is_archived)) as T;
  if (command === "list_models") return clone(models.filter(m => !m.is_archived && !providers[0].is_archived)) as T;
  if (command === "set_configuration_enabled") {
    const target = args.entity === "provider" ? providers[0] : models[0]; target.is_enabled = args.enabled as boolean; return undefined as T;
  }
  if (command === "delete_provider" || command === "delete_model") {
    if (fail_archive) throw new Error("fixture archive failed");
    if (command === "delete_provider") providers[0].is_archived = true;
    models[0].is_archived = true; return undefined as T;
  }
  if (command === "add_model") {
    models.push({ id:"fresh", provider_id:"primary",name:args.name as string,model_name:args.modelName as string,is_enabled:true,is_archived:false,supports_summary:args.supportsSummary as boolean,supports_translation:args.supportsTranslation as boolean,supports_tagging:args.supportsTagging as boolean });
    return undefined as T;
  }
  if (command === "get_agent_profile") return { agent_type: args.agentType, primary_model_id: "model", fallback_model_id: null, target_language: "en", detail_level: "medium" } as T;
  if (command === "get_setting") return null as T;
  if (command === "get_agent_fingerprint") return "synthetic-config" as T;
  throw new Error(`Unhandled transport: ${command}`);
}
