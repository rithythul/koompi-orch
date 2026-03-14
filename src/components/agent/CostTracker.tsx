import { useAgentStore } from "../../stores/agentStore";

interface CostTrackerProps {
  sessionId: string;
}

export function CostTracker({ sessionId }: CostTrackerProps) {
  const session = useAgentStore((s) => s.sessions[sessionId]);

  if (!session) {
    return null;
  }

  const { metrics } = session;

  const totalTokens = metrics.tokensIn + metrics.tokensOut;
  const duration = metrics.durationMs;

  return (
    <div className="flex flex-col gap-2 px-3 py-2">
      <h4 className="text-xs font-semibold uppercase text-gray-400">
        Metrics
      </h4>

      <div className="grid grid-cols-2 gap-2">
        {/* Tokens In */}
        <MetricCard
          label="Tokens In"
          value={formatNumber(metrics.tokensIn)}
          sublabel={`${((metrics.tokensIn / (totalTokens || 1)) * 100).toFixed(0)}%`}
        />

        {/* Tokens Out */}
        <MetricCard
          label="Tokens Out"
          value={formatNumber(metrics.tokensOut)}
          sublabel={`${((metrics.tokensOut / (totalTokens || 1)) * 100).toFixed(0)}%`}
        />

        {/* Cost */}
        <MetricCard
          label="Cost"
          value={`$${metrics.costUsd.toFixed(2)}`}
          sublabel={totalTokens > 0 ? `$${((metrics.costUsd / totalTokens) * 1000).toFixed(3)}/1k tok` : ""}
          highlight={metrics.costUsd > 5}
        />

        {/* Duration */}
        <MetricCard
          label="Duration"
          value={formatDuration(duration)}
          sublabel={totalTokens > 0 ? `${(totalTokens / (duration / 1000)).toFixed(0)} tok/s` : ""}
        />
      </div>

      {/* Token bar */}
      <div className="mt-1">
        <div className="flex items-center justify-between text-[10px] text-gray-500 mb-0.5">
          <span>Total: {formatNumber(totalTokens)} tokens</span>
        </div>
        <div className="h-1.5 bg-gray-800 rounded-full overflow-hidden flex">
          <div
            className="bg-blue-500 h-full"
            style={{
              width: `${totalTokens > 0 ? (metrics.tokensIn / totalTokens) * 100 : 50}%`,
            }}
          />
          <div
            className="bg-green-500 h-full"
            style={{
              width: `${totalTokens > 0 ? (metrics.tokensOut / totalTokens) * 100 : 50}%`,
            }}
          />
        </div>
        <div className="flex items-center gap-3 mt-1 text-[10px] text-gray-600">
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-blue-500" /> Input
          </span>
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-green-500" /> Output
          </span>
        </div>
      </div>
    </div>
  );
}

function MetricCard({
  label,
  value,
  sublabel,
  highlight = false,
}: {
  label: string;
  value: string;
  sublabel?: string;
  highlight?: boolean;
}) {
  return (
    <div className="bg-gray-800/50 rounded-md px-2.5 py-1.5">
      <div className="text-[10px] text-gray-500 uppercase">{label}</div>
      <div
        className={`text-sm font-mono font-medium ${
          highlight ? "text-red-400" : "text-gray-200"
        }`}
      >
        {value}
      </div>
      {sublabel && (
        <div className="text-[10px] text-gray-600">{sublabel}</div>
      )}
    </div>
  );
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

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
