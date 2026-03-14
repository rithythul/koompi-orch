import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";
import { SettingsPage } from "./SettingsPage";
import { ApiKeyManager } from "./ApiKeyManager";
import { AgentTemplates } from "./AgentTemplates";
import { ThemeToggle } from "./ThemeToggle";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe("ThemeToggle", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders dark and light mode buttons", () => {
    render(<ThemeToggle theme="dark" onToggle={vi.fn()} />);
    expect(screen.getByText("Dark")).toBeDefined();
    expect(screen.getByText("Light")).toBeDefined();
  });

  it("highlights the active theme", () => {
    render(<ThemeToggle theme="dark" onToggle={vi.fn()} />);
    const darkBtn = screen.getByText("Dark").closest("button");
    expect(darkBtn?.className).toContain("bg-accent");
  });

  it("calls onToggle when the other theme is clicked", () => {
    const onToggle = vi.fn();
    render(<ThemeToggle theme="dark" onToggle={onToggle} />);
    fireEvent.click(screen.getByText("Light"));
    expect(onToggle).toHaveBeenCalledWith("light");
  });
});

describe("ApiKeyManager", () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders provider list", () => {
    render(
      <ApiKeyManager
        keys={[
          { provider: "anthropic", label: "Anthropic", hasKey: true },
          { provider: "openai", label: "OpenAI", hasKey: false },
        ]}
        onSaveKey={vi.fn()}
        onDeleteKey={vi.fn()}
      />
    );
    expect(screen.getByText("Anthropic")).toBeDefined();
    expect(screen.getByText("OpenAI")).toBeDefined();
  });

  it("shows configured badge when key exists", () => {
    render(
      <ApiKeyManager
        keys={[{ provider: "anthropic", label: "Anthropic", hasKey: true }]}
        onSaveKey={vi.fn()}
        onDeleteKey={vi.fn()}
      />
    );
    expect(screen.getByText("Configured")).toBeDefined();
  });

  it("shows not configured badge when key missing", () => {
    render(
      <ApiKeyManager
        keys={[{ provider: "openai", label: "OpenAI", hasKey: false }]}
        onSaveKey={vi.fn()}
        onDeleteKey={vi.fn()}
      />
    );
    expect(screen.getByText("Not configured")).toBeDefined();
  });
});

describe("AgentTemplates", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders template list with names", () => {
    render(
      <AgentTemplates
        templates={[
          {
            id: "claude-code",
            name: "Claude Code",
            command: "claude",
            args: ["--dangerously-skip-permissions"],
            inputMode: "pty_stdin",
            outputMode: "json_stream",
            builtIn: true,
          },
          {
            id: "codex",
            name: "Codex",
            command: "codex",
            args: [],
            inputMode: "pty_stdin",
            outputMode: "text_markers",
            builtIn: true,
          },
        ]}
        onSave={vi.fn()}
        onDelete={vi.fn()}
      />
    );
    expect(screen.getByText("Claude Code")).toBeDefined();
    expect(screen.getByText("Codex")).toBeDefined();
  });

  it("shows built-in badge for built-in templates", () => {
    render(
      <AgentTemplates
        templates={[
          {
            id: "claude-code",
            name: "Claude Code",
            command: "claude",
            args: [],
            inputMode: "pty_stdin",
            outputMode: "json_stream",
            builtIn: true,
          },
        ]}
        onSave={vi.fn()}
        onDelete={vi.fn()}
      />
    );
    expect(screen.getByText("Built-in")).toBeDefined();
  });
});

describe("SettingsPage", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders all settings sections", () => {
    render(<SettingsPage />);
    expect(screen.getByText("Appearance")).toBeDefined();
    expect(screen.getByText("API Keys")).toBeDefined();
    expect(screen.getByText("Agent Templates")).toBeDefined();
    expect(screen.getByText("General")).toBeDefined();
  });
});
