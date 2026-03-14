import { useEffect } from "react";
import { Sidebar } from "./Sidebar";
import { CenterPanel } from "./CenterPanel";
import { RightPanel } from "./RightPanel";
import { useSettingsStore } from "../../stores/settingsStore";
import { registerKeybinding } from "../../lib/keybindings";

export function ThreePanel() {
  const { sidebarCollapsed, rightPanelCollapsed, zenMode, toggleSidebar, toggleRightPanel, toggleZenMode } =
    useSettingsStore();

  useEffect(() => {
    registerKeybinding("b", ["ctrl"], toggleSidebar, "Toggle sidebar");
    registerKeybinding("j", ["ctrl"], toggleRightPanel, "Toggle right panel");
    registerKeybinding("z", ["ctrl", "shift"], toggleZenMode, "Toggle zen mode");
  }, [toggleSidebar, toggleRightPanel, toggleZenMode]);

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar collapsed={zenMode || sidebarCollapsed} />
      <CenterPanel />
      <RightPanel collapsed={zenMode || rightPanelCollapsed} />
    </div>
  );
}
