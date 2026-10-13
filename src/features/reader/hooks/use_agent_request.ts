import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { abandon_agent_waiting, cancel_agent_task, get_agent_status } from "@/lib/tauri_bindings";
import type { TaskType } from "@/types/agent";
import { useReaderRequest } from "./use_reader_request";

type Phase = "idle" | "starting" | "waiting" | "running" | "cancelling" | "cancelled";

export function useAgentRequest(entry_id: number, task_type: TaskType) {
  const begin_owner = useReaderRequest(entry_id);
  const [phase, set_phase] = useState<Phase>("idle");
  const [cancel_error, set_cancel_error] = useState<string | null>(null);
  const active = useRef<{ request_id: string; is_current: () => boolean } | null>(null);
  const query_client = useQueryClient();
  const [background_cancel_id, set_background_cancel_id] = useState<string | null>(null);
  const { data: status, isFetching: status_fetching, error: status_error } = useQuery({
    queryKey: ["agent_status", task_type],
    queryFn: () => get_agent_status(task_type),
    refetchInterval: 1000,
    retry: false,
  });
  const background_running = !active.current && status?.current_entry_id === entry_id && !!status.current_request_id;
  useEffect(() => {
    if (background_cancel_id && status && status.current_request_id !== background_cancel_id) {
      set_background_cancel_id(null);
      set_phase("cancelled");
    }
  }, [background_cancel_id, status]);

  useLayoutEffect(() => () => {
    const request = active.current;
    active.current = null;
    if (request) void abandon_agent_waiting(task_type, request.request_id).catch(() => {});
  }, [entry_id, task_type]);

  const begin_request = useCallback(() => {
    const owner = begin_owner();
    const request_id = crypto.randomUUID();
    active.current = { request_id, is_current: owner.is_current };
    set_cancel_error(null);
    set_phase("starting");
    return {
      ...owner,
      request_id,
      handle_event: (message: { type?: string; request_id?: string; state?: string }) => {
        if (message.type !== "task_state") return false;
        if (owner.is_current() && message.request_id === request_id) {
          if (message.state === "waiting" || message.state === "running" || message.state === "cancelled") {
            set_phase(message.state);
          }
        }
        return true;
      },
      finish: (cancelled = false) => {
        if (!owner.is_current()) return;
        active.current = null;
        set_phase((previous) => cancelled || previous === "cancelled" ? "cancelled" : "idle");
        owner.finish();
        void query_client.invalidateQueries({ queryKey: ["agent_status", task_type] });
      },
    };
  }, [begin_owner, query_client, task_type]);

  const cancel = useCallback(async () => {
    const request = active.current ?? (background_running && status?.current_request_id ? {
      request_id: status.current_request_id,
      is_current: begin_owner().is_current,
    } : null);
    if (!request) return;
    const background = !active.current;
    const previous_phase = phase;
    if (background) set_background_cancel_id(request.request_id);
    set_phase("cancelling");
    set_cancel_error(null);
    try {
      await cancel_agent_task(task_type, request.request_id);
      void query_client.invalidateQueries({ queryKey: ["agent_status", task_type] });
    } catch (error) {
      if (!request.is_current()) return;
      set_cancel_error(String(error));
      set_phase(background ? "idle" : previous_phase);
      if (background) set_background_cancel_id(null);
    }
  }, [task_type, background_running, status, query_client, begin_owner, phase]);

  return {
    begin_request, phase, cancel, cancel_error, background_running,
    status_ready: status !== undefined && !status_fetching && !status_error,
    is_pending: background_running || phase === "starting" || phase === "waiting" || phase === "running" || phase === "cancelling",
  };
}
