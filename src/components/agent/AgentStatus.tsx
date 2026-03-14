import { invoke } from "@tauri-apps/api/core";
import { useAgentStore, type AgentSessionStatus } from "../../stores/agentStore";

const STATUS_CONFIG: Record<
  AgentSessionStatus,
  { color: string; bgColor: string; label: string; animate: boolean }
> = {
  running: {
    color: "text-green-400",
    bgColor: "bg-green-500/15",
    label: "Running",
    animate: true,
  },
  paused: {
    color: "text-yellow-400",
    bgColor: "bg-yellow-500/15",
    label: "Paused",
    animate: false,
  },
  completed: {
    color: "text-gray-400",
    bgColor: "bg-gray-500/15",
    label: "Completed",
    animate: false,
  },
  crashed: {
    color: "text-red-400",
    bgColor: "bg-red-500/15",
    label: "Crashed",
    animate: false,
  },
};

interface AgentStatusProps {
  sessionId: string;
  /** Show action buttons (pause/resume/kill) */
  showActions?: boolean;
}

export function AgentStatus({
  sessionId,
  showActions = true,
}: AgentStatusProps) {
  const session = useAgentStore((s) => s.sessions[sessionId]);
  const updateStatus = useAgentStore((s) => s.updateSessionStatus);

  if (!session) {
    return (
      <div className="text-xs text-gray-600">No session</div>
    );
  }

  const config = STATUS_CONFIG[session.status];

  const handlePause = async () => {
    try {
      await invoke("pause_agent", { sessionId });
      updateStatus(sessionId, "paused");
    } catch (err) {
      console.error("Failed to pause agent:", err);
    }
  };

  const handleResume = async () => {
    try {
      await invoke("resume_agent", { sessionId });
      updateStatus(sessionId, "running");
    } catch (err) {
      console.error("Failed to resume agent:", err);
    }
  };

  const handleKill = async () => {
    try {
      await invoke("kill_agent", { sessionId });
      updateStatus(sessionId, "crashed");
    } catch (err) {
      console.error("Failed to kill agent:", err);
    }
  };

  const elapsed = session.startedAt
    ? formatDuration(Date.now() - new Date(session.startedAt).getTime())
    : "--";

  return (
    <div className={`flex items-center gap-3 px-3 py-2 rounded-md ${config.bgColor}`}>
      {/* Status indicator */}
      <div className="flex items-center gap-2">
        <span
          className={`w-2 h-2 rounded-full ${config.color.replace("text-", "bg-")} ${
            config.animate ? "animate-pulse" : ""
          }`}
        />
        <span className={`text-sm font-medium ${config.color}`}>
          {config.label}
        </span>
      </div>

      {/* Agent info */}
      <span className="text-xs text-gray-500">
        {session.agentType}
        {session.model ? ` / ${session.model}` : ""}
      </span>

      {/* Duration */}
      <span className="text-xs text-gray-600">{elapsed}</span>

      {/* PID */}
      {session.pid && (
        <span className="text-[10px] text-gray-700 font-mono">
          PID {session.pid}
        </span>
      )}

      {/* Action buttons */}
      {showActions && (
        <div className="flex items-center gap-1 ml-auto">
          {session.status === "running" && (
            <>
              <button
                type="button"
                onClick={handlePause}
                className="px-2 py-0.5 text-xs text-yellow-400 hover:bg-yellow-500/20 rounded"
                title="Pause agent (SIGSTOP)"
              >
                Pause
              </button>
              <button
                type="button"
                onClick={handleKill}
                className="px-2 py-0.5 text-xs text-red-400 hover:bg-red-500/20 rounded"
                title="Kill agent (SIGKILL)"
              >
                Kill
              </button>
            </>
          )}
          {session.status === "paused" && (
            <>
              <button
                type="button"
                onClick={handleResume}
                className="px-2 py-0.5 text-xs text-green-400 hover:bg-green-500/20 rounded"
                title="Resume agent (SIGCONT)"
              >
                Resume
              </button>
              <button
                type="button"
                onClick={handleKill}
                className="px-2 py-0.5 text-xs text-red-400 hover:bg-red-500/20 rounded"
                title="Kill agent"
              >
                Kill
              </button>
            </>
          )}
          {session.status === "crashed" && (
            <button
              type="button"
              onClick={handleResume}
              className="px-2 py-0.5 text-xs text-blue-400 hover:bg-blue-500/20 rounded"
              title="Retry agent"
            >
              Retry
            </button>
          )}
        </div>
      )}
    </div>
  );
}

/** Format milliseconds to human-readable duration */
function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) return `${minutes}m ${remainingSeconds}s`;
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return `${hours}h ${remainingMinutes}m`;
}
