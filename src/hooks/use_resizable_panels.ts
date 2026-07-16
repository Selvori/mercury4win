// mercury4win-linux/src/hooks/use_resizable_panels.ts
// Draggable panel splitter hook for resizable layouts

import { useState, useCallback, useRef } from "react";

interface UseResizablePanelsOptions {
  defaultSizes: number[]; // initial panel sizes in pixels
  minSize?: number;       // minimum panel size, default 160
}

export function useResizablePanels({
  defaultSizes,
  minSize = 160,
}: UseResizablePanelsOptions) {
  const [sizes, set_sizes] = useState<number[]>(defaultSizes);
  const dragging = useRef<{ index: number; start_x: number; start_sizes: number[] } | null>(null);

  const start_resize = useCallback(
    (splitter_index: number) => (e: React.MouseEvent) => {
      e.preventDefault();
      dragging.current = {
        index: splitter_index,
        start_x: e.clientX,
        start_sizes: [...sizes],
      };
    },
    [sizes],
  );

  const on_mouse_move = useCallback((e: MouseEvent) => {
    if (!dragging.current) return;
    const { index, start_x, start_sizes } = dragging.current;
    const delta = e.clientX - start_x;

    const new_sizes = [...start_sizes];
    // Splitter between panel[i] and panel[i+1]
    // panel[i] gets +delta, panel[i+1] gets -delta
    const new_left = Math.max(minSize, start_sizes[index] + delta);
    const right_index = index + 1;
    const total = start_sizes[index] + start_sizes[right_index];
    const new_right = Math.max(minSize, total - new_left);
    new_sizes[index] = total - new_right;
    new_sizes[right_index] = new_right;

    set_sizes(new_sizes);
  }, [minSize]);

  const stop_resize = useCallback(() => {
    dragging.current = null;
  }, []);

  return { sizes, start_resize, on_mouse_move, stop_resize };
}
