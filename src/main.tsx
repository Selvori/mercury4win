import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App.tsx";
import "./styles/globals.css";
import "./i18n/i18n";

const query_client = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: 1,
      // Refetching on window focus can rebuild inputs and steal focus while
      // the user is typing (e.g. right after clicking an input). Feeds refresh
      // on-demand via the Sync button, so disable the background refetch.
      refetchOnWindowFocus: false,
    },
  },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={query_client}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
);
