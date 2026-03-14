import { create } from "zustand";

export type AgentSessionStatus = "running" | "paused" | "completed" | "crashed";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "tool";
  content: string;
  turn: number;
  timestamp: string;
  /** For tool messages: the tool name (e.g. "Read", "Write", "Bash") */
  toolName?: string;
  /** Whether tool content is collapsed in the UI */
  collapsed?: boolean;
}

export interface AgentMetrics {
  tokensIn: number;
  tokensOut: number;
  costUsd: number;
  durationMs: number;
}

export interface AgentSession {
  id: string;
  workspaceId: string;
  agentType: string;
  model: string | null;
  rolePreset: string | null;
  status: AgentSessionStatus;
  pid: number | null;
  messages: ChatMessage[];
  metrics: AgentMetrics;
  startedAt: string;
  endedAt: string | null;
}

interface AgentState {
  sessions: Record<string, AgentSession>;
  activeSessionId: string | null;

  // Actions
  setSession: (session: AgentSession) => void;
  removeSession: (id: string) => void;
  setActiveSession: (id: string | null) => void;
  updateSessionStatus: (id: string, status: AgentSessionStatus) => void;
  appendMessage: (sessionId: string, message: ChatMessage) => void;
  updateMetrics: (sessionId: string, metrics: Partial<AgentMetrics>) => void;
  toggleToolCollapse: (sessionId: string, messageId: string) => void;

  // Derived
  activeSession: () => AgentSession | undefined;
  sessionForWorkspace: (workspaceId: string) => AgentSession | undefined;
}

export const useAgentStore = create<AgentState>((set, get) => ({
  sessions: {},
  activeSessionId: null,

  setSession: (session) =>
    set((state) => ({
      sessions: { ...state.sessions, [session.id]: session },
    })),

  removeSession: (id) =>
    set((state) => {
      const { [id]: _, ...rest } = state.sessions;
      return {
        sessions: rest,
        activeSessionId: state.activeSessionId === id ? null : state.activeSessionId,
      };
    }),

  setActiveSession: (id) => set({ activeSessionId: id }),

  updateSessionStatus: (id, status) =>
    set((state) => {
      const session = state.sessions[id];
      if (!session) return state;
      return {
        sessions: {
          ...state.sessions,
          [id]: { ...session, status },
        },
      };
    }),

  appendMessage: (sessionId, message) =>
    set((state) => {
      const session = state.sessions[sessionId];
      if (!session) return state;
      return {
        sessions: {
          ...state.sessions,
          [sessionId]: {
            ...session,
            messages: [...session.messages, message],
          },
        },
      };
    }),

  updateMetrics: (sessionId, metrics) =>
    set((state) => {
      const session = state.sessions[sessionId];
      if (!session) return state;
      return {
        sessions: {
          ...state.sessions,
          [sessionId]: {
            ...session,
            metrics: { ...session.metrics, ...metrics },
          },
        },
      };
    }),

  toggleToolCollapse: (sessionId, messageId) =>
    set((state) => {
      const session = state.sessions[sessionId];
      if (!session) return state;
      return {
        sessions: {
          ...state.sessions,
          [sessionId]: {
            ...session,
            messages: session.messages.map((m) =>
              m.id === messageId ? { ...m, collapsed: !m.collapsed } : m
            ),
          },
        },
      };
    }),

  activeSession: () => {
    const { sessions, activeSessionId } = get();
    return activeSessionId ? sessions[activeSessionId] : undefined;
  },

  sessionForWorkspace: (workspaceId) => {
    const { sessions } = get();
    return Object.values(sessions).find(
      (s) => s.workspaceId === workspaceId && s.status === "running"
    );
  },
}));
