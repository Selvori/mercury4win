// mercury4win/src/stores/app_store.ts
import { create } from "zustand";

export type Theme = "light" | "dark" | "auto";
export type Locale = "en" | "zh-Hans";
export type FontFamily = "sans-serif" | "serif" | "system";
export type ReaderStyle = "classic" | "paper";
type ColorScheme = "light" | "dark";

function system_color_scheme(): ColorScheme {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  } catch { return "light"; }
}

interface AppState {
  theme: Theme;
  system_theme: ColorScheme;
  reader_style: ReaderStyle;
  locale: Locale;
  font_family: FontFamily;
  font_size: number;
  set_theme: (theme: Theme) => void;
  set_system_theme: (theme: ColorScheme) => void;
  set_reader_style: (style: ReaderStyle) => void;
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
  system_theme: system_color_scheme(),
  reader_style: (() => {
    try {
      if (localStorage.getItem("mercury_reader_style") === "paper") return "paper";
    } catch {}
    return "classic";
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
    set({ theme, system_theme: system_color_scheme() });
  },
  set_system_theme: (system_theme) => set({ system_theme }),
  set_reader_style: (reader_style) => {
    try { localStorage.setItem("mercury_reader_style", reader_style); } catch {}
    set({ reader_style });
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
    const { theme, system_theme } = get();
    return theme === "auto" ? system_theme : theme;
  },
}));
