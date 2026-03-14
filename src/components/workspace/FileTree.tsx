import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Status from git: modified, added, deleted, renamed, untracked */
type GitFileStatus = "M" | "A" | "D" | "R" | "?" | null;

interface FileNode {
  name: string;
  path: string;
  isDir: boolean;
  children?: FileNode[];
  gitStatus: GitFileStatus;
}

const GIT_STATUS_COLORS: Record<string, string> = {
  M: "text-yellow-400",
  A: "text-green-400",
  D: "text-red-400",
  R: "text-blue-400",
  "?": "text-gray-500",
};

interface FileTreeProps {
  workspaceId: string;
  worktreePath: string;
  onFileSelect?: (filePath: string) => void;
}

function FileTreeNode({
  node,
  depth,
  onFileSelect,
}: {
  node: FileNode;
  depth: number;
  onFileSelect?: (filePath: string) => void;
}) {
  const [expanded, setExpanded] = useState(depth < 1);

  const handleClick = () => {
    if (node.isDir) {
      setExpanded(!expanded);
    } else {
      onFileSelect?.(node.path);
    }
  };

  return (
    <div>
      <button
        type="button"
        onClick={handleClick}
        className="w-full text-left flex items-center gap-1 px-1 py-0.5 text-sm hover:bg-white/5 rounded"
        style={{ paddingLeft: `${depth * 16 + 4}px` }}
      >
        {/* Expand/collapse icon for directories */}
        {node.isDir ? (
          <span className="w-4 text-gray-500 text-xs">
            {expanded ? "\u25BE" : "\u25B8"}
          </span>
        ) : (
          <span className="w-4" />
        )}

        {/* File/folder icon */}
        <span className="text-xs">
          {node.isDir ? (expanded ? "\uD83D\uDCC2" : "\uD83D\uDCC1") : "\uD83D\uDCC4"}
        </span>

        {/* Name */}
        <span
          className={`truncate ${
            node.gitStatus
              ? GIT_STATUS_COLORS[node.gitStatus] ?? "text-gray-300"
              : "text-gray-300"
          }`}
        >
          {node.name}
        </span>

        {/* Git status badge */}
        {node.gitStatus && (
          <span
            className={`ml-auto text-[10px] font-mono ${
              GIT_STATUS_COLORS[node.gitStatus] ?? "text-gray-500"
            }`}
          >
            {node.gitStatus}
          </span>
        )}
      </button>

      {node.isDir && expanded && node.children && (
        <div>
          {node.children.map((child) => (
            <FileTreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              onFileSelect={onFileSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function FileTree({
  workspaceId,
  worktreePath,
  onFileSelect,
}: FileTreeProps) {
  const [tree, setTree] = useState<FileNode[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadTree = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<FileNode[]>("list_workspace_files", {
        workspaceId,
        worktreePath,
      });
      setTree(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [workspaceId, worktreePath]);

  useEffect(() => {
    loadTree();
  }, [loadTree]);

  if (loading) {
    return (
      <div className="text-sm text-gray-500 p-2">Loading files...</div>
    );
  }

  if (error) {
    return (
      <div className="text-sm text-red-400 p-2">
        Error: {error}
        <button
          type="button"
          onClick={loadTree}
          className="ml-2 text-blue-400 hover:underline"
        >
          Retry
        </button>
      </div>
    );
  }

  if (tree.length === 0) {
    return (
      <div className="text-sm text-gray-500 p-2">No files in workspace</div>
    );
  }

  return (
    <div className="text-sm">
      <div className="flex items-center justify-between px-2 py-1 mb-1">
        <span className="text-xs font-semibold uppercase text-gray-400">
          Files
        </span>
        <button
          type="button"
          onClick={loadTree}
          className="text-xs text-gray-500 hover:text-gray-300"
          title="Refresh"
        >
          &#x21BB;
        </button>
      </div>
      {tree.map((node) => (
        <FileTreeNode
          key={node.path}
          node={node}
          depth={0}
          onFileSelect={onFileSelect}
        />
      ))}
    </div>
  );
}
