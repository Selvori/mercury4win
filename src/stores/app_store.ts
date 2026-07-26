// mercury4win-linux/src/stores/app_store.ts
import { create } from "zustand";

export type Theme = "light" | "dark" | "auto";
export type Locale = "en" | "zh-Hans";
export type FontFamily = "sans-serif" | "serif" | "system";

interface AppState {
  theme: Theme;
  locale: Locale;
  font_family: FontFamily;
  font_size: number;
  set_theme: (theme: Theme) => void;
  set_locale: (locale: Locale) => void;
  set_font_family: (family: FontFamily) => void;
  set_font_size: (size: number) => void;
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
  font_family: (() => {
    try {
      const stored = localStorage.getItem("mercury_font_family");
      if (stored === "sans-serif" || stored === "serif" || stored === "system") return stored;
    } catch {}
    return "sans-serif";
  })(),
  font_size: (() => {
    try {
      const stored = localStorage.getItem("mercury_font_size");
      const parsed = parseInt(stored || "16", 10);
      if (parsed >= 12 && parsed <= 24) return parsed;
    } catch {}
    return 16;
  })(),
  set_theme: (theme) => {
    try { localStorage.setItem("mercury_theme", theme); } catch {}
    set({ theme });
  },
  set_locale: (locale) => set({ locale }),
  set_font_family: (family) => {
    try { localStorage.setItem("mercury_font_family", family); } catch {}
    set({ font_family: family });
  },
  set_font_size: (size) => {
    try { localStorage.setItem("mercury_font_size", size.toString()); } catch {}
    set({ font_size: size });
  },
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
