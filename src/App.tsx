// mercury4win/src/App.tsx
import { AppShell } from "@/components/layout/app_shell";
import { useTheme } from "@/hooks/use_theme";

function App() {
  useTheme();

  return <AppShell />;
}

export default App;
