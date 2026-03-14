import { Suspense, useState } from "react";
import { Routes, Route } from "react-router-dom";
import { lazy } from "react";
import type { PipelineStep } from "../agent/PipelineBuilder";

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

const defaultStats = {
  totalWorkspaces: 3,
  activeAgents: 5,
  totalCostUsd: 12.47,
  totalTokens: 284500,
};

const defaultSessions = [
  { id: "s1", workspaceName: "auth-service", agentType: "claude-code", status: "running" as const, costUsd: 0.42, startedAt: "2026-03-14T15:30:00Z" },
  { id: "s2", workspaceName: "payment-api", agentType: "codex", status: "completed" as const, costUsd: 1.87, startedAt: "2026-03-14T15:15:00Z" },
  { id: "s3", workspaceName: "ui-dashboard", agentType: "gemini-cli", status: "crashed" as const, costUsd: 0.05, startedAt: "2026-03-14T14:30:00Z" },
];

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
          <Route
            path="/dashboard"
            element={
              <ProjectDashboard
                stats={defaultStats}
                recentSessions={defaultSessions}
              />
            }
          />
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
  return (
    <PageShell
      title="Workspaces"
      subtitle="Kanban board"
      count={0}
      action={
        <button className="flex items-center gap-1.5 px-3 py-1.5 text-[12px] font-medium bg-accent hover:bg-accent-hover text-white rounded-md transition-colors duration-150">
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M8 3V13M3 8H13"/>
          </svg>
          New Workspace
        </button>
      }
    >
      <div className="grid grid-cols-4 gap-4 h-full">
        {(["backlog", "active", "review", "done"] as const).map((status) => (
          <KanbanColumn key={status} status={status} />
        ))}
      </div>
    </PageShell>
  );
}

function KanbanColumn({ status }: { status: string }) {
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
        <span className="ml-auto text-[10px] font-mono text-text-ghost">0</span>
      </div>
      <div className="flex-1 p-3 flex items-center justify-center min-h-[180px]">
        <p className="text-[11px] text-text-ghost text-center">
          Drop workspaces here
        </p>
      </div>
    </div>
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
