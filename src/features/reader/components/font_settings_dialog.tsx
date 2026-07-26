// mercury4win-linux/src/features/reader/components/font_settings_dialog.tsx
// Font settings dialog for reader customization

import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useAppStore, type FontFamily } from "@/stores/app_store";

interface Props {
  open: boolean;
  on_close: () => void;
}

const PREVIEW_FONTS: Record<FontFamily, string> = {
  "sans-serif": '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  serif: '"Georgia", "Times New Roman", serif',
  system: '-apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", sans-serif',
};

export function FontSettingsDialog({ open, on_close }: Props) {
  const { t } = useTranslation();
  const font_family = useAppStore((s) => s.font_family);
  const font_size = useAppStore((s) => s.font_size);
  const set_font_family = useAppStore((s) => s.set_font_family);
  const set_font_size = useAppStore((s) => s.set_font_size);

  const font_families: { value: FontFamily; label: string }[] = [
    { value: "sans-serif", label: t("fontSettings.sansSerif") },
    { value: "serif", label: t("fontSettings.serif") },
    { value: "system", label: t("fontSettings.system") },
  ];

  return (
    <Dialog open={open} onOpenChange={(o) => !o && on_close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("fontSettings.title")}</DialogTitle>
        </DialogHeader>

        <div className="space-y-5 py-2">
          {/* Font family */}
          <div>
            <label className="mb-2 block text-sm font-medium">
              {t("fontSettings.fontFamily")}
            </label>
            <div className="grid grid-cols-3 gap-2">
              {font_families.map((ff) => (
                <button
                  key={ff.value}
                  onClick={() => set_font_family(ff.value)}
                  style={{ fontFamily: PREVIEW_FONTS[ff.value] }}
                  className={`rounded-md border px-3 py-2 text-sm transition-colors ${
                    font_family === ff.value
                      ? "border-primary bg-primary/10 text-primary font-medium"
                      : "border-border bg-background hover:bg-muted"
                  }`}
                >
                  {ff.label}
                </button>
              ))}
            </div>
          </div>

          {/* Font size */}
          <div>
            <label className="mb-2 block text-sm font-medium">
              {t("fontSettings.fontSize")}
              <span className="ml-2 text-muted-foreground">{font_size}px</span>
            </label>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="icon-sm"
                onClick={() => set_font_size(Math.max(12, font_size - 1))}
                disabled={font_size <= 12}
                aria-label={t("fontSettings.decrease")}
              >
                −
              </Button>
              <input
                type="range"
                min="12"
                max="24"
                step="1"
                value={font_size}
                onChange={(e) => set_font_size(parseInt(e.target.value, 10))}
                className="flex-1 accent-primary"
              />
              <Button
                variant="outline"
                size="icon-sm"
                onClick={() => set_font_size(Math.min(24, font_size + 1))}
                disabled={font_size >= 24}
                aria-label={t("fontSettings.increase")}
              >
                +
              </Button>
            </div>
            <div className="mt-1 flex justify-between text-xs text-muted-foreground">
              <span>12px</span>
              <span>24px</span>
            </div>
          </div>

          {/* Preview */}
          <div className="rounded-md border border-border bg-muted/50 p-4">
            <p className="mb-2 text-xs text-muted-foreground">
              {t("fontSettings.preview")}
            </p>
            <p
              style={{
                fontFamily: PREVIEW_FONTS[font_family],
                fontSize: `${font_size}px`,
                lineHeight: 1.8,
              }}
            >
              {t("fontSettings.previewText")}
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={on_close}>
            {t("fontSettings.close")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
