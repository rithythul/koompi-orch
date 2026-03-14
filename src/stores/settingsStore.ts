import { create } from "zustand";

export type Theme = "dark" | "light";

export interface ApiKeyEntry {
  provider: string;
  label: string;
  hasKey: boolean;
}

export interface AgentTemplate {
  id: string;
  name: string;
  command: string;
  args: string[];
  inputMode: string;
  outputMode: string;
  builtIn: boolean;
}

interface SettingsState {
  theme: Theme;
  maxConcurrentAgents: number;
  defaultAgent: string;
  defaultRole: string;
  autoReview: boolean;
  autoCheckpoint: boolean;
  apiKeys: ApiKeyEntry[];
  templates: AgentTemplate[];
  sidebarCollapsed: boolean;
  rightPanelCollapsed: boolean;
  zenMode: boolean;

  setTheme: (theme: Theme) => void;
  setMaxConcurrentAgents: (max: number) => void;
  setDefaultAgent: (agent: string) => void;
  setDefaultRole: (role: string) => void;
  setAutoReview: (auto: boolean) => void;
  setAutoCheckpoint: (auto: boolean) => void;
  setApiKeys: (keys: ApiKeyEntry[]) => void;
  updateApiKey: (provider: string, hasKey: boolean) => void;
  setTemplates: (templates: AgentTemplate[]) => void;
  addTemplate: (template: AgentTemplate) => void;
  updateTemplate: (id: string, patch: Partial<AgentTemplate>) => void;
  removeTemplate: (id: string) => void;
  toggleSidebar: () => void;
  toggleRightPanel: () => void;
  toggleZenMode: () => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  theme: "dark",
  maxConcurrentAgents: 10,
  defaultAgent: "claude-code",
  defaultRole: "implementer",
  autoReview: true,
  autoCheckpoint: true,
  apiKeys: [],
  templates: [],
  sidebarCollapsed: false,
  rightPanelCollapsed: false,
  zenMode: false,

  setTheme: (theme) => set({ theme }),
  setMaxConcurrentAgents: (max) => set({ maxConcurrentAgents: max }),
  setDefaultAgent: (agent) => set({ defaultAgent: agent }),
  setDefaultRole: (role) => set({ defaultRole: role }),
  setAutoReview: (auto) => set({ autoReview: auto }),
  setAutoCheckpoint: (auto) => set({ autoCheckpoint: auto }),
  setApiKeys: (keys) => set({ apiKeys: keys }),
  updateApiKey: (provider, hasKey) =>
    set((state) => ({
      apiKeys: state.apiKeys.map((k) =>
        k.provider === provider ? { ...k, hasKey } : k
      ),
    })),
  setTemplates: (templates) => set({ templates }),
  addTemplate: (template) =>
    set((state) => ({ templates: [...state.templates, template] })),
  updateTemplate: (id, patch) =>
    set((state) => ({
      templates: state.templates.map((t) =>
        t.id === id ? { ...t, ...patch } : t
      ),
    })),
  removeTemplate: (id) =>
    set((state) => ({
      templates: state.templates.filter((t) => t.id !== id),
    })),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  toggleRightPanel: () => set((s) => ({ rightPanelCollapsed: !s.rightPanelCollapsed })),
  toggleZenMode: () =>
    set((s) => ({
      zenMode: !s.zenMode,
      sidebarCollapsed: !s.zenMode ? true : s.sidebarCollapsed,
      rightPanelCollapsed: !s.zenMode ? true : s.rightPanelCollapsed,
    })),
}));
