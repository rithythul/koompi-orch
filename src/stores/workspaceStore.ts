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
