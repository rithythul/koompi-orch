import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";
import { ProjectDashboard } from "./ProjectDashboard";
import { MetricsChart } from "./MetricsChart";
import { GlobalSearch } from "./GlobalSearch";

// Mock recharts to avoid canvas rendering in tests
vi.mock("recharts", () => ({
  ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  LineChart: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="line-chart">{children}</div>
  ),
  Line: () => <div data-testid="line" />,
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
  Tooltip: () => <div data-testid="tooltip" />,
  CartesianGrid: () => <div data-testid="cartesian-grid" />,
  BarChart: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="bar-chart">{children}</div>
  ),
  Bar: () => <div data-testid="bar" />,
  Legend: () => <div data-testid="legend" />,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

describe("ProjectDashboard", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders summary stat cards", () => {
    render(
      <ProjectDashboard
        stats={{
          totalWorkspaces: 12,
          activeAgents: 3,
          totalCostUsd: 45.67,
          totalTokens: 1234567,
        }}
        recentSessions={[]}
      />
    );
    expect(screen.getByText("12")).toBeDefined();
    expect(screen.getByText("3")).toBeDefined();
    expect(screen.getByText("$45.67")).toBeDefined();
    expect(screen.getByText("Workspaces")).toBeDefined();
    expect(screen.getByText("Active Agents")).toBeDefined();
    expect(screen.getByText("Total Cost")).toBeDefined();
  });

  it("renders recent sessions list", () => {
    render(
      <ProjectDashboard
        stats={{
          totalWorkspaces: 1,
          activeAgents: 0,
          totalCostUsd: 0,
          totalTokens: 0,
        }}
        recentSessions={[
          {
            id: "s1",
            workspaceName: "feat-auth",
            agentType: "claude-code",
            status: "completed",
            costUsd: 1.23,
            startedAt: "2026-03-14T10:00:00Z",
          },
        ]}
      />
    );
    expect(screen.getByText("feat-auth")).toBeDefined();
    expect(screen.getByText("claude-code")).toBeDefined();
  });
});

describe("MetricsChart", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders chart container with title", () => {
    render(
      <MetricsChart
        title="Cost Over Time"
        data={[
          { label: "Mon", value: 1.5 },
          { label: "Tue", value: 2.3 },
        ]}
        dataKey="value"
        color="#7aa2f7"
      />
    );
    expect(screen.getByText("Cost Over Time")).toBeDefined();
    expect(screen.getByTestId("responsive-container")).toBeDefined();
  });

  it("renders bar chart when type is bar", () => {
    render(
      <MetricsChart
        title="Tokens by Agent"
        data={[{ label: "claude", value: 5000 }]}
        dataKey="value"
        color="#9ece6a"
        chartType="bar"
      />
    );
    expect(screen.getByTestId("bar-chart")).toBeDefined();
  });
});

describe("GlobalSearch", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders search input", () => {
    render(<GlobalSearch onSelect={vi.fn()} />);
    expect(screen.getByPlaceholderText("Search all workspaces, sessions, files...")).toBeDefined();
  });

  it("calls onSelect when a result is clicked", () => {
    const onSelect = vi.fn();
    render(<GlobalSearch onSelect={onSelect} results={[
      { id: "ws-1", type: "workspace", title: "feat-auth", subtitle: "my-app" },
    ]} />);
    fireEvent.click(screen.getByText("feat-auth"));
    expect(onSelect).toHaveBeenCalledWith({
      id: "ws-1",
      type: "workspace",
      title: "feat-auth",
      subtitle: "my-app",
    });
  });
});
