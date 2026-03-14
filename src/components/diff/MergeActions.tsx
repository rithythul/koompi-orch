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
      <div className="flex items-center gap-2 text-xs text-gray-400">
        <span className="w-2 h-2 rounded-full bg-blue-500" />
        <span className="font-mono">{branch}</span>
      </div>

      <input
        type="text"
        value={commitMessage}
        onChange={(e) => setCommitMessage(e.target.value)}
        placeholder="Commit message (optional)"
        className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-1.5 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
      />

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
