import { useCallback, useLayoutEffect, useRef } from "react";

// Display ownership only: invalidating a request does not cancel backend work.
export function useReaderRequest(entry_id: number) {
  const current_request = useRef<object | null>(null);

  useLayoutEffect(() => () => {
    current_request.current = null;
  }, [entry_id]);

  return useCallback(() => {
    const token = {};
    current_request.current = token;
    return {
      entry_id,
      is_current: () => current_request.current === token,
      finish: () => {
        if (current_request.current === token) current_request.current = null;
      },
    };
  }, [entry_id]);
}
