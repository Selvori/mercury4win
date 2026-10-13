// mercury4win/src/hooks/use_keyboard_shortcuts.ts
// Keyboard shortcuts for Mercury

import { useEffect } from "react";

export interface ShortcutMap {
  /** Move selection down (j) */
  on_down?: () => void;
  /** Move selection up (k) */
  on_up?: () => void;
  /** Open/select current entry (o / Enter) */
  on_open?: () => void;
  /** Toggle read status (m) */
  on_toggle_read?: () => void;
  /** Toggle star (s) */
  on_toggle_star?: () => void;
  /** Mark all read in current feed (shift+a) */
  on_mark_all_read?: () => void;
  /** Toggle reader/summary/translation panel (r / t / n) */
  on_toggle_reader?: () => void;
  on_toggle_translation?: () => void;
  on_toggle_note?: () => void;
  /** Refresh feed (shift+r) */
  on_refresh?: () => void;
  /** Add feed (ctrl+n) */
  on_add_feed?: () => void;
}

export function useKeyboardShortcuts(shortcuts: ShortcutMap) {
  useEffect(() => {
    function handler(e: KeyboardEvent) {
      if (e.defaultPrevented || e.isComposing || e.keyCode === 229 || e.repeat || e.altKey) return;
      const target = e.target instanceof Element ? e.target : null;
      if (target?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"]')) return;
      if (document.querySelector('dialog[open], [role="dialog"][aria-modal="true"]')) return;
      if (document.querySelector('[data-digest-selecting="true"]')) return;

      const key = e.key.toLowerCase();
      const ctrl = e.ctrlKey || e.metaKey;
      const shift = e.shiftKey;

      if (ctrl && key === "n" && shortcuts.on_add_feed) {
        e.preventDefault();
        shortcuts.on_add_feed?.();
        return;
      }
      if (ctrl) return;

      if (shift && key === "a" && shortcuts.on_mark_all_read) {
        e.preventDefault();
        shortcuts.on_mark_all_read?.();
        return;
      }

      if (shift && key === "r" && shortcuts.on_refresh) {
        e.preventDefault();
        shortcuts.on_refresh?.();
        return;
      }
      if (shift) return;

      switch (key) {
        case "j":
          if (!shortcuts.on_down) return;
          e.preventDefault();
          shortcuts.on_down?.();
          break;
        case "k":
          if (!shortcuts.on_up) return;
          e.preventDefault();
          shortcuts.on_up?.();
          break;
        case "enter":
        case "o":
          if (!shortcuts.on_open || target?.closest('button, a, [role="button"]')) return;
          e.preventDefault();
          shortcuts.on_open?.();
          break;
        case "m":
          if (!shortcuts.on_toggle_read) return;
          e.preventDefault();
          shortcuts.on_toggle_read?.();
          break;
        case "s":
          if (!shortcuts.on_toggle_star) return;
          e.preventDefault();
          shortcuts.on_toggle_star?.();
          break;
        case "r":
          if (!shortcuts.on_toggle_reader) return;
          e.preventDefault();
          shortcuts.on_toggle_reader?.();
          break;
        case "t":
          if (!shortcuts.on_toggle_translation) return;
          e.preventDefault();
          shortcuts.on_toggle_translation?.();
          break;
        case "n":
          if (!shortcuts.on_toggle_note) return;
          e.preventDefault();
          shortcuts.on_toggle_note?.();
          break;
      }
    }

    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [shortcuts]);
}
