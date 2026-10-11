// mercury4win/src/components/layout/theme_toggle.tsx
// Theme toggle button (light/dark/auto)

import { Sun, Moon, Monitor } from "lucide-react";
import { useAppStore, type Theme } from "@/stores/app_store";
import { Button } from "@/components/ui/button";

export function ThemeToggle() {
  const theme = useAppStore((s) => s.theme);
  const set_theme = useAppStore((s) => s.set_theme);

  const cycle_theme = () => {
    const next: Record<Theme, Theme> = {
      light: "dark",
      dark: "auto",
      auto: "light",
    };
    set_theme(next[theme]);
  };

  const icon_map: Record<Theme, typeof Sun> = {
    light: Sun,
    dark: Moon,
    auto: Monitor,
  };

  const Icon = icon_map[theme];

  return (
    <Button
      variant="ghost"
      size="icon-sm"
      onClick={cycle_theme}
      title={`Theme: ${theme}`}
      aria-label="Toggle theme"
    >
      <Icon className="h-4 w-4" />
    </Button>
  );
}
