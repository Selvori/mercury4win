// mercury4win/src/components/ui/dialog.tsx
// React-controlled modal overlay (replaces native <dialog> showModal).
//
// Native <dialog>'s modal focus-lock can misbehave inside Tauri/WebView2:
// a modal dialog grabs keyboard/mouse focus away from other overlays (e.g. the
// Settings panel), so an input outside the dialog loses focus on click. This
// implementation manages focus explicitly in React instead:
//   - on open, focus the first focusable control inside the dialog
//   - on close, restore focus to the element that was active before opening
//   - Escape closes the dialog
//   - clicks on the backdrop close it
//
// The public API (open / onOpenChange / children) is unchanged, so callers
// don't need edits.

import {
  type ReactNode,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useId,
  useRef,
} from "react";
import { cn } from "@/lib/utils";

interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
  /** Extra classes for the panel (e.g. wider layout like Settings). */
  className?: string;
  /** Focus the first focusable control on open. Default true. */
  focusOnOpen?: boolean;
}

const FOCUSABLE =
  'input:not([disabled]), textarea:not([disabled]), select:not([disabled]), button:not([disabled]), [tabindex]:not([tabindex="-1"])';

const TitleIdContext = createContext<string | undefined>(undefined);

export function Dialog({
  open,
  onOpenChange,
  children,
  className,
  focusOnOpen = true,
}: DialogProps) {
  const panel_ref = useRef<HTMLDivElement>(null);
  const title_id = useId();
  const restore_ref = useRef<Element | null>(null);

  // Remember which element had focus before the dialog opened, so we can
  // restore it on close.
  useEffect(() => {
    if (open) {
      restore_ref.current = document.activeElement;
    }
  }, [open]);

  // Focus the first focusable control when the dialog opens.
  useEffect(() => {
    if (!open || !focusOnOpen) return;
    const panel = panel_ref.current;
    if (!panel) return;
    const first = panel.querySelector<HTMLElement>(FOCUSABLE);
    first?.focus();
  }, [open, focusOnOpen]);

  // Escape closes the dialog.
  const handle_key_down = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape" && open) {
        e.preventDefault();
        onOpenChange(false);
      }
    },
    [open, onOpenChange],
  );

  useEffect(() => {
    document.addEventListener("keydown", handle_key_down);
    return () => document.removeEventListener("keydown", handle_key_down);
  }, [handle_key_down]);

  // Restore focus when the dialog closes.
  useEffect(() => {
    if (open) return;
    const target = restore_ref.current;
    if (target instanceof HTMLElement && target.isConnected) {
      target.focus?.();
    }
    restore_ref.current = null;
  }, [open]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onMouseDown={(e) => {
        // Only a press that starts on the backdrop closes the dialog. Events
        // from inputs and other controls may bubble here without affecting
        // their native focus behavior.
        if (e.target === e.currentTarget) {
          onOpenChange(false);
        }
      }}
    >
      <div
        ref={panel_ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={title_id}
        className={cn(
          "flex max-h-[85vh] w-full max-w-lg flex-col rounded-xl border border-border bg-popover p-0 text-popover-foreground shadow-xl",
          className,
        )}
      >
        <TitleIdContext.Provider value={title_id}>
          {children}
        </TitleIdContext.Provider>
      </div>
    </div>
  );
}

export function DialogHeader({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return <div className={cn("mb-4", className)}>{children}</div>;
}

export function DialogTitle({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  const id = useContext(TitleIdContext);
  return (
    <h2
      id={id}
      className={cn("text-lg font-semibold text-foreground", className)}
    >
      {children}
    </h2>
  );
}

export function DialogContent({
  children,
  className,
  ...props
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("p-6", className)} {...props}>
      {children}
    </div>
  );
}

export function DialogFooter({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("-mx-6 -mb-6 mt-6 flex justify-end gap-2 rounded-b-xl border-t bg-muted/50 p-4", className)}>
      {children}
    </div>
  );
}
