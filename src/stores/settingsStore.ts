import { create } from "zustand";
import type { AppConfig } from "../lib/ipc";

interface SettingsState {
  config: AppConfig | null;
  sidebarCollapsed: boolean;
  rightPanelCollapsed: boolean;
  zenMode: boolean;
  setConfig: (config: AppConfig) => void;
  toggleSidebar: () => void;
  toggleRightPanel: () => void;
  toggleZenMode: () => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  config: null,
  sidebarCollapsed: false,
  rightPanelCollapsed: false,
  zenMode: false,
  setConfig: (config) => set({ config }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  toggleRightPanel: () => set((s) => ({ rightPanelCollapsed: !s.rightPanelCollapsed })),
  toggleZenMode: () =>
    set((s) => ({
      zenMode: !s.zenMode,
      sidebarCollapsed: !s.zenMode ? true : s.sidebarCollapsed,
      rightPanelCollapsed: !s.zenMode ? true : s.rightPanelCollapsed,
    })),
}));
