import { useEffect } from "react";
import { useAppStore } from "@/stores/app_store";

/** One appearance subscription for the app and its reactive Reader consumers. */
export function useTheme() {
  const effective = useAppStore((s) => s.effective_theme());
  const set_system_theme = useAppStore((s) => s.set_system_theme);

  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => set_system_theme(media.matches ? "dark" : "light");
    media.addEventListener("change", update);
    // Re-sample after subscribing: appearance may have changed since store init.
    update();
    return () => media.removeEventListener("change", update);
  }, [set_system_theme]);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", effective === "dark");
  }, [effective]);
}
