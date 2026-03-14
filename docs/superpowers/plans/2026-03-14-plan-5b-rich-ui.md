# Plan 5B: Rich UI & Polish — Terminal, Editor, Diff, Dashboard, Settings, Plugins

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the rich interactive components: embedded terminal, code editor, diff viewer, project dashboard, settings UI, live query bridge, notification toasts, plugin UI stubs, and pipeline builder.
**Architecture:** React components consuming Zustand stores (from Plan 5A) and Tauri IPC. Heavy components (Terminal, Monaco) are lazy-loaded. SurrealDB LIVE queries push real-time updates via Tauri events.
**Tech Stack:** React, TypeScript, Tailwind CSS, Zustand, xterm.js v5 (@xterm/xterm, @xterm/addon-fit, @xterm/addon-web-links), @monaco-editor/react, recharts, @tauri-apps/api, Vitest, @testing-library/react
**Spec Reference:** Sections 5, 8, 12, 15a, 18 of the design spec

---

## Chunk 1: Terminal Component

### Task 1: Terminal component (xterm.js) — Embed PTY output from Tauri backend

**Files:**
- Create: `~/projects/koompi-orch/src/components/terminal/Terminal.tsx`
- Create: `~/projects/koompi-orch/src/components/terminal/Terminal.test.tsx`

- [ ] **Step 1: Install xterm.js dependencies**

```bash
cd ~/projects/koompi-orch && pnpm add @xterm/xterm @xterm/addon-fit @xterm/addon-web-links
```

- [ ] **Step 2: Write failing test for Terminal component**

Create `src/components/terminal/Terminal.test.tsx`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { Terminal } from "./Terminal";

// Mock @xterm/xterm
const mockWrite = vi.fn();
const mockDispose = vi.fn();
const mockOpen = vi.fn();
const mockOnData = vi.fn();
const mockLoadAddon = vi.fn();

vi.mock("@xterm/xterm", () => ({
  Terminal: vi.fn().mockImplementation(() => ({
    open: mockOpen,
    write: mockWrite,
    dispose: mockDispose,
    onData: mockOnData,
    loadAddon: mockLoadAddon,
    options: {},
  })),
}));

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: vi.fn().mockImplementation(() => ({
    fit: vi.fn(),
    dispose: vi.fn(),
  })),
}));

vi.mock("@xterm/addon-web-links", () => ({
  WebLinksAddon: vi.fn().mockImplementation(() => ({
    dispose: vi.fn(),
  })),
}));

// Mock Tauri APIs
const mockListen = vi.fn().mockResolvedValue(vi.fn());
const mockInvoke = vi.fn().mockResolvedValue(undefined);

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("Terminal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    cleanup();
  });

  it("renders terminal container with correct test id", () => {
    render(<Terminal sessionId="session-1" />);
    expect(screen.getByTestId("terminal-container")).toBeDefined();
  });

  it("initializes xterm instance on mount", async () => {
    render(<Terminal sessionId="session-1" />);
    const { Terminal: XTerm } = await import("@xterm/xterm");
    expect(XTerm).toHaveBeenCalledWith(
      expect.objectContaining({
        cursorBlink: true,
        fontFamily: "monospace",
        fontSize: 14,
      })
    );
    expect(mockOpen).toHaveBeenCalled();
  });

  it("subscribes to Tauri pty_output event for the session", () => {
    render(<Terminal sessionId="session-42" />);
    expect(mockListen).toHaveBeenCalledWith(
      "pty_output:session-42",
      expect.any(Function)
    );
  });

  it("sends user input to backend via invoke", () => {
    render(<Terminal sessionId="session-1" />);
    // Simulate onData callback
    const onDataCallback = mockOnData.mock.calls[0]?.[0];
    if (onDataCallback) {
      onDataCallback("hello");
      expect(mockInvoke).toHaveBeenCalledWith("pty_write", {
        sessionId: "session-1",
        data: "hello",
      });
    }
  });

  it("disposes xterm on unmount", () => {
    const { unmount } = render(<Terminal sessionId="session-1" />);
    unmount();
    expect(mockDispose).toHaveBeenCalled();
  });
});
```

Run test (should fail — component does not exist):
```bash
cd ~/projects/koompi-orch && pnpm test src/components/terminal/Terminal.test.tsx
```

Expected: test fails with "Cannot find module './Terminal'"

- [ ] **Step 3: Implement Terminal component**

Create `src/components/terminal/Terminal.tsx`:
```typescript
import { useEffect, useRef } from "react";
import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import "@xterm/xterm/css/xterm.css";

interface TerminalProps {
  sessionId: string;
  /** Font size in pixels (default: 14) */
  fontSize?: number;
  /** Whether the terminal is read-only (default: false) */
  readOnly?: boolean;
}

export function Terminal({
  sessionId,
  fontSize = 14,
  readOnly = false,
}: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new XTerm({
      cursorBlink: true,
      fontFamily: "monospace",
      fontSize,
      theme: {
        background: "#1a1b26",
        foreground: "#c0caf5",
        cursor: "#c0caf5",
        selectionBackground: "#33467c",
        black: "#15161e",
        red: "#f7768e",
        green: "#9ece6a",
        yellow: "#e0af68",
        blue: "#7aa2f7",
        magenta: "#bb9af7",
        cyan: "#7dcfff",
        white: "#a9b1d6",
      },
      scrollback: 10000,
      convertEol: true,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    term.open(containerRef.current);

    // Fit to container
    try {
      fitAddon.fit();
    } catch {
      // Container may not be visible yet
    }

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    // Send user input to backend PTY
    if (!readOnly) {
      term.onData((data: string) => {
        invoke("pty_write", { sessionId, data }).catch((err: unknown) => {
          console.error("Failed to write to PTY:", err);
        });
      });
    }

    // Listen for PTY output from backend
    let unlisten: (() => void) | undefined;
    listen<{ data: string }>(`pty_output:${sessionId}`, (event) => {
      term.write(event.payload.data);
    }).then((fn) => {
      unlisten = fn;
    });

    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      try {
        fitAddon.fit();
      } catch {
        // Ignore fit errors during transitions
      }
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      unlisten?.();
      resizeObserver.disconnect();
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, [sessionId, fontSize, readOnly]);

  return (
    <div
      ref={containerRef}
      data-testid="terminal-container"
      className="w-full h-full min-h-[200px] bg-[#1a1b26] rounded-lg overflow-hidden"
    />
  );
}
```

- [ ] **Step 4: Run tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/components/terminal/Terminal.test.tsx
```

Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
cd ~/projects/koompi-orch && git add src/components/terminal/Terminal.tsx src/components/terminal/Terminal.test.tsx && git commit -m "feat(ui): add Terminal component with xterm.js PTY integration"
```

---

## Chunk 2: Code Editor (Monaco)

### Task 2: CodeEditor component (Monaco) — Read-only file viewer with syntax highlighting

**Files:**
- Create: `~/projects/koompi-orch/src/components/editor/CodeEditor.tsx`
- Create: `~/projects/koompi-orch/src/components/editor/CodeEditor.test.tsx`

- [ ] **Step 1: Install Monaco editor**

```bash
cd ~/projects/koompi-orch && pnpm add @monaco-editor/react
```

- [ ] **Step 2: Write failing test for CodeEditor**

Create `src/components/editor/CodeEditor.test.tsx`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { CodeEditor } from "./CodeEditor";

// Mock @monaco-editor/react
vi.mock("@monaco-editor/react", () => ({
  default: vi.fn(({ value, language, options, ...rest }: {
    value: string;
    language: string;
    options: Record<string, unknown>;
    theme: string;
    height: string;
    "data-testid"?: string;
  }) => (
    <div
      data-testid="monaco-editor-mock"
      data-value={value}
      data-language={language}
      data-readonly={String(options?.readOnly ?? false)}
      data-theme={rest.theme}
    />
  )),
}));

describe("CodeEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    cleanup();
  });

  it("renders the Monaco editor wrapper", () => {
    render(<CodeEditor filePath="src/main.rs" content="fn main() {}" />);
    expect(screen.getByTestId("code-editor-wrapper")).toBeDefined();
  });

  it("passes content to Monaco editor", () => {
    render(<CodeEditor filePath="src/main.rs" content="fn main() {}" />);
    const editor = screen.getByTestId("monaco-editor-mock");
    expect(editor.getAttribute("data-value")).toBe("fn main() {}");
  });

  it("detects language from file extension", () => {
    render(<CodeEditor filePath="src/app.tsx" content="export default {}" />);
    const editor = screen.getByTestId("monaco-editor-mock");
    expect(editor.getAttribute("data-language")).toBe("typescript");
  });

  it("uses rust language for .rs files", () => {
    render(<CodeEditor filePath="src/main.rs" content="fn main() {}" />);
    const editor = screen.getByTestId("monaco-editor-mock");
    expect(editor.getAttribute("data-language")).toBe("rust");
  });

  it("defaults to read-only mode", () => {
    render(<CodeEditor filePath="test.py" content="print('hi')" />);
    const editor = screen.getByTestId("monaco-editor-mock");
    expect(editor.getAttribute("data-readonly")).toBe("true");
  });

  it("displays file path in header", () => {
    render(<CodeEditor filePath="src/lib/utils.ts" content="" />);
    expect(screen.getByText("src/lib/utils.ts")).toBeDefined();
  });

  it("uses dark theme by default", () => {
    render(<CodeEditor filePath="test.js" content="" />);
    const editor = screen.getByTestId("monaco-editor-mock");
    expect(editor.getAttribute("data-theme")).toBe("vs-dark");
  });
});
```

Run test (should fail):
```bash
cd ~/projects/koompi-orch && pnpm test src/components/editor/CodeEditor.test.tsx
```

Expected: fails with "Cannot find module './CodeEditor'"

- [ ] **Step 3: Implement CodeEditor component**

Create `src/components/editor/CodeEditor.tsx`:
```typescript
import MonacoEditor from "@monaco-editor/react";

interface CodeEditorProps {
  filePath: string;
  content: string;
  /** Override the auto-detected language */
  language?: string;
  /** Allow editing (default: false — read-only viewer) */
  readOnly?: boolean;
  /** Editor height (default: "100%") */
  height?: string;
  /** Theme (default: "vs-dark") */
  theme?: "vs-dark" | "light";
  /** Called when content changes (only when readOnly=false) */
  onChange?: (value: string | undefined) => void;
}

/** Map file extensions to Monaco language IDs */
function detectLanguage(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
  const langMap: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    rs: "rust",
    py: "python",
    go: "go",
    java: "java",
    kt: "kotlin",
    rb: "ruby",
    c: "c",
    cpp: "cpp",
    h: "c",
    hpp: "cpp",
    cs: "csharp",
    swift: "swift",
    md: "markdown",
    json: "json",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    xml: "xml",
    html: "html",
    css: "css",
    scss: "scss",
    sql: "sql",
    sh: "shell",
    bash: "shell",
    zsh: "shell",
    dockerfile: "dockerfile",
    graphql: "graphql",
    proto: "protobuf",
    lua: "lua",
    zig: "zig",
    surql: "sql",
  };
  return langMap[ext] ?? "plaintext";
}

export function CodeEditor({
  filePath,
  content,
  language,
  readOnly = true,
  height = "100%",
  theme = "vs-dark",
  onChange,
}: CodeEditorProps) {
  const resolvedLanguage = language ?? detectLanguage(filePath);

  return (
    <div
      data-testid="code-editor-wrapper"
      className="flex flex-col h-full bg-[#1e1e1e] rounded-lg overflow-hidden"
    >
      {/* File path header */}
      <div className="flex items-center px-3 py-1.5 bg-[#252526] border-b border-[#3c3c3c] text-xs text-gray-400">
        <span className="font-mono truncate">{filePath}</span>
        <span className="ml-auto text-[10px] text-gray-600 uppercase">
          {resolvedLanguage}
        </span>
      </div>

      {/* Editor */}
      <div className="flex-1 min-h-0">
        <MonacoEditor
          height={height}
          language={resolvedLanguage}
          value={content}
          theme={theme}
          onChange={onChange}
          options={{
            readOnly,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            fontSize: 13,
            lineNumbers: "on",
            renderLineHighlight: "line",
            wordWrap: "on",
            padding: { top: 8 },
            domReadOnly: readOnly,
          }}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Run tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/components/editor/CodeEditor.test.tsx
```

Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
cd ~/projects/koompi-orch && git add src/components/editor/CodeEditor.tsx src/components/editor/CodeEditor.test.tsx && git commit -m "feat(ui): add CodeEditor component with Monaco and language detection"
```

---

## Chunk 3: Diff Viewer Components

### Task 3: DiffViewer + DiffComment + TurnDiff + MergeActions

**Files:**
- Create: `~/projects/koompi-orch/src/components/diff/DiffViewer.tsx`
- Create: `~/projects/koompi-orch/src/components/diff/DiffComment.tsx`
- Create: `~/projects/koompi-orch/src/components/diff/TurnDiff.tsx`
- Create: `~/projects/koompi-orch/src/components/diff/MergeActions.tsx`
- Create: `~/projects/koompi-orch/src/components/diff/DiffViewer.test.tsx`

- [ ] **Step 1: Write failing test for DiffViewer**

Create `src/components/diff/DiffViewer.test.tsx`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";
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

  it("calls onAction with the correct action type", () => {
    const onAction = vi.fn();
    render(
      <MergeActions
        workspaceId="ws-1"
        branch="feat/auth"
        hasChanges
        onAction={onAction}
      />
    );
    fireEvent.click(screen.getByText("Commit"));
    expect(onAction).toHaveBeenCalledWith("commit");
  });
});
```

Run test (should fail):
```bash
cd ~/projects/koompi-orch && pnpm test src/components/diff/DiffViewer.test.tsx
```

Expected: fails with "Cannot find module './DiffViewer'"

- [ ] **Step 2: Implement DiffViewer**

Create `src/components/diff/DiffViewer.tsx`:
```typescript
import { DiffEditor } from "@monaco-editor/react";

interface DiffViewerProps {
  filePath: string;
  original: string;
  modified: string;
  /** Override language detection */
  language?: string;
  /** Editor height (default: "100%") */
  height?: string;
}

function detectLanguage(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
  const langMap: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    rs: "rust",
    py: "python",
    go: "go",
    java: "java",
    md: "markdown",
    json: "json",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    html: "html",
    css: "css",
    sql: "sql",
    sh: "shell",
  };
  return langMap[ext] ?? "plaintext";
}

export function DiffViewer({
  filePath,
  original,
  modified,
  language,
  height = "100%",
}: DiffViewerProps) {
  const resolvedLanguage = language ?? detectLanguage(filePath);

  return (
    <div
      data-testid="diff-viewer-wrapper"
      className="flex flex-col h-full bg-[#1e1e1e] rounded-lg overflow-hidden"
    >
      {/* Header */}
      <div className="flex items-center px-3 py-1.5 bg-[#252526] border-b border-[#3c3c3c] text-xs">
        <span className="font-mono text-gray-400 truncate">{filePath}</span>
        <span className="ml-auto text-[10px] text-gray-600 uppercase">
          {resolvedLanguage}
        </span>
      </div>

      {/* Diff editor */}
      <div className="flex-1 min-h-0">
        <DiffEditor
          height={height}
          language={resolvedLanguage}
          original={original}
          modified={modified}
          theme="vs-dark"
          options={{
            readOnly: true,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            fontSize: 13,
            renderSideBySide: true,
            renderOverviewRuler: false,
          }}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Implement DiffComment**

Create `src/components/diff/DiffComment.tsx`:
```typescript
interface DiffCommentProps {
  author: string;
  content: string;
  lineNumber: number;
  timestamp: string;
  resolved?: boolean;
  onResolve?: () => void;
}

export function DiffComment({
  author,
  content,
  lineNumber,
  timestamp,
  resolved = false,
  onResolve,
}: DiffCommentProps) {
  const formattedTime = new Date(timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  return (
    <div
      className={`border rounded-lg p-3 text-sm ${
        resolved
          ? "border-green-800/50 bg-green-900/10"
          : "border-gray-700 bg-gray-800/50"
      }`}
    >
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center gap-2">
          <span className="font-medium text-gray-200">{author}</span>
          <span className="text-[10px] font-mono text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">
            L{lineNumber}
          </span>
          {resolved && (
            <span
              data-testid="comment-resolved-badge"
              className="text-[10px] font-semibold text-green-400 bg-green-900/30 px-1.5 py-0.5 rounded"
            >
              Resolved
            </span>
          )}
        </div>
        <span className="text-[10px] text-gray-600">{formattedTime}</span>
      </div>
      <p className="text-gray-300 leading-relaxed">{content}</p>
      {!resolved && onResolve && (
        <button
          type="button"
          onClick={onResolve}
          className="mt-2 text-xs text-green-400 hover:text-green-300"
        >
          Mark resolved
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Implement TurnDiff**

Create `src/components/diff/TurnDiff.tsx`:
```typescript
interface TurnFile {
  path: string;
  status: string;
}

interface TurnDiffProps {
  turn: number;
  files: TurnFile[];
  onSelectFile: (filePath: string) => void;
  /** Whether this turn is currently selected/expanded */
  active?: boolean;
}

const STATUS_COLORS: Record<string, string> = {
  M: "text-yellow-400",
  A: "text-green-400",
  D: "text-red-400",
  R: "text-blue-400",
};

const STATUS_LABELS: Record<string, string> = {
  M: "Modified",
  A: "Added",
  D: "Deleted",
  R: "Renamed",
};

export function TurnDiff({
  turn,
  files,
  onSelectFile,
  active = false,
}: TurnDiffProps) {
  return (
    <div
      className={`border rounded-lg overflow-hidden ${
        active ? "border-blue-500/50" : "border-gray-700"
      }`}
    >
      {/* Turn header */}
      <div
        className={`flex items-center justify-between px-3 py-2 ${
          active ? "bg-blue-500/10" : "bg-gray-800/50"
        }`}
      >
        <span className="text-sm font-medium text-gray-200">Turn {turn}</span>
        <span className="text-xs text-gray-500">
          {files.length} file{files.length !== 1 ? "s" : ""}
        </span>
      </div>

      {/* File list */}
      <div className="divide-y divide-gray-800">
        {files.map((file) => (
          <button
            key={file.path}
            type="button"
            onClick={() => onSelectFile(file.path)}
            className="w-full text-left flex items-center gap-2 px-3 py-1.5 text-sm hover:bg-white/5 transition-colors"
          >
            <span
              className={`font-mono text-[10px] font-bold w-4 text-center ${
                STATUS_COLORS[file.status] ?? "text-gray-500"
              }`}
              title={STATUS_LABELS[file.status] ?? file.status}
            >
              {file.status}
            </span>
            <span className="font-mono text-gray-300 truncate">
              {file.path}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Implement MergeActions**

Create `src/components/diff/MergeActions.tsx`:
```typescript
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type MergeActionType = "commit" | "push" | "merge" | "create-pr";

interface MergeActionsProps {
  workspaceId: string;
  branch: string;
  hasChanges: boolean;
  onAction: (action: MergeActionType) => void;
}

interface ActionButton {
  type: MergeActionType;
  label: string;
  /** Disabled when no uncommitted changes exist */
  needsChanges: boolean;
  color: string;
  hoverColor: string;
}

const ACTIONS: ActionButton[] = [
  {
    type: "commit",
    label: "Commit",
    needsChanges: true,
    color: "bg-green-600",
    hoverColor: "hover:bg-green-700",
  },
  {
    type: "push",
    label: "Push",
    needsChanges: true,
    color: "bg-blue-600",
    hoverColor: "hover:bg-blue-700",
  },
  {
    type: "merge",
    label: "Merge",
    needsChanges: false,
    color: "bg-purple-600",
    hoverColor: "hover:bg-purple-700",
  },
  {
    type: "create-pr",
    label: "Create PR",
    needsChanges: false,
    color: "bg-orange-600",
    hoverColor: "hover:bg-orange-700",
  },
];

export function MergeActions({
  workspaceId,
  branch,
  hasChanges,
  onAction,
}: MergeActionsProps) {
  const [loading, setLoading] = useState<MergeActionType | null>(null);
  const [commitMessage, setCommitMessage] = useState("");

  const handleAction = async (action: MergeActionType) => {
    setLoading(action);
    try {
      if (action === "commit") {
        await invoke("git_commit", {
          workspaceId,
          message: commitMessage || `Agent changes on ${branch}`,
        });
        setCommitMessage("");
      } else if (action === "push") {
        await invoke("git_push", { workspaceId });
      } else if (action === "merge") {
        await invoke("git_merge_to_main", { workspaceId });
      } else if (action === "create-pr") {
        await invoke("create_pull_request", { workspaceId });
      }
      onAction(action);
    } catch (err) {
      console.error(`Failed to ${action}:`, err);
    } finally {
      setLoading(null);
    }
  };

  return (
    <div data-testid="merge-actions" className="flex flex-col gap-3">
      {/* Branch indicator */}
      <div className="flex items-center gap-2 text-xs text-gray-400">
        <span className="w-2 h-2 rounded-full bg-blue-500" />
        <span className="font-mono">{branch}</span>
      </div>

      {/* Commit message input */}
      <input
        type="text"
        value={commitMessage}
        onChange={(e) => setCommitMessage(e.target.value)}
        placeholder="Commit message (optional)"
        className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-1.5 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
      />

      {/* Action buttons */}
      <div className="flex flex-wrap gap-2">
        {ACTIONS.map((action) => {
          const disabled =
            (action.needsChanges && !hasChanges) || loading !== null;
          return (
            <button
              key={action.type}
              type="button"
              onClick={() => handleAction(action.type)}
              disabled={disabled}
              className={`px-3 py-1.5 text-xs font-medium text-white rounded-md transition-colors
                ${action.color} ${action.hoverColor}
                disabled:opacity-50 disabled:cursor-not-allowed`}
            >
              {loading === action.type ? "..." : action.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
```

- [ ] **Step 6: Run tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/components/diff/DiffViewer.test.tsx
```

Expected: all 10 tests pass (3 DiffViewer + 2 DiffComment + 2 TurnDiff + 3 MergeActions).

- [ ] **Step 7: Commit**

```bash
cd ~/projects/koompi-orch && git add src/components/diff/ && git commit -m "feat(ui): add DiffViewer, DiffComment, TurnDiff, and MergeActions components"
```

---

## Chunk 4: Dashboard Components

### Task 4: ProjectDashboard + MetricsChart + GlobalSearch

**Files:**
- Create: `~/projects/koompi-orch/src/components/dashboard/ProjectDashboard.tsx`
- Create: `~/projects/koompi-orch/src/components/dashboard/MetricsChart.tsx`
- Create: `~/projects/koompi-orch/src/components/dashboard/GlobalSearch.tsx`
- Create: `~/projects/koompi-orch/src/components/dashboard/Dashboard.test.tsx`

- [ ] **Step 1: Install recharts**

```bash
cd ~/projects/koompi-orch && pnpm add recharts
```

- [ ] **Step 2: Write failing test for dashboard components**

Create `src/components/dashboard/Dashboard.test.tsx`:
```typescript
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
```

Run test (should fail):
```bash
cd ~/projects/koompi-orch && pnpm test src/components/dashboard/Dashboard.test.tsx
```

Expected: fails with "Cannot find module './ProjectDashboard'"

- [ ] **Step 3: Implement MetricsChart**

Create `src/components/dashboard/MetricsChart.tsx`:
```typescript
import {
  ResponsiveContainer,
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
} from "recharts";

interface DataPoint {
  label: string;
  value: number;
  [key: string]: string | number;
}

interface MetricsChartProps {
  title: string;
  data: DataPoint[];
  dataKey: string;
  color: string;
  /** Chart type: "line" (default) or "bar" */
  chartType?: "line" | "bar";
  /** Chart height in pixels (default: 200) */
  height?: number;
  /** Format function for tooltip values */
  formatValue?: (value: number) => string;
}

export function MetricsChart({
  title,
  data,
  dataKey,
  color,
  chartType = "line",
  height = 200,
  formatValue,
}: MetricsChartProps) {
  const tooltipFormatter = formatValue
    ? (val: number) => [formatValue(val), title]
    : undefined;

  return (
    <div className="bg-gray-800/50 border border-gray-700 rounded-lg p-4">
      <h3 className="text-sm font-medium text-gray-300 mb-3">{title}</h3>
      <ResponsiveContainer width="100%" height={height}>
        {chartType === "bar" ? (
          <BarChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#2d2d3f" />
            <XAxis
              dataKey="label"
              tick={{ fontSize: 11, fill: "#6b7280" }}
              axisLine={{ stroke: "#374151" }}
            />
            <YAxis
              tick={{ fontSize: 11, fill: "#6b7280" }}
              axisLine={{ stroke: "#374151" }}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "#1f2937",
                border: "1px solid #374151",
                borderRadius: "8px",
                fontSize: "12px",
              }}
              formatter={tooltipFormatter}
            />
            <Bar dataKey={dataKey} fill={color} radius={[4, 4, 0, 0]} />
          </BarChart>
        ) : (
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#2d2d3f" />
            <XAxis
              dataKey="label"
              tick={{ fontSize: 11, fill: "#6b7280" }}
              axisLine={{ stroke: "#374151" }}
            />
            <YAxis
              tick={{ fontSize: 11, fill: "#6b7280" }}
              axisLine={{ stroke: "#374151" }}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "#1f2937",
                border: "1px solid #374151",
                borderRadius: "8px",
                fontSize: "12px",
              }}
              formatter={tooltipFormatter}
            />
            <Line
              type="monotone"
              dataKey={dataKey}
              stroke={color}
              strokeWidth={2}
              dot={{ fill: color, r: 3 }}
              activeDot={{ r: 5 }}
            />
          </LineChart>
        )}
      </ResponsiveContainer>
    </div>
  );
}
```

- [ ] **Step 4: Implement ProjectDashboard**

Create `src/components/dashboard/ProjectDashboard.tsx`:
```typescript
interface DashboardStats {
  totalWorkspaces: number;
  activeAgents: number;
  totalCostUsd: number;
  totalTokens: number;
}

interface RecentSession {
  id: string;
  workspaceName: string;
  agentType: string;
  status: string;
  costUsd: number;
  startedAt: string;
}

interface ProjectDashboardProps {
  stats: DashboardStats;
  recentSessions: RecentSession[];
}

const STATUS_COLORS: Record<string, string> = {
  running: "bg-blue-500",
  paused: "bg-yellow-500",
  completed: "bg-green-500",
  crashed: "bg-red-500",
};

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function StatCard({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="bg-gray-800/50 border border-gray-700 rounded-lg p-4">
      <div className="text-xs text-gray-500 mb-1">{label}</div>
      <div className={`text-2xl font-bold ${color}`}>{value}</div>
    </div>
  );
}

export function ProjectDashboard({
  stats,
  recentSessions,
}: ProjectDashboardProps) {
  return (
    <div className="flex flex-col gap-6 p-4">
      {/* Summary cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          label="Workspaces"
          value={String(stats.totalWorkspaces)}
          color="text-gray-100"
        />
        <StatCard
          label="Active Agents"
          value={String(stats.activeAgents)}
          color="text-blue-400"
        />
        <StatCard
          label="Total Cost"
          value={`$${stats.totalCostUsd.toFixed(2)}`}
          color="text-yellow-400"
        />
        <StatCard
          label="Tokens Used"
          value={formatTokens(stats.totalTokens)}
          color="text-purple-400"
        />
      </div>

      {/* Recent sessions */}
      <div className="bg-gray-800/50 border border-gray-700 rounded-lg">
        <div className="px-4 py-3 border-b border-gray-700">
          <h3 className="text-sm font-medium text-gray-300">
            Recent Sessions
          </h3>
        </div>
        {recentSessions.length === 0 ? (
          <div className="px-4 py-8 text-sm text-gray-500 text-center">
            No sessions yet. Create a workspace and start an agent.
          </div>
        ) : (
          <div className="divide-y divide-gray-700/50">
            {recentSessions.map((session) => (
              <div
                key={session.id}
                className="flex items-center justify-between px-4 py-3 hover:bg-white/5"
              >
                <div className="flex items-center gap-3">
                  <span
                    className={`w-2 h-2 rounded-full ${
                      STATUS_COLORS[session.status] ?? "bg-gray-500"
                    }`}
                  />
                  <div>
                    <div className="text-sm font-medium text-gray-200">
                      {session.workspaceName}
                    </div>
                    <div className="text-xs text-gray-500">
                      {session.agentType}
                    </div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-xs text-gray-400">
                    ${session.costUsd.toFixed(2)}
                  </div>
                  <div className="text-[10px] text-gray-600">
                    {new Date(session.startedAt).toLocaleDateString()}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Implement GlobalSearch**

Create `src/components/dashboard/GlobalSearch.tsx`:
```typescript
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface SearchResult {
  id: string;
  type: "workspace" | "session" | "file";
  title: string;
  subtitle: string;
}

interface GlobalSearchProps {
  onSelect: (result: SearchResult) => void;
  /** Pre-populated results (for testing / static use) */
  results?: SearchResult[];
}

const TYPE_ICONS: Record<string, string> = {
  workspace: "W",
  session: "S",
  file: "F",
};

const TYPE_COLORS: Record<string, string> = {
  workspace: "bg-blue-500/20 text-blue-400",
  session: "bg-green-500/20 text-green-400",
  file: "bg-gray-500/20 text-gray-400",
};

export function GlobalSearch({ onSelect, results: staticResults }: GlobalSearchProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>(staticResults ?? []);
  const [loading, setLoading] = useState(false);

  const search = useCallback(
    async (q: string) => {
      setQuery(q);
      if (staticResults) {
        // When static results are provided, filter client-side
        setResults(
          staticResults.filter(
            (r) =>
              r.title.toLowerCase().includes(q.toLowerCase()) ||
              r.subtitle.toLowerCase().includes(q.toLowerCase())
          )
        );
        return;
      }
      if (q.length < 2) {
        setResults([]);
        return;
      }
      setLoading(true);
      try {
        const res = await invoke<SearchResult[]>("global_search", {
          query: q,
        });
        setResults(res);
      } catch (err) {
        console.error("Search failed:", err);
        setResults([]);
      } finally {
        setLoading(false);
      }
    },
    [staticResults]
  );

  return (
    <div className="flex flex-col gap-2">
      <input
        type="text"
        value={query}
        onChange={(e) => search(e.target.value)}
        placeholder="Search all workspaces, sessions, files..."
        className="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm text-gray-200 focus:outline-none focus:border-blue-500 placeholder:text-gray-600"
      />

      {loading && (
        <div className="text-xs text-gray-500 px-2">Searching...</div>
      )}

      {results.length > 0 && (
        <div className="border border-gray-700 rounded-lg overflow-hidden bg-gray-800/80 max-h-80 overflow-y-auto">
          {results.map((result) => (
            <button
              key={`${result.type}-${result.id}`}
              type="button"
              onClick={() => onSelect(result)}
              className="w-full text-left flex items-center gap-3 px-3 py-2.5 text-sm hover:bg-white/5 transition-colors border-b border-gray-800 last:border-b-0"
            >
              <span
                className={`w-6 h-6 rounded flex items-center justify-center text-[10px] font-bold ${
                  TYPE_COLORS[result.type] ?? "bg-gray-700 text-gray-400"
                }`}
              >
                {TYPE_ICONS[result.type] ?? "?"}
              </span>
              <div className="flex-1 min-w-0">
                <div className="font-medium text-gray-200 truncate">
                  {result.title}
                </div>
                <div className="text-xs text-gray-500 truncate">
                  {result.subtitle}
                </div>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 6: Run tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/components/dashboard/Dashboard.test.tsx
```

Expected: all 7 tests pass (3 ProjectDashboard + 2 MetricsChart + 2 GlobalSearch).

- [ ] **Step 7: Commit**

```bash
cd ~/projects/koompi-orch && git add src/components/dashboard/ && git commit -m "feat(ui): add ProjectDashboard, MetricsChart, and GlobalSearch components"
```

---

## Chunk 5: Settings Components

### Task 5: SettingsPage + ApiKeyManager + AgentTemplates + ThemeToggle

**Files:**
- Create: `~/projects/koompi-orch/src/components/settings/SettingsPage.tsx`
- Create: `~/projects/koompi-orch/src/components/settings/ApiKeyManager.tsx`
- Create: `~/projects/koompi-orch/src/components/settings/AgentTemplates.tsx`
- Create: `~/projects/koompi-orch/src/components/settings/ThemeToggle.tsx`
- Create: `~/projects/koompi-orch/src/stores/settingsStore.ts`
- Create: `~/projects/koompi-orch/src/components/settings/Settings.test.tsx`

- [ ] **Step 1: Write failing test for settings components**

Create `src/components/settings/Settings.test.tsx`:
```typescript
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
    expect(darkBtn?.className).toContain("bg-blue-500");
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
```

Run test (should fail):
```bash
cd ~/projects/koompi-orch && pnpm test src/components/settings/Settings.test.tsx
```

Expected: fails with "Cannot find module './SettingsPage'"

- [ ] **Step 2: Create settingsStore**

Create `src/stores/settingsStore.ts`:
```typescript
import { create } from "zustand";

export type Theme = "dark" | "light";

export interface ApiKeyEntry {
  provider: string;
  label: string;
  hasKey: boolean;
}

export interface AgentTemplate {
  id: string;
  name: string;
  command: string;
  args: string[];
  inputMode: string;
  outputMode: string;
  builtIn: boolean;
}

interface SettingsState {
  theme: Theme;
  maxConcurrentAgents: number;
  defaultAgent: string;
  defaultRole: string;
  autoReview: boolean;
  autoCheckpoint: boolean;
  apiKeys: ApiKeyEntry[];
  templates: AgentTemplate[];

  // Actions
  setTheme: (theme: Theme) => void;
  setMaxConcurrentAgents: (max: number) => void;
  setDefaultAgent: (agent: string) => void;
  setDefaultRole: (role: string) => void;
  setAutoReview: (auto: boolean) => void;
  setAutoCheckpoint: (auto: boolean) => void;
  setApiKeys: (keys: ApiKeyEntry[]) => void;
  updateApiKey: (provider: string, hasKey: boolean) => void;
  setTemplates: (templates: AgentTemplate[]) => void;
  addTemplate: (template: AgentTemplate) => void;
  updateTemplate: (id: string, patch: Partial<AgentTemplate>) => void;
  removeTemplate: (id: string) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  theme: "dark",
  maxConcurrentAgents: 10,
  defaultAgent: "claude-code",
  defaultRole: "implementer",
  autoReview: true,
  autoCheckpoint: true,
  apiKeys: [],
  templates: [],

  setTheme: (theme) => set({ theme }),
  setMaxConcurrentAgents: (max) => set({ maxConcurrentAgents: max }),
  setDefaultAgent: (agent) => set({ defaultAgent: agent }),
  setDefaultRole: (role) => set({ defaultRole: role }),
  setAutoReview: (auto) => set({ autoReview: auto }),
  setAutoCheckpoint: (auto) => set({ autoCheckpoint: auto }),
  setApiKeys: (keys) => set({ apiKeys: keys }),
  updateApiKey: (provider, hasKey) =>
    set((state) => ({
      apiKeys: state.apiKeys.map((k) =>
        k.provider === provider ? { ...k, hasKey } : k
      ),
    })),
  setTemplates: (templates) => set({ templates }),
  addTemplate: (template) =>
    set((state) => ({ templates: [...state.templates, template] })),
  updateTemplate: (id, patch) =>
    set((state) => ({
      templates: state.templates.map((t) =>
        t.id === id ? { ...t, ...patch } : t
      ),
    })),
  removeTemplate: (id) =>
    set((state) => ({
      templates: state.templates.filter((t) => t.id !== id),
    })),
}));
```

- [ ] **Step 3: Implement ThemeToggle**

Create `src/components/settings/ThemeToggle.tsx`:
```typescript
type Theme = "dark" | "light";

interface ThemeToggleProps {
  theme: Theme;
  onToggle: (theme: Theme) => void;
}

export function ThemeToggle({ theme, onToggle }: ThemeToggleProps) {
  return (
    <div className="flex items-center gap-1 bg-gray-800 rounded-lg p-1">
      <button
        type="button"
        onClick={() => onToggle("dark")}
        className={`px-3 py-1.5 text-xs font-medium rounded-md transition-colors ${
          theme === "dark"
            ? "bg-blue-500 text-white"
            : "text-gray-400 hover:text-gray-200"
        }`}
      >
        Dark
      </button>
      <button
        type="button"
        onClick={() => onToggle("light")}
        className={`px-3 py-1.5 text-xs font-medium rounded-md transition-colors ${
          theme === "light"
            ? "bg-blue-500 text-white"
            : "text-gray-400 hover:text-gray-200"
        }`}
      >
        Light
      </button>
    </div>
  );
}
```

- [ ] **Step 4: Implement ApiKeyManager**

Create `src/components/settings/ApiKeyManager.tsx`:
```typescript
import { useState } from "react";

interface ApiKeyEntry {
  provider: string;
  label: string;
  hasKey: boolean;
}

interface ApiKeyManagerProps {
  keys: ApiKeyEntry[];
  onSaveKey: (provider: string, key: string) => void;
  onDeleteKey: (provider: string) => void;
}

export function ApiKeyManager({
  keys,
  onSaveKey,
  onDeleteKey,
}: ApiKeyManagerProps) {
  const [editingProvider, setEditingProvider] = useState<string | null>(null);
  const [keyValue, setKeyValue] = useState("");

  const handleSave = (provider: string) => {
    if (keyValue.trim()) {
      onSaveKey(provider, keyValue.trim());
      setKeyValue("");
      setEditingProvider(null);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      {keys.map((entry) => (
        <div
          key={entry.provider}
          className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg"
        >
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-gray-200">
              {entry.label}
            </span>
            {entry.hasKey ? (
              <span className="text-[10px] font-semibold text-green-400 bg-green-900/30 px-1.5 py-0.5 rounded">
                Configured
              </span>
            ) : (
              <span className="text-[10px] font-semibold text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">
                Not configured
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {editingProvider === entry.provider ? (
              <>
                <input
                  type="password"
                  value={keyValue}
                  onChange={(e) => setKeyValue(e.target.value)}
                  placeholder="sk-..."
                  className="w-48 bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-gray-200 focus:outline-none focus:border-blue-500"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleSave(entry.provider);
                    if (e.key === "Escape") {
                      setEditingProvider(null);
                      setKeyValue("");
                    }
                  }}
                />
                <button
                  type="button"
                  onClick={() => handleSave(entry.provider)}
                  className="px-2 py-1 text-xs text-green-400 hover:text-green-300"
                >
                  Save
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setEditingProvider(null);
                    setKeyValue("");
                  }}
                  className="px-2 py-1 text-xs text-gray-500 hover:text-gray-300"
                >
                  Cancel
                </button>
              </>
            ) : (
              <>
                <button
                  type="button"
                  onClick={() => setEditingProvider(entry.provider)}
                  className="px-2 py-1 text-xs text-blue-400 hover:text-blue-300"
                >
                  {entry.hasKey ? "Update" : "Add"}
                </button>
                {entry.hasKey && (
                  <button
                    type="button"
                    onClick={() => onDeleteKey(entry.provider)}
                    className="px-2 py-1 text-xs text-red-400 hover:text-red-300"
                  >
                    Delete
                  </button>
                )}
              </>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 5: Implement AgentTemplates**

Create `src/components/settings/AgentTemplates.tsx`:
```typescript
import { useState } from "react";

interface AgentTemplate {
  id: string;
  name: string;
  command: string;
  args: string[];
  inputMode: string;
  outputMode: string;
  builtIn: boolean;
}

interface AgentTemplatesProps {
  templates: AgentTemplate[];
  onSave: (template: AgentTemplate) => void;
  onDelete: (id: string) => void;
}

export function AgentTemplates({
  templates,
  onSave,
  onDelete,
}: AgentTemplatesProps) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editForm, setEditForm] = useState<Partial<AgentTemplate>>({});

  const startEdit = (template: AgentTemplate) => {
    setEditingId(template.id);
    setEditForm({ ...template });
  };

  const handleSave = () => {
    if (editingId && editForm.name && editForm.command) {
      onSave({
        id: editForm.id ?? editingId,
        name: editForm.name,
        command: editForm.command,
        args: editForm.args ?? [],
        inputMode: editForm.inputMode ?? "pty_stdin",
        outputMode: editForm.outputMode ?? "text_markers",
        builtIn: editForm.builtIn ?? false,
      });
      setEditingId(null);
      setEditForm({});
    }
  };

  return (
    <div className="flex flex-col gap-2">
      {templates.map((template) => (
        <div
          key={template.id}
          className="px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg"
        >
          {editingId === template.id ? (
            <div className="flex flex-col gap-2">
              <input
                type="text"
                value={editForm.name ?? ""}
                onChange={(e) =>
                  setEditForm({ ...editForm, name: e.target.value })
                }
                placeholder="Template name"
                className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              />
              <input
                type="text"
                value={editForm.command ?? ""}
                onChange={(e) =>
                  setEditForm({ ...editForm, command: e.target.value })
                }
                placeholder="Command (e.g., claude)"
                className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              />
              <input
                type="text"
                value={(editForm.args ?? []).join(" ")}
                onChange={(e) =>
                  setEditForm({
                    ...editForm,
                    args: e.target.value.split(" ").filter(Boolean),
                  })
                }
                placeholder="Args (space-separated)"
                className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              />
              <div className="flex gap-2">
                <select
                  value={editForm.inputMode ?? "pty_stdin"}
                  onChange={(e) =>
                    setEditForm({ ...editForm, inputMode: e.target.value })
                  }
                  className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-gray-200"
                >
                  <option value="pty_stdin">PTY stdin</option>
                  <option value="flag_message">Flag message</option>
                  <option value="file_prompt">File prompt</option>
                </select>
                <select
                  value={editForm.outputMode ?? "text_markers"}
                  onChange={(e) =>
                    setEditForm({ ...editForm, outputMode: e.target.value })
                  }
                  className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-gray-200"
                >
                  <option value="json_stream">JSON stream</option>
                  <option value="text_markers">Text markers</option>
                  <option value="raw_pty">Raw PTY</option>
                </select>
              </div>
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={handleSave}
                  className="px-3 py-1 text-xs text-green-400 hover:text-green-300 border border-green-800 rounded"
                >
                  Save
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setEditingId(null);
                    setEditForm({});
                  }}
                  className="px-3 py-1 text-xs text-gray-500 hover:text-gray-300"
                >
                  Cancel
                </button>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-gray-200">
                  {template.name}
                </span>
                {template.builtIn && (
                  <span className="text-[10px] font-semibold text-blue-400 bg-blue-900/30 px-1.5 py-0.5 rounded">
                    Built-in
                  </span>
                )}
                <span className="text-xs text-gray-500 font-mono">
                  {template.command}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => startEdit(template)}
                  className="px-2 py-1 text-xs text-blue-400 hover:text-blue-300"
                >
                  Edit
                </button>
                {!template.builtIn && (
                  <button
                    type="button"
                    onClick={() => onDelete(template.id)}
                    className="px-2 py-1 text-xs text-red-400 hover:text-red-300"
                  >
                    Delete
                  </button>
                )}
              </div>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 6: Implement SettingsPage**

Create `src/components/settings/SettingsPage.tsx`:
```typescript
import { useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore, type Theme } from "../../stores/settingsStore";
import { ThemeToggle } from "./ThemeToggle";
import { ApiKeyManager } from "./ApiKeyManager";
import { AgentTemplates } from "./AgentTemplates";

const DEFAULT_API_KEYS = [
  { provider: "anthropic", label: "Anthropic", hasKey: false },
  { provider: "openai", label: "OpenAI", hasKey: false },
  { provider: "google", label: "Google (Gemini)", hasKey: false },
];

export function SettingsPage() {
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);
  const maxConcurrentAgents = useSettingsStore((s) => s.maxConcurrentAgents);
  const setMaxConcurrentAgents = useSettingsStore(
    (s) => s.setMaxConcurrentAgents
  );
  const defaultAgent = useSettingsStore((s) => s.defaultAgent);
  const setDefaultAgent = useSettingsStore((s) => s.setDefaultAgent);
  const defaultRole = useSettingsStore((s) => s.defaultRole);
  const setDefaultRole = useSettingsStore((s) => s.setDefaultRole);
  const autoReview = useSettingsStore((s) => s.autoReview);
  const setAutoReview = useSettingsStore((s) => s.setAutoReview);
  const autoCheckpoint = useSettingsStore((s) => s.autoCheckpoint);
  const setAutoCheckpoint = useSettingsStore((s) => s.setAutoCheckpoint);
  const apiKeys = useSettingsStore((s) => s.apiKeys);
  const setApiKeys = useSettingsStore((s) => s.setApiKeys);
  const updateApiKey = useSettingsStore((s) => s.updateApiKey);
  const templates = useSettingsStore((s) => s.templates);
  const setTemplates = useSettingsStore((s) => s.setTemplates);

  // Load settings on mount
  useEffect(() => {
    invoke<{ apiKeys: typeof DEFAULT_API_KEYS }>("get_settings")
      .then((settings) => {
        if (settings.apiKeys) setApiKeys(settings.apiKeys);
      })
      .catch(() => {
        // Use defaults if backend not available
        if (apiKeys.length === 0) setApiKeys(DEFAULT_API_KEYS);
      });

    invoke<typeof templates>("list_agent_templates")
      .then(setTemplates)
      .catch(() => {
        // Defaults handled by store
      });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleThemeToggle = useCallback(
    (newTheme: Theme) => {
      setTheme(newTheme);
      invoke("set_setting", { key: "theme", value: newTheme }).catch(
        (err: unknown) => console.error("Failed to save theme:", err)
      );
      // Apply to document
      document.documentElement.classList.toggle("dark", newTheme === "dark");
      document.documentElement.classList.toggle("light", newTheme === "light");
    },
    [setTheme]
  );

  const handleSaveKey = useCallback(
    (provider: string, key: string) => {
      invoke("save_api_key", { provider, key })
        .then(() => updateApiKey(provider, true))
        .catch((err: unknown) =>
          console.error("Failed to save API key:", err)
        );
    },
    [updateApiKey]
  );

  const handleDeleteKey = useCallback(
    (provider: string) => {
      invoke("delete_api_key", { provider })
        .then(() => updateApiKey(provider, false))
        .catch((err: unknown) =>
          console.error("Failed to delete API key:", err)
        );
    },
    [updateApiKey]
  );

  const handleSaveTemplate = useCallback(
    (template: (typeof templates)[0]) => {
      invoke("save_agent_template", { template })
        .then(() => {
          const exists = templates.find((t) => t.id === template.id);
          if (exists) {
            setTemplates(
              templates.map((t) => (t.id === template.id ? template : t))
            );
          } else {
            setTemplates([...templates, template]);
          }
        })
        .catch((err: unknown) =>
          console.error("Failed to save template:", err)
        );
    },
    [templates, setTemplates]
  );

  const handleDeleteTemplate = useCallback(
    (id: string) => {
      invoke("delete_agent_template", { id })
        .then(() => setTemplates(templates.filter((t) => t.id !== id)))
        .catch((err: unknown) =>
          console.error("Failed to delete template:", err)
        );
    },
    [templates, setTemplates]
  );

  return (
    <div className="max-w-2xl mx-auto py-6 px-4 flex flex-col gap-8">
      <h1 className="text-xl font-bold text-gray-100">Settings</h1>

      {/* Appearance */}
      <section>
        <h2 className="text-sm font-semibold text-gray-300 mb-3 uppercase tracking-wider">
          Appearance
        </h2>
        <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
          <span className="text-sm text-gray-300">Theme</span>
          <ThemeToggle theme={theme} onToggle={handleThemeToggle} />
        </div>
      </section>

      {/* General */}
      <section>
        <h2 className="text-sm font-semibold text-gray-300 mb-3 uppercase tracking-wider">
          General
        </h2>
        <div className="flex flex-col gap-3">
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">
              Max concurrent agents
            </span>
            <input
              type="number"
              min={1}
              max={50}
              value={maxConcurrentAgents}
              onChange={(e) =>
                setMaxConcurrentAgents(Number(e.target.value) || 10)
              }
              className="w-16 bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 text-center focus:outline-none focus:border-blue-500"
            />
          </div>
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">Default agent</span>
            <select
              value={defaultAgent}
              onChange={(e) => setDefaultAgent(e.target.value)}
              className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
            >
              <option value="claude-code">Claude Code</option>
              <option value="codex">Codex</option>
              <option value="gemini-cli">Gemini CLI</option>
              <option value="aider">Aider</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">Default role</span>
            <select
              value={defaultRole}
              onChange={(e) => setDefaultRole(e.target.value)}
              className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
            >
              <option value="architect">Architect</option>
              <option value="implementer">Implementer</option>
              <option value="reviewer">Reviewer</option>
              <option value="tester">Tester</option>
              <option value="shipper">Shipper</option>
              <option value="fixer">Fixer</option>
            </select>
          </div>
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">Auto-review</span>
            <button
              type="button"
              onClick={() => setAutoReview(!autoReview)}
              className={`w-10 h-5 rounded-full transition-colors ${
                autoReview ? "bg-blue-500" : "bg-gray-600"
              }`}
            >
              <span
                className={`block w-4 h-4 rounded-full bg-white transform transition-transform ${
                  autoReview ? "translate-x-5" : "translate-x-0.5"
                }`}
              />
            </button>
          </div>
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">Auto-checkpoint</span>
            <button
              type="button"
              onClick={() => setAutoCheckpoint(!autoCheckpoint)}
              className={`w-10 h-5 rounded-full transition-colors ${
                autoCheckpoint ? "bg-blue-500" : "bg-gray-600"
              }`}
            >
              <span
                className={`block w-4 h-4 rounded-full bg-white transform transition-transform ${
                  autoCheckpoint ? "translate-x-5" : "translate-x-0.5"
                }`}
              />
            </button>
          </div>
        </div>
      </section>

      {/* API Keys */}
      <section>
        <h2 className="text-sm font-semibold text-gray-300 mb-3 uppercase tracking-wider">
          API Keys
        </h2>
        <p className="text-xs text-gray-500 mb-3">
          Keys are stored securely via OS keychain (Stronghold). They are never
          written to config files.
        </p>
        <ApiKeyManager
          keys={apiKeys.length > 0 ? apiKeys : DEFAULT_API_KEYS}
          onSaveKey={handleSaveKey}
          onDeleteKey={handleDeleteKey}
        />
      </section>

      {/* Agent Templates */}
      <section>
        <h2 className="text-sm font-semibold text-gray-300 mb-3 uppercase tracking-wider">
          Agent Templates
        </h2>
        <AgentTemplates
          templates={templates}
          onSave={handleSaveTemplate}
          onDelete={handleDeleteTemplate}
        />
      </section>
    </div>
  );
}
```

- [ ] **Step 7: Run tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/components/settings/Settings.test.tsx
```

Expected: all 8 tests pass (3 ThemeToggle + 3 ApiKeyManager + 1 AgentTemplates + 1 SettingsPage).

- [ ] **Step 8: Commit**

```bash
cd ~/projects/koompi-orch && git add src/stores/settingsStore.ts src/components/settings/ && git commit -m "feat(ui): add SettingsPage, ApiKeyManager, AgentTemplates, ThemeToggle, and settingsStore"
```

---

## Chunk 6: LIVE Query Bridge

### Task 6: useLiveQuery hook — SurrealDB LIVE SELECT to React state via Tauri events

**Files:**
- Create: `~/projects/koompi-orch/src/hooks/useLiveQuery.ts`
- Create: `~/projects/koompi-orch/src/hooks/useLiveQuery.test.ts`

- [ ] **Step 1: Write failing test for useLiveQuery**

Create `src/hooks/useLiveQuery.test.ts`:
```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, cleanup } from "@testing-library/react";
import { useLiveQuery } from "./useLiveQuery";

// Track registered listeners and invoke callback
type ListenerCallback = (event: { payload: unknown }) => void;
const listeners = new Map<string, ListenerCallback>();
const mockUnlisten = vi.fn();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, callback: ListenerCallback) => {
    listeners.set(event, callback);
    return Promise.resolve(mockUnlisten);
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue("live-query-uuid-123"),
}));

describe("useLiveQuery", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners.clear();
  });

  afterEach(() => {
    cleanup();
  });

  it("returns initial empty data and loading true", () => {
    const { result } = renderHook(() =>
      useLiveQuery<{ id: string; name: string }>({
        table: "workspace",
        initialData: [],
      })
    );
    expect(result.current.data).toEqual([]);
    expect(result.current.loading).toBe(true);
  });

  it("registers a Tauri event listener for the table", async () => {
    const { listen } = await import("@tauri-apps/api/event");
    renderHook(() =>
      useLiveQuery<{ id: string }>({ table: "workspace", initialData: [] })
    );
    expect(listen).toHaveBeenCalledWith(
      "live:workspace",
      expect.any(Function)
    );
  });

  it("invokes start_live_query on mount", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    renderHook(() =>
      useLiveQuery<{ id: string }>({ table: "session", initialData: [] })
    );
    expect(invoke).toHaveBeenCalledWith("start_live_query", {
      table: "session",
    });
  });

  it("updates data on CREATE event", async () => {
    const { result } = renderHook(() =>
      useLiveQuery<{ id: string; name: string }>({
        table: "workspace",
        initialData: [],
      })
    );

    // Simulate Tauri event
    const callback = listeners.get("live:workspace");
    expect(callback).toBeDefined();

    act(() => {
      callback!({
        payload: {
          action: "CREATE",
          result: { id: "ws-1", name: "feat-auth" },
        },
      });
    });

    expect(result.current.data).toEqual([{ id: "ws-1", name: "feat-auth" }]);
  });

  it("updates existing record on UPDATE event", async () => {
    const { result } = renderHook(() =>
      useLiveQuery<{ id: string; name: string }>({
        table: "workspace",
        initialData: [{ id: "ws-1", name: "old-name" }],
      })
    );

    const callback = listeners.get("live:workspace");
    act(() => {
      callback!({
        payload: {
          action: "UPDATE",
          result: { id: "ws-1", name: "new-name" },
        },
      });
    });

    expect(result.current.data).toEqual([{ id: "ws-1", name: "new-name" }]);
  });

  it("removes record on DELETE event", async () => {
    const { result } = renderHook(() =>
      useLiveQuery<{ id: string; name: string }>({
        table: "workspace",
        initialData: [
          { id: "ws-1", name: "keep" },
          { id: "ws-2", name: "remove" },
        ],
      })
    );

    const callback = listeners.get("live:workspace");
    act(() => {
      callback!({
        payload: {
          action: "DELETE",
          result: { id: "ws-2", name: "remove" },
        },
      });
    });

    expect(result.current.data).toEqual([{ id: "ws-1", name: "keep" }]);
  });

  it("unlistens and stops query on unmount", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const { unmount } = renderHook(() =>
      useLiveQuery<{ id: string }>({ table: "workspace", initialData: [] })
    );

    unmount();

    expect(mockUnlisten).toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledWith("stop_live_query", {
      queryId: "live-query-uuid-123",
    });
  });
});
```

Run test (should fail):
```bash
cd ~/projects/koompi-orch && pnpm test src/hooks/useLiveQuery.test.ts
```

Expected: fails with "Cannot find module './useLiveQuery'"

- [ ] **Step 2: Implement useLiveQuery hook**

Create `src/hooks/useLiveQuery.ts`:
```typescript
import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

interface LiveQueryAction<T> {
  action: "CREATE" | "UPDATE" | "DELETE";
  result: T;
}

interface UseLiveQueryOptions<T extends { id: string }> {
  /** SurrealDB table name to subscribe to */
  table: string;
  /** Initial data (e.g., from a one-time query) */
  initialData: T[];
  /** Optional filter function applied to incoming records */
  filter?: (record: T) => boolean;
  /** Whether to enable the subscription (default: true) */
  enabled?: boolean;
}

interface UseLiveQueryResult<T> {
  data: T[];
  loading: boolean;
  error: string | null;
}

export function useLiveQuery<T extends { id: string }>({
  table,
  initialData,
  filter,
  enabled = true,
}: UseLiveQueryOptions<T>): UseLiveQueryResult<T> {
  const [data, setData] = useState<T[]>(initialData);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const queryIdRef = useRef<string | null>(null);

  const handleEvent = useCallback(
    (event: { payload: unknown }) => {
      const { action, result } = event.payload as LiveQueryAction<T>;
      if (filter && !filter(result)) return;

      setData((prev) => {
        switch (action) {
          case "CREATE":
            // Avoid duplicates
            if (prev.some((item) => item.id === result.id)) {
              return prev.map((item) =>
                item.id === result.id ? result : item
              );
            }
            return [...prev, result];

          case "UPDATE":
            return prev.map((item) =>
              item.id === result.id ? result : item
            );

          case "DELETE":
            return prev.filter((item) => item.id !== result.id);

          default:
            return prev;
        }
      });

      setLoading(false);
    },
    [filter]
  );

  useEffect(() => {
    if (!enabled) {
      setLoading(false);
      return;
    }

    let unlisten: (() => void) | undefined;
    let cancelled = false;

    const setup = async () => {
      try {
        // Listen for Tauri events from the backend LIVE query bridge
        unlisten = await listen(`live:${table}`, handleEvent);

        // Start the LIVE query on the backend
        const queryId = await invoke<string>("start_live_query", { table });
        if (!cancelled) {
          queryIdRef.current = queryId;
          setLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
          setLoading(false);
        }
      }
    };

    setup();

    return () => {
      cancelled = true;
      unlisten?.();

      // Stop the LIVE query on the backend
      if (queryIdRef.current) {
        invoke("stop_live_query", { queryId: queryIdRef.current }).catch(
          (err: unknown) =>
            console.error("Failed to stop live query:", err)
        );
        queryIdRef.current = null;
      }
    };
  }, [table, enabled, handleEvent]);

  return { data, loading, error };
}
```

- [ ] **Step 3: Run tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/hooks/useLiveQuery.test.ts
```

Expected: all 7 tests pass.

- [ ] **Step 4: Commit**

```bash
cd ~/projects/koompi-orch && git add src/hooks/useLiveQuery.ts src/hooks/useLiveQuery.test.ts && git commit -m "feat(ui): add useLiveQuery hook for SurrealDB LIVE SELECT bridge"
```

---

## Chunk 7: Notification System

### Task 7: Notification toast UI — Toast component wired to notificationStore

**Files:**
- Create: `~/projects/koompi-orch/src/components/notifications/ToastContainer.tsx`
- Create: `~/projects/koompi-orch/src/components/notifications/Toast.tsx`
- Create: `~/projects/koompi-orch/src/components/notifications/Notifications.test.tsx`

- [ ] **Step 1: Write failing test for notification UI**

Create `src/components/notifications/Notifications.test.tsx`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup, fireEvent, act } from "@testing-library/react";
import { Toast } from "./Toast";
import { ToastContainer } from "./ToastContainer";
import { useNotificationStore } from "../../stores/notificationStore";

describe("Toast", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders title and message", () => {
    render(
      <Toast
        notification={{
          id: "n1",
          type: "info",
          title: "Agent started",
          message: "Claude Code is running on feat-auth",
          autoCloseMs: 0,
          createdAt: Date.now(),
        }}
        onDismiss={vi.fn()}
      />
    );
    expect(screen.getByText("Agent started")).toBeDefined();
    expect(screen.getByText("Claude Code is running on feat-auth")).toBeDefined();
  });

  it("renders correct color for error type", () => {
    const { container } = render(
      <Toast
        notification={{
          id: "n2",
          type: "error",
          title: "Agent crashed",
          message: "Process exited with code 1",
          autoCloseMs: 0,
          createdAt: Date.now(),
        }}
        onDismiss={vi.fn()}
      />
    );
    const toastEl = container.firstChild as HTMLElement;
    expect(toastEl.className).toContain("border-red");
  });

  it("renders correct color for success type", () => {
    const { container } = render(
      <Toast
        notification={{
          id: "n3",
          type: "success",
          title: "Done",
          message: "Completed",
          autoCloseMs: 0,
          createdAt: Date.now(),
        }}
        onDismiss={vi.fn()}
      />
    );
    const toastEl = container.firstChild as HTMLElement;
    expect(toastEl.className).toContain("border-green");
  });

  it("calls onDismiss when close button is clicked", () => {
    const onDismiss = vi.fn();
    render(
      <Toast
        notification={{
          id: "n1",
          type: "info",
          title: "Test",
          message: "Test message",
          autoCloseMs: 0,
          createdAt: Date.now(),
        }}
        onDismiss={onDismiss}
      />
    );
    fireEvent.click(screen.getByTitle("Dismiss"));
    expect(onDismiss).toHaveBeenCalledWith("n1");
  });
});

describe("ToastContainer", () => {
  beforeEach(() => {
    cleanup();
    // Reset notification store
    act(() => {
      useNotificationStore.getState().clearAll();
    });
  });

  it("renders no toasts when store is empty", () => {
    const { container } = render(<ToastContainer />);
    expect(container.querySelectorAll("[data-testid='toast']").length).toBe(0);
  });

  it("renders toasts from the notification store", () => {
    act(() => {
      useNotificationStore.getState().addNotification({
        type: "success",
        title: "Workspace created",
        message: "feat-auth ready",
        autoCloseMs: 0,
      });
    });

    render(<ToastContainer />);
    expect(screen.getByText("Workspace created")).toBeDefined();
    expect(screen.getByText("feat-auth ready")).toBeDefined();
  });

  it("respects maxVisible limit", () => {
    act(() => {
      const store = useNotificationStore.getState();
      for (let i = 0; i < 8; i++) {
        store.addNotification({
          type: "info",
          title: `Notification ${i}`,
          message: `Message ${i}`,
          autoCloseMs: 0,
        });
      }
    });

    const { container } = render(<ToastContainer />);
    const toasts = container.querySelectorAll("[data-testid='toast']");
    expect(toasts.length).toBeLessThanOrEqual(5);
  });
});
```

Run test (should fail):
```bash
cd ~/projects/koompi-orch && pnpm test src/components/notifications/Notifications.test.tsx
```

Expected: fails with "Cannot find module './Toast'"

- [ ] **Step 2: Implement Toast component**

Create `src/components/notifications/Toast.tsx`:
```typescript
import type { Notification } from "../../stores/notificationStore";

interface ToastProps {
  notification: Notification;
  onDismiss: (id: string) => void;
}

const TYPE_STYLES: Record<string, { border: string; icon: string; iconColor: string }> = {
  info: {
    border: "border-blue-500/50",
    icon: "i",
    iconColor: "text-blue-400 bg-blue-500/20",
  },
  success: {
    border: "border-green-500/50",
    icon: "\u2713",
    iconColor: "text-green-400 bg-green-500/20",
  },
  warning: {
    border: "border-yellow-500/50",
    icon: "!",
    iconColor: "text-yellow-400 bg-yellow-500/20",
  },
  error: {
    border: "border-red-500/50",
    icon: "\u2717",
    iconColor: "text-red-400 bg-red-500/20",
  },
};

export function Toast({ notification, onDismiss }: ToastProps) {
  const style = TYPE_STYLES[notification.type] ?? TYPE_STYLES.info;

  return (
    <div
      data-testid="toast"
      className={`flex items-start gap-3 px-4 py-3 bg-gray-800 border ${style.border} rounded-lg shadow-xl min-w-[300px] max-w-[420px] animate-slide-in`}
    >
      {/* Icon */}
      <span
        className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold flex-shrink-0 ${style.iconColor}`}
      >
        {style.icon}
      </span>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium text-gray-200">
          {notification.title}
        </div>
        <div className="text-xs text-gray-400 mt-0.5 leading-relaxed">
          {notification.message}
        </div>
      </div>

      {/* Dismiss */}
      <button
        type="button"
        title="Dismiss"
        onClick={() => onDismiss(notification.id)}
        className="text-gray-600 hover:text-gray-300 text-sm leading-none flex-shrink-0 mt-0.5"
      >
        &times;
      </button>
    </div>
  );
}
```

- [ ] **Step 3: Implement ToastContainer**

Create `src/components/notifications/ToastContainer.tsx`:
```typescript
import { useNotificationStore } from "../../stores/notificationStore";
import { Toast } from "./Toast";

export function ToastContainer() {
  const visibleNotifications = useNotificationStore(
    (s) => s.visibleNotifications
  );
  const dismissNotification = useNotificationStore(
    (s) => s.dismissNotification
  );

  const toasts = visibleNotifications();

  return (
    <div className="fixed bottom-4 right-4 z-[200] flex flex-col-reverse gap-2 pointer-events-none">
      {toasts.map((notification) => (
        <div key={notification.id} className="pointer-events-auto">
          <Toast notification={notification} onDismiss={dismissNotification} />
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 4: Run tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/components/notifications/Notifications.test.tsx
```

Expected: all 7 tests pass (4 Toast + 3 ToastContainer).

- [ ] **Step 5: Commit**

```bash
cd ~/projects/koompi-orch && git add src/components/notifications/ && git commit -m "feat(ui): add Toast and ToastContainer notification components"
```

---

## Chunk 8: Plugin System UI Stubs

### Task 8: Plugin list, enable/disable, manifest viewer

**Files:**
- Create: `~/projects/koompi-orch/src/components/plugins/PluginList.tsx`
- Create: `~/projects/koompi-orch/src/components/plugins/PluginManifest.tsx`
- Create: `~/projects/koompi-orch/src/components/plugins/Plugins.test.tsx`

- [ ] **Step 1: Write failing test for plugin components**

Create `src/components/plugins/Plugins.test.tsx`:
```typescript
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
```

Run test (should fail):
```bash
cd ~/projects/koompi-orch && pnpm test src/components/plugins/Plugins.test.tsx
```

Expected: fails with "Cannot find module './PluginList'"

- [ ] **Step 2: Implement PluginList**

Create `src/components/plugins/PluginList.tsx`:
```typescript
interface Plugin {
  name: string;
  version: string;
  description: string;
  author: string;
  capabilities: string[];
  enabled: boolean;
}

interface PluginListProps {
  plugins: Plugin[];
  onToggle: (name: string, enabled: boolean) => void;
  onSelect: (name: string) => void;
}

const CAPABILITY_COLORS: Record<string, string> = {
  agent_type: "bg-blue-500/20 text-blue-400",
  pipeline_step: "bg-purple-500/20 text-purple-400",
  event_handler: "bg-green-500/20 text-green-400",
};

export function PluginList({ plugins, onToggle, onSelect }: PluginListProps) {
  if (plugins.length === 0) {
    return (
      <div className="text-sm text-gray-500 text-center py-8">
        No plugins installed. Add WASM plugins to{" "}
        <code className="text-xs bg-gray-800 px-1 py-0.5 rounded">
          ~/.koompi-orch/plugins/
        </code>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {plugins.map((plugin) => (
        <div
          key={plugin.name}
          className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg hover:bg-gray-800/80 transition-colors"
        >
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => onSelect(plugin.name)}
                className="text-sm font-medium text-gray-200 hover:text-blue-400 transition-colors"
              >
                {plugin.name}
              </button>
              <span className="text-[10px] text-gray-600">
                v{plugin.version}
              </span>
            </div>
            <div className="text-xs text-gray-500 mt-0.5 truncate">
              {plugin.description}
            </div>
            <div className="flex gap-1 mt-1.5">
              {plugin.capabilities.map((cap) => (
                <span
                  key={cap}
                  className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${
                    CAPABILITY_COLORS[cap] ?? "bg-gray-700 text-gray-400"
                  }`}
                >
                  {cap}
                </span>
              ))}
            </div>
          </div>

          {/* Enable/disable toggle */}
          <button
            type="button"
            role="switch"
            aria-checked={plugin.enabled}
            onClick={() => onToggle(plugin.name, !plugin.enabled)}
            className={`w-10 h-5 rounded-full transition-colors flex-shrink-0 ml-4 ${
              plugin.enabled ? "bg-blue-500" : "bg-gray-600"
            }`}
          >
            <span
              className={`block w-4 h-4 rounded-full bg-white transform transition-transform ${
                plugin.enabled ? "translate-x-5" : "translate-x-0.5"
              }`}
            />
          </button>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Implement PluginManifest**

Create `src/components/plugins/PluginManifest.tsx`:
```typescript
interface ConfigSchemaEntry {
  type: string;
  required: boolean;
  secret: boolean;
}

interface ManifestData {
  name: string;
  version: string;
  description: string;
  author: string;
  capabilities: string[];
  wasmPath: string;
  configSchema: Record<string, ConfigSchemaEntry>;
}

interface PluginManifestProps {
  manifest: ManifestData;
}

export function PluginManifest({ manifest }: PluginManifestProps) {
  const configEntries = Object.entries(manifest.configSchema);

  return (
    <div className="flex flex-col gap-4 p-4 bg-gray-800/50 border border-gray-700 rounded-lg">
      {/* Header */}
      <div>
        <div className="flex items-center gap-3">
          <h3 className="text-lg font-semibold text-gray-100">
            {manifest.name}
          </h3>
          <span className="text-xs text-gray-500 bg-gray-800 px-2 py-0.5 rounded">
            {manifest.version}
          </span>
        </div>
        <p className="text-sm text-gray-400 mt-1">{manifest.description}</p>
      </div>

      {/* Details */}
      <div className="grid grid-cols-2 gap-4 text-sm">
        <div>
          <span className="text-xs text-gray-500 uppercase">Author</span>
          <div className="text-gray-300 mt-0.5">{manifest.author}</div>
        </div>
        <div>
          <span className="text-xs text-gray-500 uppercase">WASM Path</span>
          <div className="text-gray-300 mt-0.5 font-mono text-xs truncate">
            {manifest.wasmPath}
          </div>
        </div>
      </div>

      {/* Capabilities */}
      <div>
        <span className="text-xs text-gray-500 uppercase">Capabilities</span>
        <div className="flex gap-1.5 mt-1">
          {manifest.capabilities.map((cap) => (
            <span
              key={cap}
              className="text-xs px-2 py-1 rounded bg-gray-700 text-gray-300"
            >
              {cap}
            </span>
          ))}
        </div>
      </div>

      {/* Config Schema */}
      {configEntries.length > 0 && (
        <div>
          <span className="text-xs text-gray-500 uppercase">
            Configuration Schema
          </span>
          <div className="mt-2 border border-gray-700 rounded-lg overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-gray-900/50">
                  <th className="text-left px-3 py-2 text-xs font-medium text-gray-500">
                    Key
                  </th>
                  <th className="text-left px-3 py-2 text-xs font-medium text-gray-500">
                    Type
                  </th>
                  <th className="text-left px-3 py-2 text-xs font-medium text-gray-500">
                    Required
                  </th>
                  <th className="text-left px-3 py-2 text-xs font-medium text-gray-500">
                    Flags
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-800">
                {configEntries.map(([key, schema]) => (
                  <tr key={key} className="hover:bg-white/5">
                    <td className="px-3 py-2 font-mono text-gray-300">
                      {key}
                    </td>
                    <td className="px-3 py-2 text-gray-400">{schema.type}</td>
                    <td className="px-3 py-2 text-gray-400">
                      {schema.required ? "yes" : "no"}
                    </td>
                    <td className="px-3 py-2">
                      {schema.secret && (
                        <span className="text-[10px] font-semibold text-yellow-400 bg-yellow-900/30 px-1.5 py-0.5 rounded">
                          secret
                        </span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Run tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/components/plugins/Plugins.test.tsx
```

Expected: all 6 tests pass (4 PluginList + 2 PluginManifest).

- [ ] **Step 5: Commit**

```bash
cd ~/projects/koompi-orch && git add src/components/plugins/ && git commit -m "feat(ui): add PluginList and PluginManifest stub components"
```

---

## Chunk 9: PipelineBuilder UI

### Task 9: Visual pipeline stage editor with drag-and-drop

**Files:**
- Create: `~/projects/koompi-orch/src/components/agent/PipelineBuilder.tsx`
- Create: `~/projects/koompi-orch/src/components/agent/PipelineBuilder.test.tsx`

- [ ] **Step 1: Write failing test for PipelineBuilder**

Create `src/components/agent/PipelineBuilder.test.tsx`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";
import { PipelineBuilder } from "./PipelineBuilder";

// Mock @dnd-kit since we test via user interactions not drag internals
vi.mock("@dnd-kit/core", () => ({
  DndContext: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dnd-context">{children}</div>
  ),
  DragOverlay: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  closestCenter: vi.fn(),
  PointerSensor: vi.fn(),
  useSensor: vi.fn(),
  useSensors: vi.fn().mockReturnValue([]),
}));

vi.mock("@dnd-kit/sortable", () => ({
  SortableContext: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  verticalListSortingStrategy: "vertical",
  useSortable: vi.fn().mockReturnValue({
    attributes: {},
    listeners: {},
    setNodeRef: vi.fn(),
    transform: null,
    transition: null,
    isDragging: false,
  }),
}));

vi.mock("@dnd-kit/utilities", () => ({
  CSS: { Transform: { toString: () => "" } },
}));

describe("PipelineBuilder", () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders pipeline steps", () => {
    render(
      <PipelineBuilder
        steps={[
          { id: "s1", role: "architect", agentType: "claude-code" },
          { id: "s2", role: "implementer", agentType: "claude-code" },
          { id: "s3", role: "reviewer", agentType: "claude-code" },
        ]}
        onStepsChange={vi.fn()}
      />
    );
    expect(screen.getByText("architect")).toBeDefined();
    expect(screen.getByText("implementer")).toBeDefined();
    expect(screen.getByText("reviewer")).toBeDefined();
  });

  it("renders step numbers", () => {
    render(
      <PipelineBuilder
        steps={[
          { id: "s1", role: "architect", agentType: "claude-code" },
          { id: "s2", role: "implementer", agentType: "codex" },
        ]}
        onStepsChange={vi.fn()}
      />
    );
    expect(screen.getByText("1")).toBeDefined();
    expect(screen.getByText("2")).toBeDefined();
  });

  it("renders add step button", () => {
    render(
      <PipelineBuilder steps={[]} onStepsChange={vi.fn()} />
    );
    expect(screen.getByText("Add Step")).toBeDefined();
  });

  it("calls onStepsChange when a step is removed", () => {
    const onChange = vi.fn();
    render(
      <PipelineBuilder
        steps={[
          { id: "s1", role: "architect", agentType: "claude-code" },
          { id: "s2", role: "implementer", agentType: "claude-code" },
        ]}
        onStepsChange={onChange}
      />
    );
    const removeButtons = screen.getAllByTitle("Remove step");
    fireEvent.click(removeButtons[0]);
    expect(onChange).toHaveBeenCalledWith([
      { id: "s2", role: "implementer", agentType: "claude-code" },
    ]);
  });

  it("calls onStepsChange when add step is clicked", () => {
    const onChange = vi.fn();
    render(
      <PipelineBuilder steps={[]} onStepsChange={onChange} />
    );
    fireEvent.click(screen.getByText("Add Step"));
    expect(onChange).toHaveBeenCalledWith([
      expect.objectContaining({
        role: "implementer",
        agentType: "claude-code",
      }),
    ]);
  });

  it("renders agent type for each step", () => {
    render(
      <PipelineBuilder
        steps={[
          { id: "s1", role: "architect", agentType: "codex" },
        ]}
        onStepsChange={vi.fn()}
      />
    );
    expect(screen.getByText("codex")).toBeDefined();
  });

  it("renders handoff type selector", () => {
    render(
      <PipelineBuilder
        steps={[
          { id: "s1", role: "architect", agentType: "claude-code", handoffType: "summary" },
          { id: "s2", role: "implementer", agentType: "claude-code" },
        ]}
        onStepsChange={vi.fn()}
      />
    );
    // Handoff indicator appears between steps
    expect(screen.getByText("summary")).toBeDefined();
  });
});
```

Run test (should fail):
```bash
cd ~/projects/koompi-orch && pnpm test src/components/agent/PipelineBuilder.test.tsx
```

Expected: fails with "Cannot find module './PipelineBuilder'"

- [ ] **Step 2: Implement PipelineBuilder**

Create `src/components/agent/PipelineBuilder.tsx`:
```typescript
import { useCallback } from "react";
import {
  DndContext,
  DragOverlay,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
  useSortable,
  arrayMove,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

export interface PipelineStep {
  id: string;
  role: string;
  agentType: string;
  handoffType?: "summary" | "full_log" | "diff_only";
}

interface PipelineBuilderProps {
  steps: PipelineStep[];
  onStepsChange: (steps: PipelineStep[]) => void;
}

const ROLES = [
  "architect",
  "implementer",
  "reviewer",
  "tester",
  "shipper",
  "fixer",
];

const AGENT_TYPES = ["claude-code", "codex", "gemini-cli", "aider", "custom"];

const HANDOFF_TYPES: PipelineStep["handoffType"][] = [
  "summary",
  "full_log",
  "diff_only",
];

const ROLE_COLORS: Record<string, string> = {
  architect: "border-l-purple-500",
  implementer: "border-l-blue-500",
  reviewer: "border-l-yellow-500",
  tester: "border-l-green-500",
  shipper: "border-l-orange-500",
  fixer: "border-l-red-500",
};

function SortableStep({
  step,
  index,
  onRemove,
  onChange,
  showHandoff,
}: {
  step: PipelineStep;
  index: number;
  onRemove: () => void;
  onChange: (patch: Partial<PipelineStep>) => void;
  showHandoff: boolean;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: step.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.4 : 1,
  };

  return (
    <>
      <div
        ref={setNodeRef}
        style={style}
        className={`flex items-center gap-3 px-4 py-3 bg-gray-800/50 border border-gray-700 ${
          ROLE_COLORS[step.role] ?? "border-l-gray-500"
        } border-l-4 rounded-lg`}
      >
        {/* Drag handle */}
        <div
          {...attributes}
          {...listeners}
          className="cursor-grab text-gray-600 hover:text-gray-400"
        >
          <span className="text-xs select-none">::</span>
        </div>

        {/* Step number */}
        <span className="w-6 h-6 rounded-full bg-gray-700 flex items-center justify-center text-xs font-bold text-gray-300">
          {index + 1}
        </span>

        {/* Role selector */}
        <select
          value={step.role}
          onChange={(e) => onChange({ role: e.target.value })}
          className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
        >
          {ROLES.map((r) => (
            <option key={r} value={r}>
              {r}
            </option>
          ))}
        </select>

        {/* Agent type selector */}
        <select
          value={step.agentType}
          onChange={(e) => onChange({ agentType: e.target.value })}
          className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
        >
          {AGENT_TYPES.map((a) => (
            <option key={a} value={a}>
              {a}
            </option>
          ))}
        </select>

        {/* Remove */}
        <button
          type="button"
          title="Remove step"
          onClick={onRemove}
          className="ml-auto text-gray-600 hover:text-red-400 text-sm"
        >
          &times;
        </button>
      </div>

      {/* Handoff indicator between steps */}
      {showHandoff && (
        <div className="flex items-center gap-2 pl-12 py-1">
          <div className="w-px h-4 bg-gray-700" />
          <select
            value={step.handoffType ?? "summary"}
            onChange={(e) =>
              onChange({
                handoffType: e.target.value as PipelineStep["handoffType"],
              })
            }
            className="bg-gray-900 border border-gray-700 rounded px-2 py-0.5 text-[10px] text-gray-400 focus:outline-none focus:border-blue-500"
          >
            {HANDOFF_TYPES.map((h) => (
              <option key={h} value={h}>
                {h}
              </option>
            ))}
          </select>
          <div className="w-px h-4 bg-gray-700" />
        </div>
      )}
    </>
  );
}

export function PipelineBuilder({
  steps,
  onStepsChange,
}: PipelineBuilderProps) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || active.id === over.id) return;

      const oldIndex = steps.findIndex((s) => s.id === active.id);
      const newIndex = steps.findIndex((s) => s.id === over.id);
      if (oldIndex === -1 || newIndex === -1) return;

      onStepsChange(arrayMove(steps, oldIndex, newIndex));
    },
    [steps, onStepsChange]
  );

  const addStep = useCallback(() => {
    const newStep: PipelineStep = {
      id: `step-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      role: "implementer",
      agentType: "claude-code",
      handoffType: "summary",
    };
    onStepsChange([...steps, newStep]);
  }, [steps, onStepsChange]);

  const removeStep = useCallback(
    (id: string) => {
      onStepsChange(steps.filter((s) => s.id !== id));
    },
    [steps, onStepsChange]
  );

  const updateStep = useCallback(
    (id: string, patch: Partial<PipelineStep>) => {
      onStepsChange(
        steps.map((s) => (s.id === id ? { ...s, ...patch } : s))
      );
    },
    [steps, onStepsChange]
  );

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-sm font-medium text-gray-300">Pipeline Steps</h3>
        <span className="text-xs text-gray-500">
          {steps.length} step{steps.length !== 1 ? "s" : ""}
        </span>
      </div>

      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
      >
        <SortableContext
          items={steps.map((s) => s.id)}
          strategy={verticalListSortingStrategy}
        >
          {steps.map((step, index) => (
            <SortableStep
              key={step.id}
              step={step}
              index={index}
              onRemove={() => removeStep(step.id)}
              onChange={(patch) => updateStep(step.id, patch)}
              showHandoff={index < steps.length - 1}
            />
          ))}
        </SortableContext>
        <DragOverlay />
      </DndContext>

      {/* Add step button */}
      <button
        type="button"
        onClick={addStep}
        className="mt-2 px-4 py-2 text-sm text-gray-400 hover:text-gray-200 border border-dashed border-gray-700 hover:border-gray-500 rounded-lg transition-colors"
      >
        + Add Step
      </button>
    </div>
  );
}
```

- [ ] **Step 3: Run tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/components/agent/PipelineBuilder.test.tsx
```

Expected: all 7 tests pass.

- [ ] **Step 4: Commit**

```bash
cd ~/projects/koompi-orch && git add src/components/agent/PipelineBuilder.tsx src/components/agent/PipelineBuilder.test.tsx && git commit -m "feat(ui): add PipelineBuilder component with drag-and-drop stage editing"
```

---

## Chunk 10: Final Wiring

### Task 10: Route integration, lazy loading, production build verification

**Files:**
- Modify: `~/projects/koompi-orch/src/app/router.tsx`
- Modify: `~/projects/koompi-orch/src/app/App.tsx`
- Create: `~/projects/koompi-orch/src/app/routes.test.tsx`

- [ ] **Step 1: Write failing test for routes**

Create `src/app/routes.test.tsx`:
```typescript
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
```

Run test (should fail):
```bash
cd ~/projects/koompi-orch && pnpm test src/app/routes.test.tsx
```

Expected: fails because `AppRoutes` does not include these routes yet.

- [ ] **Step 2: Update router.tsx with lazy-loaded routes**

Modify `src/app/router.tsx` to add the new routes. The file should contain:
```typescript
import { lazy } from "react";
import { Routes, Route } from "react-router-dom";

// Lazy load heavy components to reduce initial bundle size
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

/**
 * Default dashboard stats for the standalone /dashboard route.
 * In production, these are fetched from the backend on mount.
 */
const defaultDashboardStats = {
  totalWorkspaces: 0,
  activeAgents: 0,
  totalCostUsd: 0,
  totalTokens: 0,
};

export function AppRoutes() {
  return (
    <Routes>
      {/* Main workspace view is handled by the ThreePanel layout at "/" */}
      <Route path="/" element={<div data-testid="main-layout">Main</div>} />

      {/* Dashboard */}
      <Route
        path="/dashboard"
        element={
          <ProjectDashboard
            stats={defaultDashboardStats}
            recentSessions={[]}
          />
        }
      />

      {/* Settings */}
      <Route path="/settings" element={<SettingsPage />} />

      {/* Plugins */}
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

      {/* Terminal (standalone, for debugging) */}
      <Route
        path="/terminal/:sessionId"
        element={<Terminal sessionId="debug" />}
      />

      {/* Editor (standalone, for file viewing) */}
      <Route
        path="/editor"
        element={<CodeEditor filePath="" content="" />}
      />

      {/* Diff viewer (standalone) */}
      <Route
        path="/diff"
        element={<DiffViewer filePath="" original="" modified="" />}
      />
    </Routes>
  );
}
```

- [ ] **Step 3: Update App.tsx to include ToastContainer**

Add the `ToastContainer` to the root `App.tsx`:
```typescript
// At the top of App.tsx, add:
import { ToastContainer } from "../components/notifications/ToastContainer";

// Inside the App component's JSX, add at the end (before closing fragment/div):
// <ToastContainer />
```

This ensures toast notifications render globally above all other content.

- [ ] **Step 4: Run route tests — verify passing**

```bash
cd ~/projects/koompi-orch && pnpm test src/app/routes.test.tsx
```

Expected: all 3 tests pass.

- [ ] **Step 5: Run full test suite**

```bash
cd ~/projects/koompi-orch && pnpm test
```

Expected: all tests across all chunks pass.

- [ ] **Step 6: Run production build**

```bash
cd ~/projects/koompi-orch && pnpm build
```

Expected: build completes without TypeScript errors or warnings. Verify no `[INEFFECTIVE_DYNAMIC_IMPORT]` warnings.

- [ ] **Step 7: Commit**

```bash
cd ~/projects/koompi-orch && git add src/app/router.tsx src/app/App.tsx src/app/routes.test.tsx && git commit -m "feat(ui): wire routes with lazy loading, add ToastContainer, verify production build"
```

---

## Verification Checklist

After completing all tasks:

- [ ] Run `pnpm dev` and verify the app builds and loads without errors
- [ ] Verify Terminal component renders xterm.js terminal with dark theme
- [ ] Verify CodeEditor renders Monaco with syntax highlighting and file path header
- [ ] Verify DiffViewer renders side-by-side diff with Monaco DiffEditor
- [ ] Verify DiffComment renders comment with author, line number, and resolved state
- [ ] Verify TurnDiff renders per-turn file list with status colors
- [ ] Verify MergeActions renders Commit, Push, Merge, and Create PR buttons
- [ ] Verify ProjectDashboard renders stat cards and recent sessions
- [ ] Verify MetricsChart renders line and bar charts via recharts
- [ ] Verify GlobalSearch renders search input with filtered results
- [ ] Verify SettingsPage renders Appearance, General, API Keys, and Agent Templates sections
- [ ] Verify ThemeToggle switches between dark and light mode
- [ ] Verify ApiKeyManager shows configured/not-configured badges
- [ ] Verify AgentTemplates renders template list with edit capability
- [ ] Verify useLiveQuery handles CREATE, UPDATE, DELETE events from SurrealDB
- [ ] Verify ToastContainer renders notification toasts from the store
- [ ] Verify PluginList renders plugins with enable/disable toggles
- [ ] Verify PluginManifest renders manifest details and config schema
- [ ] Verify PipelineBuilder renders draggable steps with role and agent selectors
- [ ] Verify lazy-loaded routes work for /dashboard, /settings, /plugins
- [ ] Run `pnpm build` to confirm no TypeScript errors
- [ ] Run `pnpm test` to confirm all tests pass
