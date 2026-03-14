# Plan 5A: Core UI Components — Workspace, Agent, and Command Palette

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the workspace sidebar, agent chat view, kanban board, and command palette.
**Architecture:** React components consuming Zustand stores, communicating with Rust backend via Tauri IPC.
**Tech Stack:** React, TypeScript, Tailwind CSS, Zustand, @dnd-kit/core, react-markdown
**Spec Reference:** Sections 5, 8 of the spec

---

## Chunk 1: Zustand Stores

### Task 1: Create workspaceStore.ts — Workspace UI state

**Files:**
- Create: `~/projects/koompi-orch/src/stores/workspaceStore.ts`

- [ ] **Step 1: Install Zustand**

```bash
cd ~/projects/koompi-orch && pnpm add zustand
```

- [ ] **Step 2: Create workspaceStore.ts**

Create `src/stores/workspaceStore.ts`:
```typescript
import { create } from "zustand";

/** Workspace status matching SurrealDB schema constraint */
export type WorkspaceStatus =
  | "backlog"
  | "active"
  | "review"
  | "done"
  | "failed";

export interface Workspace {
  id: string;
  name: string;
  branch: string;
  worktreePath: string;
  status: WorkspaceStatus;
  repoId: string;
  repoName: string;
  lockedBy: string | null;
  hasConflict: boolean;
  createdAt: string;
  updatedAt: string;
}

interface WorkspaceState {
  workspaces: Workspace[];
  selectedWorkspaceId: string | null;
  filterStatus: WorkspaceStatus | "all";

  // Actions
  setWorkspaces: (workspaces: Workspace[]) => void;
  addWorkspace: (workspace: Workspace) => void;
  updateWorkspace: (id: string, patch: Partial<Workspace>) => void;
  removeWorkspace: (id: string) => void;
  selectWorkspace: (id: string | null) => void;
  setFilterStatus: (status: WorkspaceStatus | "all") => void;
  setConflict: (id: string, hasConflict: boolean) => void;

  // Derived
  filteredWorkspaces: () => Workspace[];
  selectedWorkspace: () => Workspace | undefined;
  workspacesByStatus: () => Record<WorkspaceStatus, Workspace[]>;
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  workspaces: [],
  selectedWorkspaceId: null,
  filterStatus: "all",

  setWorkspaces: (workspaces) => set({ workspaces }),

  addWorkspace: (workspace) =>
    set((state) => ({ workspaces: [...state.workspaces, workspace] })),

  updateWorkspace: (id, patch) =>
    set((state) => ({
      workspaces: state.workspaces.map((w) =>
        w.id === id ? { ...w, ...patch } : w
      ),
    })),

  removeWorkspace: (id) =>
    set((state) => ({
      workspaces: state.workspaces.filter((w) => w.id !== id),
      selectedWorkspaceId:
        state.selectedWorkspaceId === id ? null : state.selectedWorkspaceId,
    })),

  selectWorkspace: (id) => set({ selectedWorkspaceId: id }),

  setFilterStatus: (status) => set({ filterStatus: status }),

  setConflict: (id, hasConflict) =>
    set((state) => ({
      workspaces: state.workspaces.map((w) =>
        w.id === id ? { ...w, hasConflict } : w
      ),
    })),

  filteredWorkspaces: () => {
    const { workspaces, filterStatus } = get();
    if (filterStatus === "all") return workspaces;
    return workspaces.filter((w) => w.status === filterStatus);
  },

  selectedWorkspace: () => {
    const { workspaces, selectedWorkspaceId } = get();
    return workspaces.find((w) => w.id === selectedWorkspaceId);
  },

  workspacesByStatus: () => {
    const { workspaces } = get();
    const grouped: Record<WorkspaceStatus, Workspace[]> = {
      backlog: [],
      active: [],
      review: [],
      done: [],
      failed: [],
    };
    for (const w of workspaces) {
      grouped[w.status].push(w);
    }
    return grouped;
  },
}));
```

- [ ] **Step 3: Verify store renders**

Create a temporary test in `src/app/App.tsx` that imports and reads from the store:
```typescript
import { useWorkspaceStore } from "../stores/workspaceStore";
// In component: const workspaces = useWorkspaceStore((s) => s.workspaces);
// Render: <div>{workspaces.length} workspaces</div>
```

Run `pnpm dev` and confirm the app renders without errors.

---

### Task 2: Create agentStore.ts — Agent session UI state

**Files:**
- Create: `~/projects/koompi-orch/src/stores/agentStore.ts`

- [ ] **Step 1: Create agentStore.ts**

Create `src/stores/agentStore.ts`:
```typescript
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
```

- [ ] **Step 2: Verify agentStore renders**

Import into App.tsx and confirm no errors:
```typescript
import { useAgentStore } from "../stores/agentStore";
// const sessions = useAgentStore((s) => s.sessions);
```

Run `pnpm dev` and confirm the app renders.

---

### Task 3: Create notificationStore.ts — Notification queue

**Files:**
- Create: `~/projects/koompi-orch/src/stores/notificationStore.ts`

- [ ] **Step 1: Create notificationStore.ts**

Create `src/stores/notificationStore.ts`:
```typescript
import { create } from "zustand";

export type NotificationType = "info" | "success" | "warning" | "error";

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  /** Workspace ID if notification is workspace-specific */
  workspaceId?: string;
  /** Auto-dismiss after this many ms (0 = manual dismiss) */
  autoCloseMs: number;
  createdAt: number;
}

interface NotificationState {
  notifications: Notification[];
  maxVisible: number;

  // Actions
  addNotification: (
    notification: Omit<Notification, "id" | "createdAt">
  ) => string;
  dismissNotification: (id: string) => void;
  clearAll: () => void;

  // Derived
  visibleNotifications: () => Notification[];
}

let notificationCounter = 0;

export const useNotificationStore = create<NotificationState>((set, get) => ({
  notifications: [],
  maxVisible: 5,

  addNotification: (notification) => {
    const id = `notif-${++notificationCounter}-${Date.now()}`;
    const full: Notification = {
      ...notification,
      id,
      createdAt: Date.now(),
    };
    set((state) => ({
      notifications: [...state.notifications, full],
    }));

    // Auto-dismiss
    if (notification.autoCloseMs > 0) {
      setTimeout(() => {
        get().dismissNotification(id);
      }, notification.autoCloseMs);
    }

    return id;
  },

  dismissNotification: (id) =>
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    })),

  clearAll: () => set({ notifications: [] }),

  visibleNotifications: () => {
    const { notifications, maxVisible } = get();
    return notifications.slice(-maxVisible);
  },
}));
```

- [ ] **Step 2: Verify notificationStore renders**

Import into App.tsx and confirm no errors. Run `pnpm dev`.

---

## Chunk 2: Workspace UI Components

### Task 4: Create WorkspaceCard.tsx — Workspace sidebar card

**Files:**
- Create: `~/projects/koompi-orch/src/components/workspace/WorkspaceCard.tsx`

- [ ] **Step 1: Create WorkspaceCard.tsx**

Create `src/components/workspace/WorkspaceCard.tsx`:
```typescript
import { type Workspace, useWorkspaceStore } from "../../stores/workspaceStore";
import { useAgentStore } from "../../stores/agentStore";

const STATUS_COLORS: Record<string, string> = {
  backlog: "bg-gray-500",
  active: "bg-blue-500",
  review: "bg-yellow-500",
  done: "bg-green-500",
  failed: "bg-red-500",
};

const STATUS_LABELS: Record<string, string> = {
  backlog: "Backlog",
  active: "Active",
  review: "Review",
  done: "Done",
  failed: "Failed",
};

interface WorkspaceCardProps {
  workspace: Workspace;
}

export function WorkspaceCard({ workspace }: WorkspaceCardProps) {
  const selectWorkspace = useWorkspaceStore((s) => s.selectWorkspace);
  const selectedId = useWorkspaceStore((s) => s.selectedWorkspaceId);
  const sessionForWorkspace = useAgentStore((s) => s.sessionForWorkspace);

  const isSelected = selectedId === workspace.id;
  const session = sessionForWorkspace(workspace.id);

  return (
    <button
      type="button"
      onClick={() => selectWorkspace(workspace.id)}
      className={`
        w-full text-left px-3 py-2 rounded-lg border transition-colors
        ${isSelected ? "border-blue-500 bg-blue-500/10" : "border-transparent hover:bg-white/5"}
      `}
    >
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-gray-200 truncate">
          {workspace.name}
        </span>
        <div className="flex items-center gap-1.5">
          {workspace.hasConflict && (
            <span
              className="w-2 h-2 rounded-full bg-orange-500"
              title="File conflict detected"
            />
          )}
          {session && (
            <span
              className="w-2 h-2 rounded-full bg-blue-400 animate-pulse"
              title={`Agent running (${session.agentType})`}
            />
          )}
          <span
            className={`px-1.5 py-0.5 text-[10px] font-semibold uppercase rounded ${STATUS_COLORS[workspace.status]} text-white`}
          >
            {STATUS_LABELS[workspace.status]}
          </span>
        </div>
      </div>
      <div className="flex items-center gap-2 mt-1">
        <span className="text-xs text-gray-500 truncate">
          {workspace.repoName}
        </span>
        <span className="text-xs text-gray-600">/</span>
        <span className="text-xs text-gray-400 truncate font-mono">
          {workspace.branch}
        </span>
      </div>
    </button>
  );
}
```

- [ ] **Step 2: Verify WorkspaceCard renders**

Import into the Sidebar or App.tsx with a mock workspace:
```typescript
import { WorkspaceCard } from "../components/workspace/WorkspaceCard";

const mockWorkspace = {
  id: "ws-1",
  name: "feat-auth",
  branch: "feat/auth-jwt",
  worktreePath: "/home/user/.koompi-orch/worktrees/my-app/feat-auth-ws3a7b",
  status: "active" as const,
  repoId: "repo-1",
  repoName: "my-app",
  lockedBy: null,
  hasConflict: false,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

// In JSX: <WorkspaceCard workspace={mockWorkspace} />
```

Run `pnpm dev` and confirm the card renders with status badge, repo name, and branch.

---

### Task 5: Create KanbanBoard.tsx — Drag-drop workspace kanban

**Files:**
- Create: `~/projects/koompi-orch/src/components/workspace/KanbanBoard.tsx`

- [ ] **Step 1: Install @dnd-kit/core and @dnd-kit/sortable**

```bash
cd ~/projects/koompi-orch && pnpm add @dnd-kit/core @dnd-kit/sortable @dnd-kit/utilities
```

- [ ] **Step 2: Create KanbanBoard.tsx**

Create `src/components/workspace/KanbanBoard.tsx`:
```typescript
import {
  DndContext,
  DragOverlay,
  closestCorners,
  PointerSensor,
  useSensor,
  useSensors,
  type DragStartEvent,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  type Workspace,
  type WorkspaceStatus,
  useWorkspaceStore,
} from "../../stores/workspaceStore";
import { WorkspaceCard } from "./WorkspaceCard";

const COLUMNS: { status: WorkspaceStatus; label: string; color: string }[] = [
  { status: "backlog", label: "Backlog", color: "border-gray-600" },
  { status: "active", label: "Active", color: "border-blue-600" },
  { status: "review", label: "Review", color: "border-yellow-600" },
  { status: "done", label: "Done", color: "border-green-600" },
  { status: "failed", label: "Failed", color: "border-red-600" },
];

/** A sortable workspace card inside a kanban column */
function SortableWorkspaceCard({ workspace }: { workspace: Workspace }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: workspace.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.4 : 1,
  };

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <WorkspaceCard workspace={workspace} />
    </div>
  );
}

/** A single kanban column for a workspace status */
function KanbanColumn({
  status,
  label,
  color,
  workspaces,
}: {
  status: WorkspaceStatus;
  label: string;
  color: string;
  workspaces: Workspace[];
}) {
  return (
    <div
      className={`flex-1 min-w-[180px] max-w-[280px] border-t-2 ${color} bg-gray-900/50 rounded-lg p-2`}
      data-column-status={status}
    >
      <div className="flex items-center justify-between mb-2 px-1">
        <h3 className="text-xs font-semibold uppercase text-gray-400">
          {label}
        </h3>
        <span className="text-xs text-gray-600">{workspaces.length}</span>
      </div>
      <SortableContext
        items={workspaces.map((w) => w.id)}
        strategy={verticalListSortingStrategy}
      >
        <div className="flex flex-col gap-1 min-h-[60px]">
          {workspaces.map((workspace) => (
            <SortableWorkspaceCard key={workspace.id} workspace={workspace} />
          ))}
        </div>
      </SortableContext>
    </div>
  );
}

export function KanbanBoard() {
  const workspacesByStatus = useWorkspaceStore((s) => s.workspacesByStatus);
  const updateWorkspace = useWorkspaceStore((s) => s.updateWorkspace);
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const [activeId, setActiveId] = useState<string | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 5 },
    })
  );

  const grouped = workspacesByStatus();
  const activeWorkspace = activeId
    ? workspaces.find((w) => w.id === activeId)
    : null;

  const handleDragStart = useCallback((event: DragStartEvent) => {
    setActiveId(String(event.active.id));
  }, []);

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      setActiveId(null);
      const { active, over } = event;
      if (!over) return;

      // Determine target column: check if dropped over a column container
      // or over another card (use that card's status)
      const draggedId = String(active.id);
      const overId = String(over.id);

      // Find the workspace that was dragged
      const draggedWorkspace = workspaces.find((w) => w.id === draggedId);
      if (!draggedWorkspace) return;

      // Find target status: if dropped on a workspace, use its status
      const overWorkspace = workspaces.find((w) => w.id === overId);
      let targetStatus: WorkspaceStatus | undefined;

      if (overWorkspace) {
        targetStatus = overWorkspace.status;
      } else {
        // Check if over element has column data attribute
        const overElement = document.querySelector(
          `[data-column-status="${overId}"]`
        );
        if (overElement) {
          targetStatus = overId as WorkspaceStatus;
        }
      }

      if (targetStatus && targetStatus !== draggedWorkspace.status) {
        // Update local state immediately
        updateWorkspace(draggedId, { status: targetStatus });
        // Persist to backend
        invoke("update_workspace_status", {
          workspaceId: draggedId,
          status: targetStatus,
        }).catch((err: unknown) => {
          console.error("Failed to update workspace status:", err);
          // Revert on failure
          updateWorkspace(draggedId, { status: draggedWorkspace.status });
        });
      }
    },
    [workspaces, updateWorkspace]
  );

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
    >
      <div className="flex gap-3 overflow-x-auto p-2">
        {COLUMNS.map(({ status, label, color }) => (
          <KanbanColumn
            key={status}
            status={status}
            label={label}
            color={color}
            workspaces={grouped[status]}
          />
        ))}
      </div>
      <DragOverlay>
        {activeWorkspace ? (
          <div className="opacity-80">
            <WorkspaceCard workspace={activeWorkspace} />
          </div>
        ) : null}
      </DragOverlay>
    </DndContext>
  );
}
```

- [ ] **Step 3: Verify KanbanBoard renders**

Import `KanbanBoard` into the sidebar area. Seed the workspace store with a few mock workspaces across statuses:
```typescript
useWorkspaceStore.getState().setWorkspaces([
  { id: "1", name: "feat-auth", branch: "feat/auth", worktreePath: "/tmp/w1", status: "active", repoId: "r1", repoName: "my-app", lockedBy: null, hasConflict: false, createdAt: "", updatedAt: "" },
  { id: "2", name: "fix-bug", branch: "fix/bug-123", worktreePath: "/tmp/w2", status: "review", repoId: "r1", repoName: "my-app", lockedBy: null, hasConflict: true, createdAt: "", updatedAt: "" },
  { id: "3", name: "refactor-db", branch: "refactor/db", worktreePath: "/tmp/w3", status: "backlog", repoId: "r2", repoName: "api", lockedBy: null, hasConflict: false, createdAt: "", updatedAt: "" },
]);
```

Run `pnpm dev`. Confirm five columns render with cards in the correct columns. Drag a card between columns and confirm it moves.

---

### Task 6: Create WorkspaceCreate.tsx — New workspace dialog

**Files:**
- Create: `~/projects/koompi-orch/src/components/workspace/WorkspaceCreate.tsx`

- [ ] **Step 1: Create WorkspaceCreate.tsx**

Create `src/components/workspace/WorkspaceCreate.tsx`:
```typescript
import { useState, useCallback, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import { useNotificationStore } from "../../stores/notificationStore";

interface WorkspaceCreateProps {
  open: boolean;
  onClose: () => void;
}

interface RepoOption {
  id: string;
  name: string;
  path: string;
}

const AGENT_TEMPLATES = [
  { value: "claude-code", label: "Claude Code" },
  { value: "codex", label: "Codex" },
  { value: "gemini-cli", label: "Gemini CLI" },
  { value: "aider", label: "Aider" },
  { value: "custom", label: "Custom" },
];

const ROLE_PRESETS = [
  { value: "architect", label: "Architect" },
  { value: "implementer", label: "Implementer" },
  { value: "reviewer", label: "Reviewer" },
  { value: "tester", label: "Tester" },
  { value: "shipper", label: "Shipper" },
  { value: "fixer", label: "Fixer" },
];

export function WorkspaceCreate({ open, onClose }: WorkspaceCreateProps) {
  const addWorkspace = useWorkspaceStore((s) => s.addWorkspace);
  const addNotification = useNotificationStore((s) => s.addNotification);

  const [repos, setRepos] = useState<RepoOption[]>([]);
  const [selectedRepoId, setSelectedRepoId] = useState("");
  const [workspaceName, setWorkspaceName] = useState("");
  const [branchName, setBranchName] = useState("");
  const [agentTemplate, setAgentTemplate] = useState("claude-code");
  const [rolePreset, setRolePreset] = useState("implementer");
  const [initialPrompt, setInitialPrompt] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [reposLoaded, setReposLoaded] = useState(false);

  // Load repos when dialog opens
  const loadRepos = useCallback(async () => {
    if (reposLoaded) return;
    try {
      const result = await invoke<RepoOption[]>("list_repos");
      setRepos(result);
      setReposLoaded(true);
    } catch (err) {
      console.error("Failed to load repos:", err);
    }
  }, [reposLoaded]);

  if (open && !reposLoaded) {
    loadRepos();
  }

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!selectedRepoId || !workspaceName || !branchName) return;

    setIsSubmitting(true);
    try {
      const workspace = await invoke<{
        id: string;
        worktree_path: string;
        repo_name: string;
      }>("create_workspace", {
        repoId: selectedRepoId,
        name: workspaceName,
        branch: branchName,
        agentTemplate,
        rolePreset,
        initialPrompt: initialPrompt || null,
      });

      addWorkspace({
        id: workspace.id,
        name: workspaceName,
        branch: branchName,
        worktreePath: workspace.worktree_path,
        status: initialPrompt ? "active" : "backlog",
        repoId: selectedRepoId,
        repoName: workspace.repo_name,
        lockedBy: null,
        hasConflict: false,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      });

      addNotification({
        type: "success",
        title: "Workspace created",
        message: `${workspaceName} on ${branchName}`,
        autoCloseMs: 3000,
      });

      // Reset form
      setWorkspaceName("");
      setBranchName("");
      setInitialPrompt("");
      onClose();
    } catch (err) {
      addNotification({
        type: "error",
        title: "Failed to create workspace",
        message: String(err),
        autoCloseMs: 5000,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-gray-800 rounded-xl border border-gray-700 shadow-xl w-full max-w-md p-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-100">
            New Workspace
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="text-gray-500 hover:text-gray-300 text-xl leading-none"
          >
            &times;
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          {/* Repository */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Repository
            </label>
            <select
              value={selectedRepoId}
              onChange={(e) => setSelectedRepoId(e.target.value)}
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              required
            >
              <option value="">Select a repository...</option>
              {repos.map((repo) => (
                <option key={repo.id} value={repo.id}>
                  {repo.name} — {repo.path}
                </option>
              ))}
            </select>
          </div>

          {/* Workspace Name */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Workspace Name
            </label>
            <input
              type="text"
              value={workspaceName}
              onChange={(e) => setWorkspaceName(e.target.value)}
              placeholder="feat-auth"
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              required
            />
          </div>

          {/* Branch Name */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Branch Name
            </label>
            <input
              type="text"
              value={branchName}
              onChange={(e) => setBranchName(e.target.value)}
              placeholder="feat/auth-jwt"
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              required
            />
          </div>

          {/* Agent Template */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Agent
            </label>
            <select
              value={agentTemplate}
              onChange={(e) => setAgentTemplate(e.target.value)}
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
            >
              {AGENT_TEMPLATES.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </select>
          </div>

          {/* Role Preset */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Role
            </label>
            <select
              value={rolePreset}
              onChange={(e) => setRolePreset(e.target.value)}
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
            >
              {ROLE_PRESETS.map((r) => (
                <option key={r.value} value={r.value}>
                  {r.label}
                </option>
              ))}
            </select>
          </div>

          {/* Initial Prompt (optional) */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Initial Prompt{" "}
              <span className="text-gray-600">(optional — starts agent immediately)</span>
            </label>
            <textarea
              value={initialPrompt}
              onChange={(e) => setInitialPrompt(e.target.value)}
              placeholder="Implement JWT auth module with refresh tokens..."
              rows={3}
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500 resize-none"
            />
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-2 mt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm text-gray-400 hover:text-gray-200 rounded-md"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !selectedRepoId || !workspaceName || !branchName}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-md"
            >
              {isSubmitting ? "Creating..." : "Create Workspace"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify WorkspaceCreate renders**

Add a button to App.tsx that toggles the dialog open:
```typescript
const [showCreate, setShowCreate] = useState(false);
// <button onClick={() => setShowCreate(true)}>New Workspace</button>
// <WorkspaceCreate open={showCreate} onClose={() => setShowCreate(false)} />
```

Run `pnpm dev`. Click the button and confirm the dialog opens with all form fields.

---

### Task 7: Create MultiRepoSelector.tsx — Repository picker

**Files:**
- Create: `~/projects/koompi-orch/src/components/workspace/MultiRepoSelector.tsx`

- [ ] **Step 1: Create MultiRepoSelector.tsx**

Create `src/components/workspace/MultiRepoSelector.tsx`:
```typescript
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Repo {
  id: string;
  name: string;
  path: string;
  remoteUrl: string | null;
}

interface MultiRepoSelectorProps {
  /** Currently selected repo IDs */
  selectedIds: string[];
  /** Callback when selection changes */
  onChange: (selectedIds: string[]) => void;
  /** Allow multiple selection (default: false) */
  multiple?: boolean;
}

export function MultiRepoSelector({
  selectedIds,
  onChange,
  multiple = false,
}: MultiRepoSelectorProps) {
  const [repos, setRepos] = useState<Repo[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [addingPath, setAddingPath] = useState("");

  useEffect(() => {
    invoke<Repo[]>("list_repos")
      .then(setRepos)
      .catch((err: unknown) => console.error("Failed to load repos:", err))
      .finally(() => setLoading(false));
  }, []);

  const filtered = repos.filter(
    (r) =>
      r.name.toLowerCase().includes(search.toLowerCase()) ||
      r.path.toLowerCase().includes(search.toLowerCase())
  );

  const toggleRepo = (id: string) => {
    if (multiple) {
      if (selectedIds.includes(id)) {
        onChange(selectedIds.filter((s) => s !== id));
      } else {
        onChange([...selectedIds, id]);
      }
    } else {
      onChange(selectedIds.includes(id) ? [] : [id]);
    }
  };

  const addRepo = async () => {
    if (!addingPath.trim()) return;
    try {
      const repo = await invoke<Repo>("add_repo", { path: addingPath.trim() });
      setRepos((prev) => [...prev, repo]);
      setAddingPath("");
      onChange([...selectedIds, repo.id]);
    } catch (err) {
      console.error("Failed to add repo:", err);
    }
  };

  if (loading) {
    return (
      <div className="text-sm text-gray-500 py-4 text-center">
        Loading repositories...
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {/* Search */}
      <input
        type="text"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Search repositories..."
        className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-1.5 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
      />

      {/* Repo list */}
      <div className="max-h-48 overflow-y-auto border border-gray-700 rounded-md">
        {filtered.length === 0 ? (
          <div className="text-sm text-gray-500 py-3 text-center">
            No repositories found
          </div>
        ) : (
          filtered.map((repo) => (
            <button
              key={repo.id}
              type="button"
              onClick={() => toggleRepo(repo.id)}
              className={`
                w-full text-left px-3 py-2 text-sm flex items-center gap-2
                border-b border-gray-800 last:border-b-0 transition-colors
                ${selectedIds.includes(repo.id) ? "bg-blue-500/15 text-blue-300" : "text-gray-300 hover:bg-white/5"}
              `}
            >
              <span
                className={`w-4 h-4 rounded border flex items-center justify-center text-[10px] ${
                  selectedIds.includes(repo.id)
                    ? "border-blue-500 bg-blue-500 text-white"
                    : "border-gray-600"
                }`}
              >
                {selectedIds.includes(repo.id) && "\u2713"}
              </span>
              <div className="flex-1 min-w-0">
                <div className="font-medium truncate">{repo.name}</div>
                <div className="text-xs text-gray-500 truncate">{repo.path}</div>
              </div>
            </button>
          ))
        )}
      </div>

      {/* Add new repo */}
      <div className="flex gap-2">
        <input
          type="text"
          value={addingPath}
          onChange={(e) => setAddingPath(e.target.value)}
          placeholder="/path/to/repo"
          className="flex-1 bg-gray-900 border border-gray-700 rounded-md px-3 py-1.5 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              addRepo();
            }
          }}
        />
        <button
          type="button"
          onClick={addRepo}
          disabled={!addingPath.trim()}
          className="px-3 py-1.5 text-sm bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-200 rounded-md"
        >
          Add
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify MultiRepoSelector renders**

Render `<MultiRepoSelector selectedIds={[]} onChange={console.log} />` in a test page. Run `pnpm dev` and confirm it renders the search input, repo list (empty until backend is connected), and add repo input.

---

### Task 8: Create FileTree.tsx — Workspace file explorer

**Files:**
- Create: `~/projects/koompi-orch/src/components/workspace/FileTree.tsx`

- [ ] **Step 1: Create FileTree.tsx**

Create `src/components/workspace/FileTree.tsx`:
```typescript
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Status from git: modified, added, deleted, renamed, untracked */
type GitFileStatus = "M" | "A" | "D" | "R" | "?" | null;

interface FileNode {
  name: string;
  path: string;
  isDir: boolean;
  children?: FileNode[];
  gitStatus: GitFileStatus;
}

const GIT_STATUS_COLORS: Record<string, string> = {
  M: "text-yellow-400",
  A: "text-green-400",
  D: "text-red-400",
  R: "text-blue-400",
  "?": "text-gray-500",
};

interface FileTreeProps {
  workspaceId: string;
  worktreePath: string;
  onFileSelect?: (filePath: string) => void;
}

function FileTreeNode({
  node,
  depth,
  onFileSelect,
}: {
  node: FileNode;
  depth: number;
  onFileSelect?: (filePath: string) => void;
}) {
  const [expanded, setExpanded] = useState(depth < 1);

  const handleClick = () => {
    if (node.isDir) {
      setExpanded(!expanded);
    } else {
      onFileSelect?.(node.path);
    }
  };

  return (
    <div>
      <button
        type="button"
        onClick={handleClick}
        className="w-full text-left flex items-center gap-1 px-1 py-0.5 text-sm hover:bg-white/5 rounded"
        style={{ paddingLeft: `${depth * 16 + 4}px` }}
      >
        {/* Expand/collapse icon for directories */}
        {node.isDir ? (
          <span className="w-4 text-gray-500 text-xs">
            {expanded ? "\u25BE" : "\u25B8"}
          </span>
        ) : (
          <span className="w-4" />
        )}

        {/* File/folder icon */}
        <span className="text-xs">
          {node.isDir ? (expanded ? "\uD83D\uDCC2" : "\uD83D\uDCC1") : "\uD83D\uDCC4"}
        </span>

        {/* Name */}
        <span
          className={`truncate ${
            node.gitStatus
              ? GIT_STATUS_COLORS[node.gitStatus] ?? "text-gray-300"
              : "text-gray-300"
          }`}
        >
          {node.name}
        </span>

        {/* Git status badge */}
        {node.gitStatus && (
          <span
            className={`ml-auto text-[10px] font-mono ${
              GIT_STATUS_COLORS[node.gitStatus] ?? "text-gray-500"
            }`}
          >
            {node.gitStatus}
          </span>
        )}
      </button>

      {node.isDir && expanded && node.children && (
        <div>
          {node.children.map((child) => (
            <FileTreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              onFileSelect={onFileSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function FileTree({
  workspaceId,
  worktreePath,
  onFileSelect,
}: FileTreeProps) {
  const [tree, setTree] = useState<FileNode[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadTree = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<FileNode[]>("list_workspace_files", {
        workspaceId,
        worktreePath,
      });
      setTree(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [workspaceId, worktreePath]);

  useEffect(() => {
    loadTree();
  }, [loadTree]);

  if (loading) {
    return (
      <div className="text-sm text-gray-500 p-2">Loading files...</div>
    );
  }

  if (error) {
    return (
      <div className="text-sm text-red-400 p-2">
        Error: {error}
        <button
          type="button"
          onClick={loadTree}
          className="ml-2 text-blue-400 hover:underline"
        >
          Retry
        </button>
      </div>
    );
  }

  if (tree.length === 0) {
    return (
      <div className="text-sm text-gray-500 p-2">No files in workspace</div>
    );
  }

  return (
    <div className="text-sm">
      <div className="flex items-center justify-between px-2 py-1 mb-1">
        <span className="text-xs font-semibold uppercase text-gray-400">
          Files
        </span>
        <button
          type="button"
          onClick={loadTree}
          className="text-xs text-gray-500 hover:text-gray-300"
          title="Refresh"
        >
          &#x21BB;
        </button>
      </div>
      {tree.map((node) => (
        <FileTreeNode
          key={node.path}
          node={node}
          depth={0}
          onFileSelect={onFileSelect}
        />
      ))}
    </div>
  );
}
```

- [ ] **Step 2: Verify FileTree renders**

Render `<FileTree workspaceId="ws-1" worktreePath="/tmp/test" />` in a test page. It will show "Loading files..." then an error (since backend is not connected yet). Confirm the component mounts without crashes. Run `pnpm dev`.

---

## Chunk 3: Agent UI Components

### Task 9: Create ChatView.tsx — Agent conversation view

**Files:**
- Create: `~/projects/koompi-orch/src/components/agent/ChatView.tsx`

- [ ] **Step 1: Install react-markdown and dependencies**

```bash
cd ~/projects/koompi-orch && pnpm add react-markdown remark-gfm
```

- [ ] **Step 2: Create ChatView.tsx**

Create `src/components/agent/ChatView.tsx`:
```typescript
import { useEffect, useRef } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useAgentStore, type ChatMessage } from "../../stores/agentStore";

/** Render a single tool-use block (collapsible) */
function ToolUseBlock({
  message,
  sessionId,
}: {
  message: ChatMessage;
  sessionId: string;
}) {
  const toggleCollapse = useAgentStore((s) => s.toggleToolCollapse);

  return (
    <div className="border border-gray-700 rounded-md overflow-hidden my-1">
      <button
        type="button"
        onClick={() => toggleCollapse(sessionId, message.id)}
        className="w-full flex items-center gap-2 px-3 py-1.5 bg-gray-800 hover:bg-gray-750 text-left text-sm"
      >
        <span className="text-gray-500 text-xs">
          {message.collapsed ? "\u25B8" : "\u25BE"}
        </span>
        <span className="text-blue-400 font-mono text-xs">
          {message.toolName ?? "Tool"}
        </span>
        {message.collapsed && (
          <span className="text-gray-500 text-xs truncate flex-1">
            {message.content.slice(0, 80)}
            {message.content.length > 80 ? "..." : ""}
          </span>
        )}
      </button>
      {!message.collapsed && (
        <div className="px-3 py-2 bg-gray-900/50 text-xs font-mono text-gray-300 whitespace-pre-wrap overflow-x-auto max-h-64 overflow-y-auto">
          {message.content}
        </div>
      )}
    </div>
  );
}

/** Render a single chat message bubble */
function MessageBubble({
  message,
  sessionId,
}: {
  message: ChatMessage;
  sessionId: string;
}) {
  if (message.role === "tool") {
    return <ToolUseBlock message={message} sessionId={sessionId} />;
  }

  const isUser = message.role === "user";

  return (
    <div
      className={`flex ${isUser ? "justify-end" : "justify-start"} mb-3`}
    >
      <div
        className={`
          max-w-[85%] rounded-lg px-4 py-2.5 text-sm
          ${isUser ? "bg-blue-600 text-white" : "bg-gray-800 text-gray-200"}
        `}
      >
        {isUser ? (
          <p className="whitespace-pre-wrap">{message.content}</p>
        ) : (
          <div className="prose prose-sm prose-invert max-w-none [&_pre]:bg-gray-900 [&_pre]:rounded-md [&_pre]:p-3 [&_code]:text-blue-300 [&_a]:text-blue-400">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>
              {message.content}
            </ReactMarkdown>
          </div>
        )}
        <div className="text-[10px] mt-1 opacity-50 text-right">
          {new Date(message.timestamp).toLocaleTimeString()}
        </div>
      </div>
    </div>
  );
}

interface ChatViewProps {
  sessionId: string;
}

export function ChatView({ sessionId }: ChatViewProps) {
  const session = useAgentStore((s) => s.sessions[sessionId]);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [session?.messages.length]);

  if (!session) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500 text-sm">
        No active session. Create a workspace and start an agent.
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-3 px-4 py-2 border-b border-gray-700">
        <span className="text-sm font-medium text-gray-200">
          {session.agentType}
        </span>
        {session.model && (
          <span className="text-xs text-gray-500 font-mono">
            {session.model}
          </span>
        )}
        {session.rolePreset && (
          <span className="text-xs px-1.5 py-0.5 bg-purple-500/20 text-purple-300 rounded">
            {session.rolePreset}
          </span>
        )}
        <div className="flex-1" />
        <AgentStatusDot status={session.status} />
      </div>

      {/* Messages */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto px-4 py-3 space-y-1"
      >
        {session.messages.length === 0 ? (
          <div className="text-center text-gray-500 text-sm mt-8">
            Waiting for agent output...
          </div>
        ) : (
          session.messages.map((msg) => (
            <MessageBubble
              key={msg.id}
              message={msg}
              sessionId={sessionId}
            />
          ))
        )}
      </div>
    </div>
  );
}

/** Small colored dot indicating agent status */
function AgentStatusDot({
  status,
}: {
  status: string;
}) {
  const colors: Record<string, string> = {
    running: "bg-green-500 animate-pulse",
    paused: "bg-yellow-500",
    completed: "bg-gray-500",
    crashed: "bg-red-500",
  };

  const labels: Record<string, string> = {
    running: "Running",
    paused: "Paused",
    completed: "Completed",
    crashed: "Crashed",
  };

  return (
    <div className="flex items-center gap-1.5">
      <span className={`w-2 h-2 rounded-full ${colors[status] ?? "bg-gray-500"}`} />
      <span className="text-xs text-gray-400">
        {labels[status] ?? status}
      </span>
    </div>
  );
}
```

- [ ] **Step 3: Verify ChatView renders**

Seed the agent store with a mock session and render the ChatView:
```typescript
import { useAgentStore } from "../stores/agentStore";

useAgentStore.getState().setSession({
  id: "sess-1",
  workspaceId: "ws-1",
  agentType: "claude-code",
  model: "opus-4.6",
  rolePreset: "implementer",
  status: "running",
  pid: 12345,
  messages: [
    { id: "m1", role: "user", content: "Implement JWT auth module", turn: 1, timestamp: new Date().toISOString() },
    { id: "m2", role: "assistant", content: "I'll create the auth module. Let me start by analyzing the codebase...", turn: 1, timestamp: new Date().toISOString() },
    { id: "m3", role: "tool", content: "src/lib.rs\nsrc/main.rs\nsrc/auth/mod.rs", turn: 1, timestamp: new Date().toISOString(), toolName: "Read" },
    { id: "m4", role: "assistant", content: "Here's the implementation:\n\n```rust\npub fn authenticate(token: &str) -> Result<Claims, AuthError> {\n    // verify JWT\n}\n```", turn: 2, timestamp: new Date().toISOString() },
  ],
  metrics: { tokensIn: 1200, tokensOut: 3400, costUsd: 0.05, durationMs: 45000 },
  startedAt: new Date().toISOString(),
  endedAt: null,
});

// In JSX: <ChatView sessionId="sess-1" />
```

Run `pnpm dev`. Confirm: user messages on the right (blue), assistant messages on the left (dark) with markdown rendering, tool blocks collapsible, status dot shows green pulsing "Running".

---

### Task 10: Create ChatInput.tsx — Message input with agent/model selector

**Files:**
- Create: `~/projects/koompi-orch/src/components/agent/ChatInput.tsx`

- [ ] **Step 1: Create ChatInput.tsx**

Create `src/components/agent/ChatInput.tsx`:
```typescript
import { useState, useRef, useCallback, type KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAgentStore, type ChatMessage } from "../../stores/agentStore";

const AGENT_OPTIONS = [
  { value: "claude-code", label: "Claude Code" },
  { value: "codex", label: "Codex" },
  { value: "gemini-cli", label: "Gemini CLI" },
  { value: "aider", label: "Aider" },
];

const MODEL_OPTIONS: Record<string, string[]> = {
  "claude-code": ["opus-4.6", "sonnet-4.5", "haiku-3.5"],
  codex: ["o3", "o4-mini"],
  "gemini-cli": ["gemini-2.5-pro", "gemini-2.5-flash"],
  aider: ["opus-4.6", "sonnet-4.5"],
};

interface ChatInputProps {
  sessionId: string;
  workspaceId: string;
  disabled?: boolean;
}

export function ChatInput({
  sessionId,
  workspaceId,
  disabled = false,
}: ChatInputProps) {
  const appendMessage = useAgentStore((s) => s.appendMessage);
  const session = useAgentStore((s) => s.sessions[sessionId]);

  const [message, setMessage] = useState("");
  const [selectedAgent, setSelectedAgent] = useState(
    session?.agentType ?? "claude-code"
  );
  const [selectedModel, setSelectedModel] = useState(
    session?.model ?? MODEL_OPTIONS["claude-code"][0]
  );
  const [sending, setSending] = useState(false);
  const [showDropdown, setShowDropdown] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const availableModels = MODEL_OPTIONS[selectedAgent] ?? [];

  const handleAgentChange = (agent: string) => {
    setSelectedAgent(agent);
    const models = MODEL_OPTIONS[agent] ?? [];
    setSelectedModel(models[0] ?? "");
    setShowDropdown(false);
  };

  const sendMessage = useCallback(async () => {
    const trimmed = message.trim();
    if (!trimmed || sending || disabled) return;

    setSending(true);

    // Optimistically add user message to chat
    const userMsg: ChatMessage = {
      id: `msg-${Date.now()}`,
      role: "user",
      content: trimmed,
      turn: (session?.messages.length ?? 0) + 1,
      timestamp: new Date().toISOString(),
    };
    appendMessage(sessionId, userMsg);
    setMessage("");

    try {
      await invoke("send_message_to_agent", {
        sessionId,
        workspaceId,
        message: trimmed,
        agentType: selectedAgent,
        model: selectedModel,
      });
    } catch (err) {
      console.error("Failed to send message:", err);
      // Error notification could be added here
    } finally {
      setSending(false);
      textareaRef.current?.focus();
    }
  }, [message, sending, disabled, sessionId, workspaceId, selectedAgent, selectedModel, session, appendMessage]);

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  // Auto-resize textarea
  const handleInput = (value: string) => {
    setMessage(value);
    const el = textareaRef.current;
    if (el) {
      el.style.height = "auto";
      el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
    }
  };

  return (
    <div className="border-t border-gray-700 p-3">
      <div className="flex items-end gap-2">
        {/* Agent/Model selector */}
        <div className="relative">
          <button
            type="button"
            onClick={() => setShowDropdown(!showDropdown)}
            className="flex items-center gap-1 px-2 py-1.5 text-xs bg-gray-800 border border-gray-700 rounded-md text-gray-300 hover:bg-gray-750 whitespace-nowrap"
          >
            <span>{AGENT_OPTIONS.find((a) => a.value === selectedAgent)?.label}</span>
            <span className="text-gray-500">/</span>
            <span className="text-blue-400">{selectedModel}</span>
            <span className="text-gray-500 ml-1">{"\u25BE"}</span>
          </button>

          {showDropdown && (
            <div className="absolute bottom-full mb-1 left-0 bg-gray-800 border border-gray-700 rounded-md shadow-lg z-10 min-w-[200px]">
              {AGENT_OPTIONS.map((agent) => (
                <div key={agent.value}>
                  <button
                    type="button"
                    onClick={() => handleAgentChange(agent.value)}
                    className={`w-full text-left px-3 py-1.5 text-sm hover:bg-gray-700 ${
                      selectedAgent === agent.value
                        ? "text-blue-400"
                        : "text-gray-300"
                    }`}
                  >
                    {agent.label}
                  </button>
                  {selectedAgent === agent.value && (
                    <div className="pl-4">
                      {(MODEL_OPTIONS[agent.value] ?? []).map((model) => (
                        <button
                          key={model}
                          type="button"
                          onClick={() => {
                            setSelectedModel(model);
                            setShowDropdown(false);
                          }}
                          className={`w-full text-left px-3 py-1 text-xs hover:bg-gray-700 ${
                            selectedModel === model
                              ? "text-blue-300"
                              : "text-gray-500"
                          }`}
                        >
                          {model}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Textarea */}
        <textarea
          ref={textareaRef}
          value={message}
          onChange={(e) => handleInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a message... (Enter to send, Shift+Enter for newline)"
          disabled={disabled || sending}
          rows={1}
          className="flex-1 bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 resize-none focus:outline-none focus:border-blue-500 disabled:opacity-50 min-h-[38px] max-h-[200px]"
        />

        {/* Send button */}
        <button
          type="button"
          onClick={sendMessage}
          disabled={!message.trim() || sending || disabled}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-md"
        >
          {sending ? "..." : "Send"}
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify ChatInput renders**

Render `<ChatInput sessionId="sess-1" workspaceId="ws-1" />` below the ChatView. Run `pnpm dev`. Confirm: textarea renders, agent/model dropdown opens and shows nested model list, typing and pressing Enter calls the send flow (will error without backend, but the UI should work).

---

### Task 11: Create AgentPicker.tsx — Agent type and role preset picker

**Files:**
- Create: `~/projects/koompi-orch/src/components/agent/AgentPicker.tsx`

- [ ] **Step 1: Create AgentPicker.tsx**

Create `src/components/agent/AgentPicker.tsx`:
```typescript
import { useState } from "react";

export interface AgentPickerValue {
  agentTemplate: string;
  rolePreset: string;
}

const AGENT_TEMPLATES = [
  {
    value: "claude-code",
    label: "Claude Code",
    description: "Anthropic's coding agent. JSON streaming, resume support.",
    icon: "\uD83E\uDDE0",
  },
  {
    value: "codex",
    label: "Codex",
    description: "OpenAI's coding CLI. PTY-based input/output.",
    icon: "\uD83D\uDCBB",
  },
  {
    value: "gemini-cli",
    label: "Gemini CLI",
    description: "Google's Gemini coding agent. PTY-based.",
    icon: "\u2728",
  },
  {
    value: "aider",
    label: "Aider",
    description: "Open-source pair programmer. Chat history restore.",
    icon: "\uD83D\uDEE0\uFE0F",
  },
  {
    value: "custom",
    label: "Custom",
    description: "Your own CLI agent. Configure command and args.",
    icon: "\u2699\uFE0F",
  },
];

const ROLE_PRESETS = [
  {
    value: "architect",
    label: "Architect",
    description: "Think from first principles, design before coding.",
  },
  {
    value: "implementer",
    label: "Implementer",
    description: "Write production code, follow existing patterns.",
  },
  {
    value: "reviewer",
    label: "Reviewer",
    description: "Paranoid code review: security, race conditions, trust.",
  },
  {
    value: "tester",
    label: "Tester",
    description: "Comprehensive tests, edge cases, integration tests.",
  },
  {
    value: "shipper",
    label: "Shipper",
    description: "Final-mile: sync main, run tests, open PR.",
  },
  {
    value: "fixer",
    label: "Fixer",
    description: "Debug and fix: systematic root cause, minimal changes.",
  },
];

interface AgentPickerProps {
  value: AgentPickerValue;
  onChange: (value: AgentPickerValue) => void;
}

export function AgentPicker({ value, onChange }: AgentPickerProps) {
  const [tab, setTab] = useState<"agent" | "role">("agent");

  return (
    <div className="flex flex-col gap-3">
      {/* Tab switcher */}
      <div className="flex border-b border-gray-700">
        <button
          type="button"
          onClick={() => setTab("agent")}
          className={`px-3 py-1.5 text-sm ${
            tab === "agent"
              ? "text-blue-400 border-b-2 border-blue-400"
              : "text-gray-500 hover:text-gray-300"
          }`}
        >
          Agent
        </button>
        <button
          type="button"
          onClick={() => setTab("role")}
          className={`px-3 py-1.5 text-sm ${
            tab === "role"
              ? "text-blue-400 border-b-2 border-blue-400"
              : "text-gray-500 hover:text-gray-300"
          }`}
        >
          Role
        </button>
      </div>

      {/* Agent selection */}
      {tab === "agent" && (
        <div className="flex flex-col gap-1">
          {AGENT_TEMPLATES.map((agent) => (
            <button
              key={agent.value}
              type="button"
              onClick={() =>
                onChange({ ...value, agentTemplate: agent.value })
              }
              className={`
                w-full text-left px-3 py-2 rounded-md flex items-start gap-3 transition-colors
                ${
                  value.agentTemplate === agent.value
                    ? "bg-blue-500/15 border border-blue-500/50"
                    : "hover:bg-white/5 border border-transparent"
                }
              `}
            >
              <span className="text-lg mt-0.5">{agent.icon}</span>
              <div>
                <div className="text-sm font-medium text-gray-200">
                  {agent.label}
                </div>
                <div className="text-xs text-gray-500">{agent.description}</div>
              </div>
            </button>
          ))}
        </div>
      )}

      {/* Role selection */}
      {tab === "role" && (
        <div className="flex flex-col gap-1">
          {ROLE_PRESETS.map((role) => (
            <button
              key={role.value}
              type="button"
              onClick={() =>
                onChange({ ...value, rolePreset: role.value })
              }
              className={`
                w-full text-left px-3 py-2 rounded-md transition-colors
                ${
                  value.rolePreset === role.value
                    ? "bg-purple-500/15 border border-purple-500/50"
                    : "hover:bg-white/5 border border-transparent"
                }
              `}
            >
              <div className="text-sm font-medium text-gray-200">
                {role.label}
              </div>
              <div className="text-xs text-gray-500">{role.description}</div>
            </button>
          ))}
        </div>
      )}

      {/* Current selection summary */}
      <div className="text-xs text-gray-500 px-1">
        Selected:{" "}
        <span className="text-gray-300">
          {AGENT_TEMPLATES.find((a) => a.value === value.agentTemplate)?.label}
        </span>
        {" + "}
        <span className="text-purple-300">
          {ROLE_PRESETS.find((r) => r.value === value.rolePreset)?.label}
        </span>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify AgentPicker renders**

```typescript
const [pickerValue, setPickerValue] = useState({ agentTemplate: "claude-code", rolePreset: "implementer" });
// <AgentPicker value={pickerValue} onChange={setPickerValue} />
```

Run `pnpm dev`. Confirm two tabs (Agent/Role), items highlight on click, summary line updates.

---

### Task 12: Create AgentStatus.tsx — Running/paused/done indicator

**Files:**
- Create: `~/projects/koompi-orch/src/components/agent/AgentStatus.tsx`

- [ ] **Step 1: Create AgentStatus.tsx**

Create `src/components/agent/AgentStatus.tsx`:
```typescript
import { invoke } from "@tauri-apps/api/core";
import { useAgentStore, type AgentSessionStatus } from "../../stores/agentStore";

const STATUS_CONFIG: Record<
  AgentSessionStatus,
  { color: string; bgColor: string; label: string; animate: boolean }
> = {
  running: {
    color: "text-green-400",
    bgColor: "bg-green-500/15",
    label: "Running",
    animate: true,
  },
  paused: {
    color: "text-yellow-400",
    bgColor: "bg-yellow-500/15",
    label: "Paused",
    animate: false,
  },
  completed: {
    color: "text-gray-400",
    bgColor: "bg-gray-500/15",
    label: "Completed",
    animate: false,
  },
  crashed: {
    color: "text-red-400",
    bgColor: "bg-red-500/15",
    label: "Crashed",
    animate: false,
  },
};

interface AgentStatusProps {
  sessionId: string;
  /** Show action buttons (pause/resume/kill) */
  showActions?: boolean;
}

export function AgentStatus({
  sessionId,
  showActions = true,
}: AgentStatusProps) {
  const session = useAgentStore((s) => s.sessions[sessionId]);
  const updateStatus = useAgentStore((s) => s.updateSessionStatus);

  if (!session) {
    return (
      <div className="text-xs text-gray-600">No session</div>
    );
  }

  const config = STATUS_CONFIG[session.status];

  const handlePause = async () => {
    try {
      await invoke("pause_agent", { sessionId });
      updateStatus(sessionId, "paused");
    } catch (err) {
      console.error("Failed to pause agent:", err);
    }
  };

  const handleResume = async () => {
    try {
      await invoke("resume_agent", { sessionId });
      updateStatus(sessionId, "running");
    } catch (err) {
      console.error("Failed to resume agent:", err);
    }
  };

  const handleKill = async () => {
    try {
      await invoke("kill_agent", { sessionId });
      updateStatus(sessionId, "crashed");
    } catch (err) {
      console.error("Failed to kill agent:", err);
    }
  };

  const elapsed = session.startedAt
    ? formatDuration(Date.now() - new Date(session.startedAt).getTime())
    : "--";

  return (
    <div className={`flex items-center gap-3 px-3 py-2 rounded-md ${config.bgColor}`}>
      {/* Status indicator */}
      <div className="flex items-center gap-2">
        <span
          className={`w-2 h-2 rounded-full ${config.color.replace("text-", "bg-")} ${
            config.animate ? "animate-pulse" : ""
          }`}
        />
        <span className={`text-sm font-medium ${config.color}`}>
          {config.label}
        </span>
      </div>

      {/* Agent info */}
      <span className="text-xs text-gray-500">
        {session.agentType}
        {session.model ? ` / ${session.model}` : ""}
      </span>

      {/* Duration */}
      <span className="text-xs text-gray-600">{elapsed}</span>

      {/* PID */}
      {session.pid && (
        <span className="text-[10px] text-gray-700 font-mono">
          PID {session.pid}
        </span>
      )}

      {/* Action buttons */}
      {showActions && (
        <div className="flex items-center gap-1 ml-auto">
          {session.status === "running" && (
            <>
              <button
                type="button"
                onClick={handlePause}
                className="px-2 py-0.5 text-xs text-yellow-400 hover:bg-yellow-500/20 rounded"
                title="Pause agent (SIGSTOP)"
              >
                Pause
              </button>
              <button
                type="button"
                onClick={handleKill}
                className="px-2 py-0.5 text-xs text-red-400 hover:bg-red-500/20 rounded"
                title="Kill agent (SIGKILL)"
              >
                Kill
              </button>
            </>
          )}
          {session.status === "paused" && (
            <>
              <button
                type="button"
                onClick={handleResume}
                className="px-2 py-0.5 text-xs text-green-400 hover:bg-green-500/20 rounded"
                title="Resume agent (SIGCONT)"
              >
                Resume
              </button>
              <button
                type="button"
                onClick={handleKill}
                className="px-2 py-0.5 text-xs text-red-400 hover:bg-red-500/20 rounded"
                title="Kill agent"
              >
                Kill
              </button>
            </>
          )}
          {session.status === "crashed" && (
            <button
              type="button"
              onClick={handleResume}
              className="px-2 py-0.5 text-xs text-blue-400 hover:bg-blue-500/20 rounded"
              title="Retry agent"
            >
              Retry
            </button>
          )}
        </div>
      )}
    </div>
  );
}

/** Format milliseconds to human-readable duration */
function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) return `${minutes}m ${remainingSeconds}s`;
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return `${hours}h ${remainingMinutes}m`;
}
```

- [ ] **Step 2: Verify AgentStatus renders**

Render `<AgentStatus sessionId="sess-1" />` with the mock session from Task 9. Run `pnpm dev`. Confirm: green pulsing dot, "Running" label, agent type, Pause and Kill buttons visible.

---

### Task 13: Create CostTracker.tsx — Token and cost display

**Files:**
- Create: `~/projects/koompi-orch/src/components/agent/CostTracker.tsx`

- [ ] **Step 1: Create CostTracker.tsx**

Create `src/components/agent/CostTracker.tsx`:
```typescript
import { useAgentStore } from "../../stores/agentStore";

interface CostTrackerProps {
  sessionId: string;
}

export function CostTracker({ sessionId }: CostTrackerProps) {
  const session = useAgentStore((s) => s.sessions[sessionId]);

  if (!session) {
    return null;
  }

  const { metrics } = session;

  const totalTokens = metrics.tokensIn + metrics.tokensOut;
  const duration = metrics.durationMs;

  return (
    <div className="flex flex-col gap-2 px-3 py-2">
      <h4 className="text-xs font-semibold uppercase text-gray-400">
        Metrics
      </h4>

      <div className="grid grid-cols-2 gap-2">
        {/* Tokens In */}
        <MetricCard
          label="Tokens In"
          value={formatNumber(metrics.tokensIn)}
          sublabel={`${((metrics.tokensIn / (totalTokens || 1)) * 100).toFixed(0)}%`}
        />

        {/* Tokens Out */}
        <MetricCard
          label="Tokens Out"
          value={formatNumber(metrics.tokensOut)}
          sublabel={`${((metrics.tokensOut / (totalTokens || 1)) * 100).toFixed(0)}%`}
        />

        {/* Cost */}
        <MetricCard
          label="Cost"
          value={`$${metrics.costUsd.toFixed(2)}`}
          sublabel={totalTokens > 0 ? `$${((metrics.costUsd / totalTokens) * 1000).toFixed(3)}/1k tok` : ""}
          highlight={metrics.costUsd > 5}
        />

        {/* Duration */}
        <MetricCard
          label="Duration"
          value={formatDuration(duration)}
          sublabel={totalTokens > 0 ? `${(totalTokens / (duration / 1000)).toFixed(0)} tok/s` : ""}
        />
      </div>

      {/* Token bar */}
      <div className="mt-1">
        <div className="flex items-center justify-between text-[10px] text-gray-500 mb-0.5">
          <span>Total: {formatNumber(totalTokens)} tokens</span>
        </div>
        <div className="h-1.5 bg-gray-800 rounded-full overflow-hidden flex">
          <div
            className="bg-blue-500 h-full"
            style={{
              width: `${totalTokens > 0 ? (metrics.tokensIn / totalTokens) * 100 : 50}%`,
            }}
          />
          <div
            className="bg-green-500 h-full"
            style={{
              width: `${totalTokens > 0 ? (metrics.tokensOut / totalTokens) * 100 : 50}%`,
            }}
          />
        </div>
        <div className="flex items-center gap-3 mt-1 text-[10px] text-gray-600">
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-blue-500" /> Input
          </span>
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-green-500" /> Output
          </span>
        </div>
      </div>
    </div>
  );
}

function MetricCard({
  label,
  value,
  sublabel,
  highlight = false,
}: {
  label: string;
  value: string;
  sublabel?: string;
  highlight?: boolean;
}) {
  return (
    <div className="bg-gray-800/50 rounded-md px-2.5 py-1.5">
      <div className="text-[10px] text-gray-500 uppercase">{label}</div>
      <div
        className={`text-sm font-mono font-medium ${
          highlight ? "text-red-400" : "text-gray-200"
        }`}
      >
        {value}
      </div>
      {sublabel && (
        <div className="text-[10px] text-gray-600">{sublabel}</div>
      )}
    </div>
  );
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) return `${minutes}m ${remainingSeconds}s`;
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return `${hours}h ${remainingMinutes}m`;
}
```

- [ ] **Step 2: Verify CostTracker renders**

Render `<CostTracker sessionId="sess-1" />` with the mock session from Task 9. Run `pnpm dev`. Confirm: four metric cards (Tokens In, Tokens Out, Cost, Duration), token bar with blue/green segments, legends.

---

## Chunk 4: Command Palette

### Task 14: Create CommandPalette.tsx — Mod+K fuzzy search

**Files:**
- Create: `~/projects/koompi-orch/src/components/layout/CommandPalette.tsx`

- [ ] **Step 1: Create CommandPalette.tsx**

Create `src/components/layout/CommandPalette.tsx`:
```typescript
import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useWorkspaceStore } from "../../stores/workspaceStore";

export interface PaletteAction {
  id: string;
  label: string;
  description?: string;
  shortcut?: string;
  category: "workspace" | "agent" | "navigation" | "action";
  onExecute: () => void;
}

interface CommandPaletteProps {
  /** Additional actions beyond the built-in ones */
  actions?: PaletteAction[];
  /** Callback for workspace navigation */
  onNavigateWorkspace?: (workspaceId: string) => void;
  /** Callback for creating new workspace */
  onNewWorkspace?: () => void;
}

/** Simple fuzzy match: checks if all characters in query appear in target in order */
function fuzzyMatch(query: string, target: string): { match: boolean; score: number } {
  const q = query.toLowerCase();
  const t = target.toLowerCase();

  if (q.length === 0) return { match: true, score: 0 };

  let qi = 0;
  let score = 0;
  let prevMatchIndex = -1;

  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) {
      // Consecutive matches score higher
      if (prevMatchIndex === ti - 1) score += 2;
      // Word boundary matches score higher
      if (ti === 0 || t[ti - 1] === " " || t[ti - 1] === "-" || t[ti - 1] === "/") {
        score += 3;
      }
      score += 1;
      prevMatchIndex = ti;
      qi++;
    }
  }

  return { match: qi === q.length, score };
}

export function CommandPalette({
  actions: externalActions = [],
  onNavigateWorkspace,
  onNewWorkspace,
}: CommandPaletteProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const selectWorkspace = useWorkspaceStore((s) => s.selectWorkspace);

  // Built-in actions
  const builtInActions: PaletteAction[] = useMemo(
    () => [
      {
        id: "new-workspace",
        label: "New Workspace",
        description: "Create a new workspace with agent",
        shortcut: "Mod+N",
        category: "action",
        onExecute: () => onNewWorkspace?.(),
      },
      {
        id: "toggle-left-sidebar",
        label: "Toggle Left Sidebar",
        shortcut: "Mod+[",
        category: "navigation",
        onExecute: () => {
          document.dispatchEvent(
            new CustomEvent("koompi-orch:toggle-sidebar", {
              detail: { side: "left" },
            })
          );
        },
      },
      {
        id: "toggle-right-sidebar",
        label: "Toggle Right Sidebar",
        shortcut: "Mod+]",
        category: "navigation",
        onExecute: () => {
          document.dispatchEvent(
            new CustomEvent("koompi-orch:toggle-sidebar", {
              detail: { side: "right" },
            })
          );
        },
      },
      {
        id: "zen-mode",
        label: "Zen Mode",
        description: "Hide both sidebars",
        shortcut: "Mod+Shift+Z",
        category: "navigation",
        onExecute: () => {
          document.dispatchEvent(
            new CustomEvent("koompi-orch:zen-mode")
          );
        },
      },
      {
        id: "copy-chat-markdown",
        label: "Copy Chat as Markdown",
        shortcut: "Mod+Shift+C",
        category: "action",
        onExecute: () => {
          document.dispatchEvent(
            new CustomEvent("koompi-orch:copy-chat")
          );
        },
      },
      // Dynamic workspace entries
      ...workspaces.map((ws) => ({
        id: `ws-${ws.id}`,
        label: ws.name,
        description: `${ws.repoName} / ${ws.branch} [${ws.status}]`,
        category: "workspace" as const,
        onExecute: () => {
          selectWorkspace(ws.id);
          onNavigateWorkspace?.(ws.id);
        },
      })),
    ],
    [workspaces, selectWorkspace, onNavigateWorkspace, onNewWorkspace]
  );

  const allActions = useMemo(
    () => [...builtInActions, ...externalActions],
    [builtInActions, externalActions]
  );

  // Filter and sort by fuzzy match score
  const filteredActions = useMemo(() => {
    if (!query.trim()) return allActions;
    return allActions
      .map((action) => {
        const labelMatch = fuzzyMatch(query, action.label);
        const descMatch = action.description
          ? fuzzyMatch(query, action.description)
          : { match: false, score: 0 };
        const bestScore = Math.max(
          labelMatch.match ? labelMatch.score : 0,
          descMatch.match ? descMatch.score : 0
        );
        return { action, match: labelMatch.match || descMatch.match, score: bestScore };
      })
      .filter((r) => r.match)
      .sort((a, b) => b.score - a.score)
      .map((r) => r.action);
  }, [allActions, query]);

  // Reset selected index when results change
  useEffect(() => {
    setSelectedIndex(0);
  }, [filteredActions.length]);

  // Global Mod+K listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setOpen((prev) => !prev);
        setQuery("");
        setSelectedIndex(0);
      }
      if (e.key === "Escape" && open) {
        e.preventDefault();
        setOpen(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open]);

  // Focus input when opened
  useEffect(() => {
    if (open) {
      // Small delay to ensure DOM is ready
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // Scroll selected item into view
  useEffect(() => {
    const list = listRef.current;
    if (!list) return;
    const selected = list.children[selectedIndex] as HTMLElement | undefined;
    selected?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  const executeAction = useCallback(
    (action: PaletteAction) => {
      setOpen(false);
      setQuery("");
      action.onExecute();
    },
    []
  );

  const handleInputKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) =>
        prev < filteredActions.length - 1 ? prev + 1 : 0
      );
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) =>
        prev > 0 ? prev - 1 : filteredActions.length - 1
      );
    } else if (e.key === "Enter") {
      e.preventDefault();
      const action = filteredActions[selectedIndex];
      if (action) executeAction(action);
    }
  };

  if (!open) return null;

  // Group by category
  const categoryOrder: PaletteAction["category"][] = [
    "action",
    "workspace",
    "navigation",
    "agent",
  ];
  const categoryLabels: Record<string, string> = {
    action: "Actions",
    workspace: "Workspaces",
    navigation: "Navigation",
    agent: "Agents",
  };

  let globalIndex = 0;

  return (
    <div className="fixed inset-0 z-[100] flex items-start justify-center pt-[15vh]">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={() => setOpen(false)}
        onKeyDown={() => {}}
        role="presentation"
      />

      {/* Palette */}
      <div className="relative w-full max-w-lg bg-gray-800 border border-gray-700 rounded-xl shadow-2xl overflow-hidden">
        {/* Search input */}
        <div className="flex items-center px-4 py-3 border-b border-gray-700">
          <span className="text-gray-500 mr-2 text-sm">&#x1F50D;</span>
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleInputKeyDown}
            placeholder="Search actions, workspaces..."
            className="flex-1 bg-transparent text-sm text-gray-200 outline-none placeholder:text-gray-600"
          />
          <kbd className="text-[10px] text-gray-600 bg-gray-900 px-1.5 py-0.5 rounded border border-gray-700">
            ESC
          </kbd>
        </div>

        {/* Results */}
        <div ref={listRef} className="max-h-[50vh] overflow-y-auto py-1">
          {filteredActions.length === 0 ? (
            <div className="text-sm text-gray-500 text-center py-6">
              No results for "{query}"
            </div>
          ) : (
            categoryOrder.map((category) => {
              const items = filteredActions.filter(
                (a) => a.category === category
              );
              if (items.length === 0) return null;

              return (
                <div key={category}>
                  <div className="px-4 py-1 text-[10px] font-semibold uppercase text-gray-600">
                    {categoryLabels[category]}
                  </div>
                  {items.map((action) => {
                    const idx = globalIndex++;
                    return (
                      <button
                        key={action.id}
                        type="button"
                        onClick={() => executeAction(action)}
                        onMouseEnter={() => setSelectedIndex(idx)}
                        className={`
                          w-full text-left flex items-center justify-between px-4 py-2 text-sm
                          ${idx === selectedIndex ? "bg-blue-500/15 text-blue-300" : "text-gray-300 hover:bg-white/5"}
                        `}
                      >
                        <div className="flex-1 min-w-0">
                          <span className="truncate">{action.label}</span>
                          {action.description && (
                            <span className="ml-2 text-xs text-gray-500 truncate">
                              {action.description}
                            </span>
                          )}
                        </div>
                        {action.shortcut && (
                          <kbd className="text-[10px] text-gray-600 bg-gray-900 px-1.5 py-0.5 rounded border border-gray-700 ml-2 whitespace-nowrap">
                            {action.shortcut}
                          </kbd>
                        )}
                      </button>
                    );
                  })}
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center gap-3 px-4 py-2 border-t border-gray-700 text-[10px] text-gray-600">
          <span>
            <kbd className="bg-gray-900 px-1 py-0.5 rounded border border-gray-700">
              &#x2191;&#x2193;
            </kbd>{" "}
            navigate
          </span>
          <span>
            <kbd className="bg-gray-900 px-1 py-0.5 rounded border border-gray-700">
              &#x23CE;
            </kbd>{" "}
            select
          </span>
          <span>
            <kbd className="bg-gray-900 px-1 py-0.5 rounded border border-gray-700">
              esc
            </kbd>{" "}
            close
          </span>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify CommandPalette renders**

Add `<CommandPalette />` to the root App.tsx. Run `pnpm dev`. Press Mod+K (Ctrl+K on Linux). Confirm: palette opens with search input, shows built-in actions grouped by category, arrow keys navigate, Enter selects, Escape closes, fuzzy filtering works when typing.

---

## Verification Checklist

After completing all tasks:

- [ ] Run `pnpm dev` and verify the app builds and loads without errors
- [ ] Verify all three Zustand stores can be imported and used without type errors
- [ ] Verify WorkspaceCard renders with status badge and branch info
- [ ] Verify KanbanBoard renders five columns and drag-drop moves cards
- [ ] Verify WorkspaceCreate dialog opens and shows all form fields
- [ ] Verify MultiRepoSelector renders repo list with search
- [ ] Verify FileTree renders with expand/collapse and git status colors
- [ ] Verify ChatView renders user/assistant/tool messages with markdown
- [ ] Verify ChatInput renders with agent/model dropdown
- [ ] Verify AgentPicker renders agent and role tabs with selection
- [ ] Verify AgentStatus renders status dot, label, and action buttons
- [ ] Verify CostTracker renders metric cards and token bar
- [ ] Verify CommandPalette opens with Mod+K, filters actions, and navigates with keyboard
- [ ] Run `pnpm build` to confirm no TypeScript errors
