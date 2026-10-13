// mercury4win/src/components/ui/dialog.tsx
// Modals share a stack so only the top layer owns focus and Escape.

import {
  type ReactNode,
  createContext,
  useContext,
  useLayoutEffect,
  useId,
  useRef,
} from "react";
import { createPortal } from "react-dom";
import { cn } from "@/lib/utils";

interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
  /** Extra classes for the panel (e.g. wider layout like Settings). */
  className?: string;
  /** Focus the first focusable control on open. Default true. */
  focusOnOpen?: boolean;
  aria_label?: string;
}

const FOCUSABLE =
  'a[href], summary, input:not([type="hidden"]), textarea, select, button, [tabindex], [contenteditable="true"]';

interface ModalLayer { panel: HTMLDivElement; overlay: HTMLDivElement; depth: number; restore: Element | null }
const layers: ModalLayer[] = [];
const background = new Map<HTMLElement, boolean>();
const DepthContext = createContext(0);
function top_layer() { return layers.at(-1); }
function controls(panel: HTMLElement) {
  return [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((element) =>
    element.tabIndex >= 0 && !element.matches(':disabled') && !element.closest('[inert], [hidden], [aria-hidden="true"]') &&
    visible(element));
}
function visible(element: HTMLElement) {
  for (let ancestor: HTMLElement | null = element; ancestor; ancestor = ancestor.parentElement) {
    const style = getComputedStyle(ancestor);
    if (style.display === "none" || style.visibility === "hidden" || style.visibility === "collapse") return false;
    if (ancestor.tagName === "DETAILS" && !ancestor.hasAttribute("open") &&
      !ancestor.querySelector(":scope > summary")?.contains(element)) return false;
  }
  return true;
}
function focus_layer(layer: ModalLayer) { (controls(layer.panel)[0] ?? layer.panel).focus(); }
function update_background() {
  for (const [element, was_inert] of background) element.inert = was_inert;
  background.clear();
  const top = top_layer();
  if (!top) return;
  for (const [index, layer] of layers.entries()) {
    layer.overlay.style.zIndex = String(50 + index);
    layer.panel.setAttribute("aria-modal", layer === top ? "true" : "false");
  }
  for (const element of document.body.children) {
    if (element instanceof HTMLElement && element !== top.overlay) {
      background.set(element, element.inert);
      element.inert = true;
    }
  }
}

const TitleIdContext = createContext<string | undefined>(undefined);

export function Dialog({
  open,
  onOpenChange,
  children,
  className,
  focusOnOpen = true,
  aria_label,
}: DialogProps) {
  const panel_ref = useRef<HTMLDivElement>(null);
  const overlay_ref = useRef<HTMLDivElement>(null);
  const title_id = useId();
  const depth = useContext(DepthContext);
  const restore_ref = useRef<Element | null>(null);
  const was_open = useRef(false);
  const change_ref = useRef(onOpenChange);
  change_ref.current = onOpenChange;
  if (open && !was_open.current) restore_ref.current = document.activeElement;
  was_open.current = open;

  useLayoutEffect(() => {
    if (!open) return;
    const panel = panel_ref.current;
    const overlay = overlay_ref.current;
    if (!panel || !overlay) return;
    const layer = { panel, overlay, depth, restore: restore_ref.current };
    layers.push(layer);
    layers.sort((a, b) => a.depth - b.depth);
    update_background();
    if (top_layer() === layer && (focusOnOpen || !panel.contains(document.activeElement))) focus_layer(layer);
    function key_down(e: KeyboardEvent) {
      if (top_layer() !== layer || e.defaultPrevented || e.isComposing || e.keyCode === 229) return;
      if (e.key === "Escape") {
        if (e.repeat) return;
        e.preventDefault();
        e.stopPropagation();
        change_ref.current(false);
      } else if (e.key === "Tab") {
        const items = controls(panel!);
        const index = items.indexOf(document.activeElement as HTMLElement);
        if (!items.length || index < 0 || (e.shiftKey ? index === 0 : index === items.length - 1)) {
          e.preventDefault();
          (e.shiftKey ? items.at(-1) ?? panel! : items[0] ?? panel!).focus();
        }
      }
    }
    function focus_in(e: FocusEvent) {
      if (top_layer() === layer && !panel!.contains(e.target as Node)) focus_layer(layer);
    }
    // An initially empty/disabled dialog may receive its first control later.
    const observer = new MutationObserver(() => {
      const active = document.activeElement;
      if (top_layer() === layer && (active === panel || !panel!.contains(active) || (active instanceof HTMLElement && (active.matches(':disabled') || !visible(active))))) focus_layer(layer);
    });
    observer.observe(panel, { childList: true, subtree: true, attributes: true, attributeFilter: ["disabled", "hidden", "tabindex", "style", "class", "open"] });
    document.addEventListener("keydown", key_down);
    document.addEventListener("focusin", focus_in);
    return () => {
      observer.disconnect();
      document.removeEventListener("keydown", key_down);
      document.removeEventListener("focusin", focus_in);
      const was_top = top_layer() === layer;
      layers.splice(layers.indexOf(layer), 1);
      update_background();
      if (!was_top) return;
      const next = top_layer();
      const target = layer.restore;
      if (target instanceof HTMLElement && target.isConnected && target.tabIndex >= 0 && !target.closest('[inert], [hidden]') && !target.matches(':disabled') && visible(target) && (!next || next.panel.contains(target))) target.focus();
      else if (next) focus_layer(next);
      else controls(document.body)[0]?.focus();
    };
  }, [open, depth, focusOnOpen]);

  if (!open) return null;

  return createPortal(
    <div
      ref={overlay_ref}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onMouseDown={(e) => {
        // Only a press that starts on the backdrop closes the dialog. Events
        // from inputs and other controls may bubble here without affecting
        // their native focus behavior.
        if (e.target === e.currentTarget && top_layer()?.panel === panel_ref.current) {
          onOpenChange(false);
        }
      }}
    >
      <div
        ref={panel_ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={aria_label ? undefined : title_id}
        aria-label={aria_label}
        tabIndex={-1}
        className={cn(
          "flex max-h-[85vh] w-full max-w-lg flex-col rounded-xl border border-border bg-popover p-0 text-popover-foreground shadow-xl",
          className,
        )}
      >
        <DepthContext.Provider value={depth + 1}><TitleIdContext.Provider value={title_id}>
          {children}
        </TitleIdContext.Provider></DepthContext.Provider>
      </div>
    </div>, document.body,
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
