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
