import { useEffect } from "react";
import { BrowserRouter } from "react-router-dom";
import { ThreePanel } from "../components/layout/ThreePanel";
import { CommandPalette } from "../components/layout/CommandPalette";
import { ToastContainer } from "../components/notifications/ToastContainer";
import { useSettingsStore } from "../stores/settingsStore";
import { useAgentEvents } from "../hooks/useAgentEvents";
import { useBootstrap } from "../hooks/useBootstrap";

function App() {
  const theme = useSettingsStore((s) => s.theme);
  useAgentEvents();
  useBootstrap();

  useEffect(() => {
    const root = document.documentElement;
    root.classList.remove("dark", "light");
    root.classList.add(theme);
  }, [theme]);

  return (
    <BrowserRouter>
      <ThreePanel />
      <CommandPalette />
      <ToastContainer />
    </BrowserRouter>
  );
}

export default App;
