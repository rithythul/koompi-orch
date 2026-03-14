import { DiffEditor } from "@monaco-editor/react";

interface DiffViewerProps {
  filePath: string;
  original: string;
  modified: string;
  language?: string;
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
      <div className="flex items-center px-3 py-1.5 bg-[#252526] border-b border-[#3c3c3c] text-xs">
        <span className="font-mono text-gray-400 truncate">{filePath}</span>
        <span className="ml-auto text-[10px] text-gray-600 uppercase">
          {resolvedLanguage}
        </span>
      </div>

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
