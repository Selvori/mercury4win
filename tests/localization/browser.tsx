import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "../../src/i18n/i18n";
import App from "../../src/App";
import { useAppStore } from "../../src/stores/app_store";
import * as native from "./mock_transport";

// Expose only synthetic transport observations and store actions to the runner.
Object.assign(window, { localization_test: { native, store: useAppStore } });
const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
createRoot(document.getElementById("root")!).render(
  <StrictMode><QueryClientProvider client={client}><App /></QueryClientProvider></StrictMode>,
);
