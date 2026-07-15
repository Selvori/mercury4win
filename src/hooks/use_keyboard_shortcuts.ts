// mercury4win-linux/src/hooks/use_keyboard_shortcuts.ts
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
      // Don't handle shortcuts when in an input/textarea
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      const key = e.key.toLowerCase();
      const ctrl = e.ctrlKey || e.metaKey;
      const shift = e.shiftKey;

      if (ctrl && key === "n") {
        e.preventDefault();
        shortcuts.on_add_feed?.();
        return;
      }

      if (shift && key === "a") {
        e.preventDefault();
        shortcuts.on_mark_all_read?.();
        return;
      }

      if (shift && key === "r") {
        e.preventDefault();
        shortcuts.on_refresh?.();
        return;
      }

      switch (key) {
        case "j":
          e.preventDefault();
          shortcuts.on_down?.();
          break;
        case "k":
          e.preventDefault();
          shortcuts.on_up?.();
          break;
        case "enter":
        case "o":
          e.preventDefault();
          shortcuts.on_open?.();
          break;
        case "m":
          e.preventDefault();
          shortcuts.on_toggle_read?.();
          break;
        case "s":
          e.preventDefault();
          shortcuts.on_toggle_star?.();
          break;
        case "r":
          shortcuts.on_toggle_reader?.();
          break;
        case "t":
          shortcuts.on_toggle_translation?.();
          break;
        case "n":
          shortcuts.on_toggle_note?.();
          break;
      }
    }

    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [shortcuts]);
}
