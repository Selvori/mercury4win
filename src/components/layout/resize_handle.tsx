// mercury4win-linux/src/components/layout/resize_handle.tsx
// Draggable resize handle — highlighted only while actively dragging

import { cn } from "@/lib/utils";

interface Props {
  on_mouse_down: (e: React.MouseEvent) => void;
  dragging: boolean;
}

export function ResizeHandle({ on_mouse_down, dragging }: Props) {
  return (
    <div
      className={cn(
        "relative w-1.5 shrink-0 cursor-col-resize select-none",
        dragging && "bg-primary/50",
      )}
      onMouseDown={on_mouse_down}
    >
      {/* Invisible wider hit area for easier grabbing */}
      <div className="absolute inset-y-0 -left-1 -right-1" />
    </div>
  );
}
