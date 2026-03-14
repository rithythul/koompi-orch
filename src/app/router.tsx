import { lazy } from "react";
import { Routes, Route } from "react-router-dom";

const ProjectDashboard = lazy(
  () => import("../components/dashboard/ProjectDashboard").then((m) => ({ default: m.ProjectDashboard }))
);
const SettingsPage = lazy(
  () => import("../components/settings/SettingsPage").then((m) => ({ default: m.SettingsPage }))
);
const PluginListPage = lazy(
  () => import("../components/plugins/PluginList").then((m) => ({ default: m.PluginList }))
);
const Terminal = lazy(
  () => import("../components/terminal/Terminal").then((m) => ({ default: m.Terminal }))
);
const CodeEditor = lazy(
  () => import("../components/editor/CodeEditor").then((m) => ({ default: m.CodeEditor }))
);
const DiffViewer = lazy(
  () => import("../components/diff/DiffViewer").then((m) => ({ default: m.DiffViewer }))
);

const defaultDashboardStats = {
  totalWorkspaces: 0,
  activeAgents: 0,
  totalCostUsd: 0,
  totalTokens: 0,
};

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<div data-testid="main-layout">Main</div>} />

      <Route
        path="/dashboard"
        element={
          <ProjectDashboard
            stats={defaultDashboardStats}
            recentSessions={[]}
          />
        }
      />

      <Route path="/settings" element={<SettingsPage />} />

      <Route
        path="/plugins"
        element={
          <PluginListPage
            plugins={[]}
            onToggle={() => {}}
            onSelect={() => {}}
          />
        }
      />

      <Route
        path="/terminal/:sessionId"
        element={<Terminal sessionId="debug" />}
      />

      <Route
        path="/editor"
        element={<CodeEditor filePath="" content="" />}
      />

      <Route
        path="/diff"
        element={<DiffViewer filePath="" original="" modified="" />}
      />
    </Routes>
  );
}
