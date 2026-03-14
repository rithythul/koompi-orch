import MonacoEditor from "@monaco-editor/react";

interface CodeEditorProps {
  filePath: string;
  content: string;
  language?: string;
  readOnly?: boolean;
  height?: string;
  theme?: "vs-dark" | "light";
  onChange?: (value: string | undefined) => void;
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
      <div className="flex items-center px-3 py-1.5 bg-[#252526] border-b border-[#3c3c3c] text-xs text-gray-400">
        <span className="font-mono truncate">{filePath}</span>
        <span className="ml-auto text-[10px] text-gray-600 uppercase">
          {resolvedLanguage}
        </span>
      </div>

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
