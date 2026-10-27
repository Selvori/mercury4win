import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import i18n from "../../src/i18n/i18n";
import { TopicWorkspace } from "../../src/features/topics/topic_workspace";
import { useAppStore } from "../../src/stores/app_store";
import * as transport from "./auto_mock";
const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
Object.assign(window, { topic_test: { transport, client, i18n, store: useAppStore } });
export function Root() { const [show, set_show] = useState(true); return <><button onClick={() => set_show(!show)}>Toggle workspace</button><div className="flex h-[calc(100vh-24px)]">{show && <TopicWorkspace on_configure={() => { document.title = "Configure model"; }} />}</div></>; }
createRoot(document.getElementById("root")!).render(<StrictMode><QueryClientProvider client={client}><Root /></QueryClientProvider></StrictMode>);
