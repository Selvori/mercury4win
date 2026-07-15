// mercury4win-linux/src/App.tsx
import { useEffect } from "react";
import { AppShell } from "@/components/layout/app_shell";
import { useAppStore } from "@/stores/app_store";

function App() {
  const theme = useAppStore((s) => s.theme);
  const effective = useAppStore((s) => s.effective_theme());

  useEffect(() => {
    const root = document.documentElement;
    root.classList.toggle("dark", effective === "dark");
  }, [effective]);

  // Listen for system theme changes when in auto mode
  useEffect(() => {
    if (theme !== "auto") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => {
      const root = document.documentElement;
      root.classList.toggle("dark", mq.matches);
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme]);

  return <AppShell />;
}

export default App;
