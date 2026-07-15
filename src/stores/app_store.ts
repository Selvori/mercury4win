// mercury4win-linux/src/stores/app_store.ts
import { create } from "zustand";

export type Theme = "light" | "dark" | "auto";
export type Locale = "en" | "zh-Hans";

interface AppState {
  theme: Theme;
  locale: Locale;
  set_theme: (theme: Theme) => void;
  set_locale: (locale: Locale) => void;
  effective_theme: () => "light" | "dark";
}

export const useAppStore = create<AppState>((set, get) => ({
  theme: (() => {
    try {
      const stored = localStorage.getItem("mercury_theme");
      if (stored === "light" || stored === "dark" || stored === "auto") return stored;
    } catch {}
    return "auto";
  })(),
  locale: "en",
  set_theme: (theme) => {
    try { localStorage.setItem("mercury_theme", theme); } catch {}
    set({ theme });
  },
  set_locale: (locale) => set({ locale }),
  effective_theme: () => {
    const t = get().theme;
    if (t === "auto") {
      try {
        return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
      } catch { return "light"; }
    }
    return t;
  },
}));
