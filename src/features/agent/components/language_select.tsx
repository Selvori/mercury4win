import { useTranslation } from "react-i18next";
import { useAppStore, type Locale } from "@/stores/app_store";

export function LanguageSelect() {
  const { t } = useTranslation();
  const locale = useAppStore((s) => s.locale);
  const set_locale = useAppStore((s) => s.set_locale);

  return (
    <label className="flex items-center justify-between gap-4 text-sm text-foreground">
      <span className="shrink-0">{t("common.interfaceLanguage")}</span>
      <select
        aria-label={t("common.interfaceLanguage")}
        value={locale}
        onChange={(e) => set_locale(e.target.value as Locale)}
        className="min-w-0 w-40 rounded-md border border-border bg-background px-2 py-1.5 text-foreground"
      >
        <option value="zh-Hans">{t("common.chineseName")}</option>
        <option value="en">{t("common.englishName")}</option>
      </select>
    </label>
  );
}
