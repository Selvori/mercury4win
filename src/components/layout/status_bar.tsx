import { useTranslation } from "react-i18next";

export function StatusBar() {
  const { t } = useTranslation();
  return (
    <div className="flex h-7 items-center gap-4 border-t border-border bg-sidebar px-4 text-xs text-muted-foreground">
      <span>{t("common.ready")}</span>
    </div>
  );
}
