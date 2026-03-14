import { Suspense } from "react";
import { Routes, Route } from "react-router-dom";
import { lazy } from "react";

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
  { id: "p1", name: "GitHub Integration", version: "1.0.0", enabled: true, capabilities: ["git", "pr", "issues"] },
  { id: "p2", name: "Slack Notifications", version: "0.9.0", enabled: false, capabilities: ["notify", "webhook"] },
  { id: "p3", name: "Docker Runner", version: "2.1.0", enabled: true, capabilities: ["container", "build"] },
];

export function CenterPanel() {
  return (
    <main className="flex-1 bg-primary flex flex-col h-full overflow-hidden">
      <Suspense
        fallback={
          <div className="flex-1 flex items-center justify-center text-text-secondary text-sm">
            Loading...
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
          <Route
            path="/plugins"
            element={
              <PluginList
                plugins={defaultPlugins}
                onToggle={() => {}}
                onSelect={() => {}}
              />
            }
          />
          <Route
            path="/pipelines"
            element={
              <div className="p-4">
                <div className="border-b border-border pb-3 mb-4">
                  <h2 className="text-sm font-medium text-text-primary">Pipeline Builder</h2>
                </div>
                <PipelineBuilder
                  steps={defaultPipelineSteps}
                  onStepsChange={() => {}}
                />
              </div>
            }
          />
          <Route
            path="/templates"
            element={<TemplatesView />}
          />
        </Routes>
      </Suspense>
    </main>
  );
}

function WorkspacesView() {
  return (
    <>
      <div className="border-b border-border p-3 flex items-center justify-between">
        <h2 className="text-sm font-medium text-text-primary">Workspaces</h2>
        <button className="px-3 py-1 text-xs bg-accent hover:bg-accent-hover text-white rounded transition-colors">
          + New Workspace
        </button>
      </div>
      <div className="flex-1 p-4 overflow-auto">
        <div className="grid grid-cols-4 gap-3">
          {(["backlog", "active", "review", "done"] as const).map((status) => (
            <KanbanColumn key={status} status={status} />
          ))}
        </div>
      </div>
    </>
  );
}

function KanbanColumn({ status }: { status: string }) {
  const colors: Record<string, string> = {
    backlog: "text-text-secondary",
    active: "text-accent",
    review: "text-warning",
    done: "text-success",
  };

  return (
    <div className="bg-secondary rounded-lg p-3 min-h-[200px]">
      <h3 className={`text-xs font-semibold uppercase mb-3 ${colors[status] ?? "text-text-secondary"}`}>
        {status}
      </h3>
      <p className="text-xs text-text-secondary italic">No workspaces</p>
    </div>
  );
}

function TemplatesView() {
  const templates = [
    { name: "Claude Code", command: "claude", description: "Anthropic's CLI coding agent", builtIn: true },
    { name: "Codex", command: "codex", description: "OpenAI's coding agent", builtIn: true },
    { name: "Gemini CLI", command: "gemini", description: "Google's CLI coding agent", builtIn: true },
    { name: "Aider", command: "aider", description: "Open-source AI pair programmer", builtIn: true },
  ];

  return (
    <>
      <div className="border-b border-border p-3 flex items-center justify-between">
        <h2 className="text-sm font-medium text-text-primary">Agent Templates</h2>
        <button className="px-3 py-1 text-xs bg-accent hover:bg-accent-hover text-white rounded transition-colors">
          + New Template
        </button>
      </div>
      <div className="flex-1 p-4 overflow-auto">
        <div className="flex flex-col gap-2">
          {templates.map((t) => (
            <div key={t.name} className="px-4 py-3 bg-secondary border border-border rounded-lg flex items-center justify-between">
              <div>
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-text-primary">{t.name}</span>
                  {t.builtIn && (
                    <span className="text-[10px] px-1.5 py-0.5 bg-accent/20 text-accent rounded">Built-in</span>
                  )}
                </div>
                <p className="text-xs text-text-secondary mt-0.5">{t.description}</p>
                <code className="text-[11px] text-text-secondary mt-1 block">{t.command}</code>
              </div>
              <button className="text-xs text-text-secondary hover:text-text-primary transition-colors">
                Edit
              </button>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
