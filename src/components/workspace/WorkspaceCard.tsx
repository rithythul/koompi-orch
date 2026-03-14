import { type Workspace, useWorkspaceStore } from "../../stores/workspaceStore";
import { useAgentStore } from "../../stores/agentStore";

const STATUS_COLORS: Record<string, string> = {
  backlog: "bg-gray-500",
  active: "bg-blue-500",
  review: "bg-yellow-500",
  done: "bg-green-500",
  failed: "bg-red-500",
};

const STATUS_LABELS: Record<string, string> = {
  backlog: "Backlog",
  active: "Active",
  review: "Review",
  done: "Done",
  failed: "Failed",
};

interface WorkspaceCardProps {
  workspace: Workspace;
}

export function WorkspaceCard({ workspace }: WorkspaceCardProps) {
  const selectWorkspace = useWorkspaceStore((s) => s.selectWorkspace);
  const selectedId = useWorkspaceStore((s) => s.selectedWorkspaceId);
  const sessionForWorkspace = useAgentStore((s) => s.sessionForWorkspace);

  const isSelected = selectedId === workspace.id;
  const session = sessionForWorkspace(workspace.id);

  return (
    <button
      type="button"
      onClick={() => selectWorkspace(workspace.id)}
      className={`
        w-full text-left px-3 py-2 rounded-lg border transition-colors
        ${isSelected ? "border-blue-500 bg-blue-500/10" : "border-transparent hover:bg-white/5"}
      `}
    >
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-gray-200 truncate">
          {workspace.name}
        </span>
        <div className="flex items-center gap-1.5">
          {workspace.hasConflict && (
            <span
              className="w-2 h-2 rounded-full bg-orange-500"
              title="File conflict detected"
            />
          )}
          {session && (
            <span
              className="w-2 h-2 rounded-full bg-blue-400 animate-pulse"
              title={`Agent running (${session.agentType})`}
            />
          )}
          <span
            className={`px-1.5 py-0.5 text-[10px] font-semibold uppercase rounded ${STATUS_COLORS[workspace.status]} text-white`}
          >
            {STATUS_LABELS[workspace.status]}
          </span>
        </div>
      </div>
      <div className="flex items-center gap-2 mt-1">
        <span className="text-xs text-gray-500 truncate">
          {workspace.repoName}
        </span>
        <span className="text-xs text-gray-600">/</span>
        <span className="text-xs text-gray-400 truncate font-mono">
          {workspace.branch}
        </span>
      </div>
    </button>
  );
}
