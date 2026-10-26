import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "../../src/i18n/i18n";
import App from "../../src/App";
import { useAppStore } from "../../src/stores/app_store";
import * as transport from "./mock_transport";

const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
Object.assign(window, { search_test: { transport, store: useAppStore, client } });
createRoot(document.getElementById("root")!).render(<StrictMode><QueryClientProvider client={client}><App /></QueryClientProvider></StrictMode>);
