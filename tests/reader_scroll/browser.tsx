import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import en from "../../src/i18n/en.json";
import { ReaderPane } from "../../src/features/reader/components/reader_pane";
import type { Entry } from "../../src/types";

await i18next.use(initReactI18next).init({
  lng: "en", resources: { en: { translation: en } }, interpolation: { escapeValue: false },
});
const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
createRoot(document.getElementById("app")!).render(
  <StrictMode><QueryClientProvider client={client}>
    <ReaderPane entry={{ id: 1, title: "Scroll regression", url: null } as Entry} />
  </QueryClientProvider></StrictMode>,
);
