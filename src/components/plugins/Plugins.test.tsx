import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";
import { PluginList } from "./PluginList";
import { PluginManifest } from "./PluginManifest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

const mockPlugins = [
  {
    name: "code-reviewer",
    version: "0.1.0",
    description: "Adds AI code review pipeline step",
    author: "community",
    capabilities: ["pipeline_step", "event_handler"],
    enabled: true,
  },
  {
    name: "slack-notify",
    version: "0.2.1",
    description: "Send notifications to Slack channels",
    author: "koompi",
    capabilities: ["event_handler"],
    enabled: false,
  },
];

describe("PluginList", () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders plugin names and descriptions", () => {
    render(
      <PluginList
        plugins={mockPlugins}
        onToggle={vi.fn()}
        onSelect={vi.fn()}
      />
    );
    expect(screen.getByText("code-reviewer")).toBeDefined();
    expect(screen.getByText("Adds AI code review pipeline step")).toBeDefined();
    expect(screen.getByText("slack-notify")).toBeDefined();
  });

  it("shows enabled/disabled state", () => {
    render(
      <PluginList
        plugins={mockPlugins}
        onToggle={vi.fn()}
        onSelect={vi.fn()}
      />
    );
    const toggles = screen.getAllByRole("switch");
    expect(toggles).toHaveLength(2);
  });

  it("calls onToggle with plugin name and new state", () => {
    const onToggle = vi.fn();
    render(
      <PluginList
        plugins={mockPlugins}
        onToggle={onToggle}
        onSelect={vi.fn()}
      />
    );
    const toggles = screen.getAllByRole("switch");
    fireEvent.click(toggles[0]);
    expect(onToggle).toHaveBeenCalledWith("code-reviewer", false);
  });

  it("calls onSelect when plugin name is clicked", () => {
    const onSelect = vi.fn();
    render(
      <PluginList
        plugins={mockPlugins}
        onToggle={vi.fn()}
        onSelect={onSelect}
      />
    );
    fireEvent.click(screen.getByText("code-reviewer"));
    expect(onSelect).toHaveBeenCalledWith("code-reviewer");
  });
});

describe("PluginManifest", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders manifest details", () => {
    render(
      <PluginManifest
        manifest={{
          name: "code-reviewer",
          version: "0.1.0",
          description: "Adds AI code review pipeline step",
          author: "community",
          capabilities: ["pipeline_step", "event_handler"],
          wasmPath: "~/.koompi-orch/plugins/code-reviewer/plugin.wasm",
          configSchema: {
            api_key: { type: "string", required: true, secret: true },
          },
        }}
      />
    );
    expect(screen.getByText("code-reviewer")).toBeDefined();
    expect(screen.getByText("0.1.0")).toBeDefined();
    expect(screen.getByText("community")).toBeDefined();
    expect(screen.getByText("pipeline_step")).toBeDefined();
    expect(screen.getByText("event_handler")).toBeDefined();
  });

  it("renders config schema entries", () => {
    render(
      <PluginManifest
        manifest={{
          name: "test-plugin",
          version: "1.0.0",
          description: "Test",
          author: "test",
          capabilities: [],
          wasmPath: "",
          configSchema: {
            api_key: { type: "string", required: true, secret: true },
            max_retries: { type: "number", required: false, secret: false },
          },
        }}
      />
    );
    expect(screen.getByText("api_key")).toBeDefined();
    expect(screen.getByText("max_retries")).toBeDefined();
    expect(screen.getByText("secret")).toBeDefined();
  });
});
