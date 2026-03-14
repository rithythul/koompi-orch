import { Suspense, useState, useCallback } from "react";
import { Routes, Route } from "react-router-dom";
import { lazy } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PipelineStep } from "../agent/PipelineBuilder";
import { useWorkspaceStore, type Workspace, type WorkspaceStatus } from "../../stores/workspaceStore";
import { useAgentStore } from "../../stores/agentStore";

/** Extract a string ID from SurrealDB Thing (handles various serialization formats) */
function extractId(thing: unknown): string {
  if (!thing) return "";
  if (typeof thing === "string") return thing;
  const t = thing as Record<string, unknown>;
  // Format: { tb: "workspace", id: { String: "xxx" } }
  if (t.tb && t.id) {
    const inner = t.id;
    if (typeof inner === "string") return `${t.tb}:${inner}`;
    if (typeof inner === "object" && inner !== null) {
      const s = (inner as Record<string, unknown>).String;
      if (typeof s === "string") return `${t.tb}:${s}`;
    }
    return `${t.tb}:${JSON.stringify(t.id)}`;
  }
  return String(thing);
}

const ProjectDashboard = lazy(
  () => import("../dashboard/ProjectDashboard").then((m) => ({ default: m.ProjectDashboard }))
);
const SettingsPage = lazy(
  () => import("../settings/SettingsPage").then((m) => ({ default: m.SettingsPage }))
);
const PluginList = lazy(
  () => import("../plugins/PluginList").then((m) => ({ default: m.PluginList }))
);
const PipelineBuilder = lazy(
  () => import("../agent/PipelineBuilder").then((m) => ({ default: m.PipelineBuilder }))
);

const defaultPipelineSteps = [
  { id: "s1", role: "architect", agentType: "claude-code" as const },
  { id: "s2", role: "implementer", agentType: "claude-code" as const },
  { id: "s3", role: "reviewer", agentType: "codex" as const },
];

const defaultPlugins = [
  { id: "p1", name: "GitHub Integration", version: "1.0.0", enabled: true, capabilities: ["git", "pr", "issues"], description: "Git operations and PR management", author: "koompi" },
  { id: "p2", name: "Slack Notifications", version: "0.9.0", enabled: false, capabilities: ["notify", "webhook"], description: "Send notifications to Slack channels", author: "koompi" },
  { id: "p3", name: "Docker Runner", version: "2.1.0", enabled: true, capabilities: ["container", "build"], description: "Build and run Docker containers", author: "koompi" },
];

export function CenterPanel() {
  return (
    <main className="flex-1 bg-primary flex flex-col h-full overflow-hidden relative z-[1]">
      <Suspense
        fallback={
          <div className="flex-1 flex items-center justify-center">
            <div className="flex items-center gap-3 text-text-tertiary text-[13px]">
              <div className="w-4 h-4 border-2 border-border border-t-accent rounded-full animate-spin" />
              Loading...
            </div>
          </div>
        }
      >
        <Routes>
          <Route path="/" element={<WorkspacesView />} />
          <Route path="/dashboard" element={<DashboardView />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/plugins" element={<PluginsView />} />
          <Route path="/pipelines" element={<PipelinesView />} />
          <Route path="/templates" element={<TemplatesView />} />
        </Routes>
      </Suspense>
    </main>
  );
}

/* — Reusable page shell with header — */
function PageShell({ title, subtitle, count, action, children }: {
  title: string;
  subtitle?: string;
  count?: number;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col h-full">
      <div className="h-[48px] px-6 flex items-center justify-between border-b border-border shrink-0">
        <div className="flex items-center gap-3">
          <h2 className="text-[13px] font-semibold text-text-primary">{title}</h2>
          {count !== undefined && (
            <span className="text-[10px] font-mono text-text-ghost bg-card-bg-hover px-1.5 py-0.5 rounded">
              {count}
            </span>
          )}
          {subtitle && (
            <>
              <span className="text-text-ghost">·</span>
              <span className="text-[11px] text-text-tertiary">{subtitle}</span>
            </>
          )}
        </div>
        {action}
      </div>
      <div className="flex-1 overflow-auto p-6">
        {children}
      </div>
    </div>
  );
}

/* — Workspaces / Kanban — */
function WorkspacesView() {
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const addWorkspace = useWorkspaceStore((s) => s.addWorkspace);
  const selectWorkspace = useWorkspaceStore((s) => s.selectWorkspace);
  const selectedWorkspaceId = useWorkspaceStore((s) => s.selectedWorkspaceId);
  const setActiveSession = useAgentStore((s) => s.setActiveSession);
  const setSession = useAgentStore((s) => s.setSession);
  const sessionForWorkspace = useAgentStore((s) => s.sessionForWorkspace);
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  const workspacesByStatus = (status: WorkspaceStatus) =>
    workspaces.filter((w) => w.status === status);

  const handleCreate = useCallback(async (name: string, branch: string, path: string) => {
    try {
      const ws = await invoke<Record<string, unknown>>(
        "create_workspace",
        { name, branch, worktreePath: path }
      );
      const newWs: Workspace = {
        id: extractId(ws.id) || `ws-${Date.now()}`,
        name: (ws.name as string) || name,
        branch: (ws.branch as string) || branch,
        worktreePath: (ws.worktree_path as string) || path,
        status: "backlog",
        repoId: "",
        repoName: "",
        lockedBy: null,
        hasConflict: false,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      addWorkspace(newWs);
      setShowCreateDialog(false);
    } catch (err) {
      console.error("Failed to create workspace:", err);
    }
  }, [addWorkspace]);

  const handleSelectWorkspace = useCallback((ws: Workspace) => {
    selectWorkspace(ws.id);
    // Create or find an agent session for this workspace
    const existing = sessionForWorkspace(ws.id);
    if (existing) {
      setActiveSession(existing.id);
    } else {
      const sessionId = `session-${Date.now()}`;
      setSession({
        id: sessionId,
        workspaceId: ws.id,
        agentType: "claude-code",
        model: null,
        rolePreset: null,
        status: "paused",
        pid: null,
        messages: [],
        metrics: { tokensIn: 0, tokensOut: 0, costUsd: 0, durationMs: 0 },
        startedAt: new Date().toISOString(),
        endedAt: null,
      });
      setActiveSession(sessionId);
    }
  }, [selectWorkspace, sessionForWorkspace, setActiveSession, setSession]);

  return (
    <PageShell
      title="Workspaces"
      subtitle="Kanban board"
      count={workspaces.length}
      action={
        <button
          onClick={() => setShowCreateDialog(true)}
          className="flex items-center gap-1.5 px-3 py-1.5 text-[12px] font-medium bg-accent hover:bg-accent-hover text-white rounded-md transition-colors duration-150"
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M8 3V13M3 8H13"/>
          </svg>
          New Workspace
        </button>
      }
    >
      {showCreateDialog && (
        <CreateWorkspaceDialog
          onClose={() => setShowCreateDialog(false)}
          onCreate={handleCreate}
        />
      )}
      <div className="grid grid-cols-4 gap-4 h-full">
        {(["backlog", "active", "review", "done"] as const).map((status) => (
          <KanbanColumn
            key={status}
            status={status}
            workspaces={workspacesByStatus(status)}
            selectedId={selectedWorkspaceId}
            onSelect={handleSelectWorkspace}
          />
        ))}
      </div>
    </PageShell>
  );
}

function KanbanColumn({ status, workspaces, selectedId, onSelect }: {
  status: string;
  workspaces: Workspace[];
  selectedId: string | null;
  onSelect: (ws: Workspace) => void;
}) {
  const config: Record<string, { color: string; dot: string }> = {
    backlog: { color: "text-text-tertiary", dot: "bg-text-ghost" },
    active: { color: "text-accent", dot: "bg-accent" },
    review: { color: "text-warning", dot: "bg-warning" },
    done: { color: "text-success", dot: "bg-success" },
  };
  const { color, dot } = config[status] ?? { color: "text-text-tertiary", dot: "bg-text-ghost" };

  return (
    <div className="bg-card-bg border border-border rounded-lg flex flex-col">
      <div className="px-3.5 py-3 border-b border-border flex items-center gap-2">
        <div className={`w-2 h-2 rounded-full ${dot}`} />
        <h3 className={`text-[11px] font-semibold uppercase tracking-wider ${color}`}>
          {status}
        </h3>
        <span className="ml-auto text-[10px] font-mono text-text-ghost">{workspaces.length}</span>
      </div>
      <div className="flex-1 p-3 min-h-[180px] flex flex-col gap-2">
        {workspaces.length === 0 ? (
          <p className="text-[11px] text-text-ghost text-center mt-8">
            Drop workspaces here
          </p>
        ) : (
          workspaces.map((ws) => (
            <button
              key={ws.id}
              onClick={() => onSelect(ws)}
              className={`w-full text-left px-3 py-2.5 rounded-md border transition-all duration-150 ${
                ws.id === selectedId
                  ? "border-accent bg-accent-muted"
                  : "border-border bg-card-bg-hover hover:border-border-strong"
              }`}
            >
              <div className="text-[12px] font-medium text-text-primary truncate">{ws.name}</div>
              <div className="text-[10px] font-mono text-text-ghost mt-0.5 truncate">{ws.branch}</div>
            </button>
          ))
        )}
      </div>
    </div>
  );
}

/* — Create Workspace Dialog — */
function CreateWorkspaceDialog({ onClose, onCreate }: {
  onClose: () => void;
  onCreate: (name: string, branch: string, path: string) => void;
}) {
  const [name, setName] = useState("");
  const [branch, setBranch] = useState("main");
  const [path, setPath] = useState("");

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="card-glass rounded-xl p-6 w-[400px] flex flex-col gap-4">
        <h3 className="text-[14px] font-semibold text-text-primary">New Workspace</h3>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Workspace name"
          className="bg-input-bg border border-border rounded-md px-3 py-2 text-[13px] text-text-primary placeholder:text-text-ghost focus:outline-none focus:border-accent transition-colors"
          autoFocus
        />
        <input
          type="text"
          value={branch}
          onChange={(e) => setBranch(e.target.value)}
          placeholder="Branch name"
          className="bg-input-bg border border-border rounded-md px-3 py-2 text-[13px] text-text-primary placeholder:text-text-ghost focus:outline-none focus:border-accent transition-colors"
        />
        <input
          type="text"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="Worktree path (e.g., /home/user/project)"
          className="bg-input-bg border border-border rounded-md px-3 py-2 text-[13px] text-text-primary placeholder:text-text-ghost focus:outline-none focus:border-accent transition-colors"
        />
        <div className="flex justify-end gap-2 pt-2">
          <button
            onClick={onClose}
            className="px-4 py-2 text-[12px] text-text-ghost hover:text-text-secondary transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={() => name.trim() && onCreate(name.trim(), branch.trim(), path.trim())}
            disabled={!name.trim()}
            className="px-4 py-2 text-[12px] font-medium bg-accent hover:bg-accent-hover text-white rounded-md transition-colors disabled:opacity-40"
          >
            Create
          </button>
        </div>
      </div>
    </div>
  );
}

/* — Dashboard (wired to real data) — */
function DashboardView() {
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const sessions = useAgentStore((s) => s.sessions);

  const sessionList = Object.values(sessions);
  const runningCount = sessionList.filter((s) => s.status === "running").length;
  const totalCost = sessionList.reduce((sum, s) => sum + s.metrics.costUsd, 0);
  const totalTokens = sessionList.reduce((sum, s) => sum + s.metrics.tokensIn + s.metrics.tokensOut, 0);

  const stats = {
    totalWorkspaces: workspaces.length,
    activeAgents: runningCount,
    totalCostUsd: totalCost,
    totalTokens,
  };

  const recentSessions = sessionList
    .sort((a, b) => new Date(b.startedAt).getTime() - new Date(a.startedAt).getTime())
    .slice(0, 10)
    .map((s) => {
      const ws = workspaces.find((w) => w.id === s.workspaceId);
      return {
        id: s.id,
        workspaceName: ws?.name ?? s.workspaceId,
        agentType: s.agentType,
        status: s.status,
        costUsd: s.metrics.costUsd,
        startedAt: s.startedAt,
      };
    });

  return (
    <ProjectDashboard stats={stats} recentSessions={recentSessions} />
  );
}

/* — Pipelines — */
function PipelinesView() {
  const [steps, setSteps] = useState<PipelineStep[]>(defaultPipelineSteps);

  return (
    <PageShell title="Pipelines" subtitle="Design multi-agent workflows">
      <PipelineBuilder steps={steps} onStepsChange={setSteps} />
    </PageShell>
  );
}

/* — Plugins — */
function PluginsView() {
  const [plugins, setPlugins] = useState(defaultPlugins);

  const handleToggle = (name: string, enabled: boolean) => {
    setPlugins((prev) =>
      prev.map((p) => (p.name === name ? { ...p, enabled } : p))
    );
  };

  return (
    <PageShell title="Plugins" subtitle="Manage extensions and integrations" count={plugins.length}>
      <PluginList plugins={plugins} onToggle={handleToggle} onSelect={() => {}} />
    </PageShell>
  );
}

/* — Templates — */
function TemplatesView() {
  const templates = [
    { name: "Claude Code", command: "claude", description: "Anthropic's CLI coding agent", builtIn: true, icon: "C" },
    { name: "Codex", command: "codex", description: "OpenAI's coding agent", builtIn: true, icon: "X" },
    { name: "Gemini CLI", command: "gemini", description: "Google's CLI coding agent", builtIn: true, icon: "G" },
    { name: "Aider", command: "aider", description: "Open-source AI pair programmer", builtIn: true, icon: "A" },
  ];

  return (
    <PageShell
      title="Agent Templates"
      subtitle="Pre-configured agent definitions"
      count={templates.length}
      action={
        <button className="flex items-center gap-1.5 px-3 py-1.5 text-[12px] font-medium bg-accent hover:bg-accent-hover text-white rounded-md transition-colors duration-150">
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M8 3V13M3 8H13"/>
          </svg>
          New Template
        </button>
      }
    >
      <div className="flex flex-col gap-2 stagger-children">
        {templates.map((t) => (
          <div key={t.name} className="card-glass rounded-lg px-4 py-3.5 flex items-center gap-4 transition-all duration-150 group cursor-pointer">
            <div className="w-9 h-9 rounded-lg bg-accent-muted flex items-center justify-center text-[13px] font-bold font-mono text-accent shrink-0">
              {t.icon}
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="text-[13px] font-medium text-text-primary">{t.name}</span>
                {t.builtIn && (
                  <span className="text-[9px] font-mono uppercase tracking-wider px-1.5 py-0.5 bg-accent-muted text-accent rounded">built-in</span>
                )}
              </div>
              <p className="text-[11px] text-text-tertiary mt-0.5">{t.description}</p>
            </div>
            <code className="text-[11px] font-mono text-text-ghost bg-input-bg px-2 py-1 rounded border border-border">
              {t.command}
            </code>
            <button className="text-[11px] text-text-ghost hover:text-text-secondary transition-colors opacity-0 group-hover:opacity-100">
              Edit
            </button>
          </div>
        ))}
      </div>
    </PageShell>
  );
}
