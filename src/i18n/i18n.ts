// mercury4win/src/i18n/i18n.ts
// i18next configuration

import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./en.json";
import zhHans from "./zh-Hans.json";
import { useAppStore } from "@/stores/app_store";

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    "zh-Hans": { translation: zhHans },
  },
  lng: useAppStore.getState().locale,
  fallbackLng: "en",
  interpolation: {
    escapeValue: false, // React already escapes
  },
});

// Keep all locale changes (including callers outside React) on one path.
const unsubscribe_locale = useAppStore.subscribe((state, previous) => {
  if (state.locale !== previous.locale) void i18n.changeLanguage(state.locale);
});

function update_document_language(language: string) {
  document.documentElement.lang = language;
}
update_document_language(i18n.language);
i18n.on("languageChanged", update_document_language);

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    unsubscribe_locale();
    i18n.off("languageChanged", update_document_language);
  });
}

export default i18n;
