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
