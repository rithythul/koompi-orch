import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Suspense } from "react";

// Mock all lazy-loaded components
vi.mock("../components/dashboard/ProjectDashboard", () => ({
  ProjectDashboard: () => <div data-testid="dashboard-page">Dashboard</div>,
}));

vi.mock("../components/settings/SettingsPage", () => ({
  SettingsPage: () => <div data-testid="settings-page">Settings</div>,
}));

vi.mock("../components/plugins/PluginList", () => ({
  PluginList: () => <div data-testid="plugins-page">Plugins</div>,
}));

vi.mock("../components/terminal/Terminal", () => ({
  Terminal: () => <div data-testid="terminal-component">Terminal</div>,
}));

vi.mock("../components/editor/CodeEditor", () => ({
  CodeEditor: () => <div data-testid="editor-component">Editor</div>,
}));

vi.mock("../components/notifications/ToastContainer", () => ({
  ToastContainer: () => <div data-testid="toast-container">Toasts</div>,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

// Import after mocks
import { AppRoutes } from "./router";

function renderWithRouter(route: string) {
  return render(
    <MemoryRouter initialEntries={[route]}>
      <Suspense fallback={<div>Loading...</div>}>
        <AppRoutes />
      </Suspense>
    </MemoryRouter>
  );
}

describe("AppRoutes", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders dashboard at /dashboard", async () => {
    renderWithRouter("/dashboard");
    expect(await screen.findByTestId("dashboard-page")).toBeDefined();
  });

  it("renders settings at /settings", async () => {
    renderWithRouter("/settings");
    expect(await screen.findByTestId("settings-page")).toBeDefined();
  });

  it("renders plugins at /plugins", async () => {
    renderWithRouter("/plugins");
    expect(await screen.findByTestId("plugins-page")).toBeDefined();
  });
});
