import en from "../../src/i18n/en.json";
export function useTranslation() {
  return { t(key: string, values: Record<string, unknown> = {}) {
    const result = key.split(".").reduce<unknown>((v, k) => (v as Record<string, unknown>)?.[k], en);
    return Object.entries(values).reduce((text, [name, value]) => text.replaceAll(`{{${name}}}`, String(value)), typeof result === "string" ? result : String(values.defaultValue ?? key));
  } };
}
