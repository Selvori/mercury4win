import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import en from "../../src/i18n/en.json";
import { ReaderPane } from "../../src/features/reader/components/reader_pane";
import type { Entry } from "../../src/types";
import * as native from "./mock_transport";

await i18next.use(initReactI18next).init({ lng: "en", resources: { en: { translation: en } }, interpolation: { escapeValue: false } });
const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
export function App() {
  const [entry_id, set_entry] = useState(1);
  Object.assign(window, { regression: { ...native, set_entry } });
  return <ReaderPane entry={{ id: entry_id, title: `Scroll article ${entry_id}`, url: null } as Entry} />;
}
createRoot(document.getElementById("app")!).render(
  <StrictMode><QueryClientProvider client={client}><App /></QueryClientProvider></StrictMode>,
);
