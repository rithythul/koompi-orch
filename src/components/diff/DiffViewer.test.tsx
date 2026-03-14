import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup, fireEvent, act } from "@testing-library/react";
import { DiffViewer } from "./DiffViewer";
import { DiffComment } from "./DiffComment";
import { TurnDiff } from "./TurnDiff";
import { MergeActions } from "./MergeActions";

// Mock Monaco diff editor
vi.mock("@monaco-editor/react", () => ({
  DiffEditor: vi.fn(({ original, modified, language }: {
    original: string;
    modified: string;
    language: string;
  }) => (
    <div
      data-testid="monaco-diff-mock"
      data-original={original}
      data-modified={modified}
      data-language={language}
    />
  )),
  default: vi.fn(() => <div data-testid="monaco-editor-mock" />),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe("DiffViewer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    cleanup();
  });

  it("renders diff editor with original and modified content", () => {
    render(
      <DiffViewer
        filePath="src/main.rs"
        original="fn old() {}"
        modified="fn new() {}"
      />
    );
    const diff = screen.getByTestId("monaco-diff-mock");
    expect(diff.getAttribute("data-original")).toBe("fn old() {}");
    expect(diff.getAttribute("data-modified")).toBe("fn new() {}");
  });

  it("displays file path in header", () => {
    render(
      <DiffViewer filePath="src/lib.rs" original="" modified="" />
    );
    expect(screen.getByText("src/lib.rs")).toBeDefined();
  });

  it("detects language from file path", () => {
    render(
      <DiffViewer filePath="app.tsx" original="" modified="" />
    );
    const diff = screen.getByTestId("monaco-diff-mock");
    expect(diff.getAttribute("data-language")).toBe("typescript");
  });
});

describe("DiffComment", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders comment with author and content", () => {
    render(
      <DiffComment
        author="reviewer"
        content="This needs error handling"
        lineNumber={42}
        timestamp="2026-03-14T10:00:00Z"
      />
    );
    expect(screen.getByText("reviewer")).toBeDefined();
    expect(screen.getByText("This needs error handling")).toBeDefined();
    expect(screen.getByText("L42")).toBeDefined();
  });

  it("renders resolved state when resolved is true", () => {
    render(
      <DiffComment
        author="reviewer"
        content="Fixed"
        lineNumber={10}
        timestamp="2026-03-14T10:00:00Z"
        resolved
      />
    );
    expect(screen.getByTestId("comment-resolved-badge")).toBeDefined();
  });
});

describe("TurnDiff", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders turn number and file list", () => {
    render(
      <TurnDiff
        turn={3}
        files={[
          { path: "src/auth.rs", status: "M" },
          { path: "src/new.rs", status: "A" },
        ]}
        onSelectFile={vi.fn()}
      />
    );
    expect(screen.getByText("Turn 3")).toBeDefined();
    expect(screen.getByText("src/auth.rs")).toBeDefined();
    expect(screen.getByText("src/new.rs")).toBeDefined();
  });

  it("calls onSelectFile when a file is clicked", () => {
    const onSelect = vi.fn();
    render(
      <TurnDiff
        turn={1}
        files={[{ path: "src/main.rs", status: "M" }]}
        onSelectFile={onSelect}
      />
    );
    fireEvent.click(screen.getByText("src/main.rs"));
    expect(onSelect).toHaveBeenCalledWith("src/main.rs");
  });
});

describe("MergeActions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    cleanup();
  });

  it("renders commit, push, merge, and PR buttons", () => {
    render(
      <MergeActions
        workspaceId="ws-1"
        branch="feat/auth"
        hasChanges
        onAction={vi.fn()}
      />
    );
    expect(screen.getByText("Commit")).toBeDefined();
    expect(screen.getByText("Push")).toBeDefined();
    expect(screen.getByText("Merge")).toBeDefined();
    expect(screen.getByText("Create PR")).toBeDefined();
  });

  it("disables commit and push when hasChanges is false", () => {
    render(
      <MergeActions
        workspaceId="ws-1"
        branch="feat/auth"
        hasChanges={false}
        onAction={vi.fn()}
      />
    );
    expect(screen.getByText("Commit").closest("button")?.hasAttribute("disabled")).toBe(true);
    expect(screen.getByText("Push").closest("button")?.hasAttribute("disabled")).toBe(true);
  });

  it("calls onAction with the correct action type", async () => {
    const onAction = vi.fn();
    render(
      <MergeActions
        workspaceId="ws-1"
        branch="feat/auth"
        hasChanges
        onAction={onAction}
      />
    );
    await act(async () => {
      fireEvent.click(screen.getByText("Commit"));
    });
    expect(onAction).toHaveBeenCalledWith("commit");
  });
});
