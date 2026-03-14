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

const STATUS_CONFIG: Record<string, { color: string; dot: string; bg: string }> = {
  running: { color: "text-info", dot: "bg-info", bg: "bg-info-muted" },
  paused: { color: "text-warning", dot: "bg-warning", bg: "bg-warning-muted" },
  completed: { color: "text-success", dot: "bg-success", bg: "bg-success-muted" },
  crashed: { color: "text-error", dot: "bg-error", bg: "bg-error-muted" },
};

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function StatCard({
  label,
  value,
  accent,
  icon,
}: {
  label: string;
  value: string;
  accent: string;
  icon: React.ReactNode;
}) {
  return (
    <div className="card-glass rounded-lg p-4 flex flex-col gap-3 transition-all duration-150">
      <div className="flex items-center justify-between">
        <span className="text-[11px] font-medium text-text-tertiary uppercase tracking-wider">{label}</span>
        <div className={`w-7 h-7 rounded-md ${accent} flex items-center justify-center`}>
          {icon}
        </div>
      </div>
      <div className="text-2xl font-semibold text-text-primary tracking-tight">{value}</div>
    </div>
  );
}

export function ProjectDashboard({
  stats,
  recentSessions,
}: ProjectDashboardProps) {
  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="h-[48px] px-6 flex items-center justify-between border-b border-border shrink-0">
        <div className="flex items-center gap-3">
          <h2 className="text-[13px] font-semibold text-text-primary">Dashboard</h2>
          <span className="text-text-ghost">·</span>
          <span className="text-[11px] text-text-tertiary">Overview</span>
        </div>
        <span className="text-[10px] font-mono text-text-ghost">
          {new Date().toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
        </span>
      </div>

      <div className="flex-1 overflow-auto p-6">
        {/* Stats grid */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 stagger-children">
          <StatCard
            label="Workspaces"
            value={String(stats.totalWorkspaces)}
            accent="bg-card-bg-hover"
            icon={<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" className="text-text-tertiary"><rect x="1.5" y="1.5" width="5" height="5" rx="1"/><rect x="9.5" y="1.5" width="5" height="5" rx="1"/><rect x="1.5" y="9.5" width="5" height="5" rx="1"/><rect x="9.5" y="9.5" width="5" height="5" rx="1"/></svg>}
          />
          <StatCard
            label="Active Agents"
            value={String(stats.activeAgents)}
            accent="bg-info-muted"
            icon={<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" className="text-info"><circle cx="8" cy="6" r="3"/><path d="M3 14c0-2.8 2.2-5 5-5s5 2.2 5 5"/></svg>}
          />
          <StatCard
            label="Total Cost"
            value={`$${stats.totalCostUsd.toFixed(2)}`}
            accent="bg-warning-muted"
            icon={<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" className="text-warning"><circle cx="8" cy="8" r="6.5"/><path d="M8 4V12M6 6c0-.8.9-1.5 2-1.5s2 .7 2 1.5-.9 1.5-2 1.5-2 .7-2 1.5.9 1.5 2 1.5"/></svg>}
          />
          <StatCard
            label="Tokens Used"
            value={formatTokens(stats.totalTokens)}
            accent="bg-accent-muted"
            icon={<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" className="text-accent"><path d="M2 4L8 1L14 4V12L8 15L2 12V4Z"/><path d="M8 7V10"/><circle cx="8" cy="12" r="0.5" fill="currentColor"/></svg>}
          />
        </div>

        {/* Sessions */}
        <div className="mt-6 card-glass rounded-lg overflow-hidden animate-in" style={{ animationDelay: '0.15s' }}>
          <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
            <h3 className="text-[12px] font-semibold text-text-secondary uppercase tracking-wider">
              Recent Sessions
            </h3>
            <span className="text-[10px] font-mono text-text-ghost">{recentSessions.length} sessions</span>
          </div>
          {recentSessions.length === 0 ? (
            <div className="px-5 py-10 text-[13px] text-text-ghost text-center">
              No sessions yet. Create a workspace and start an agent.
            </div>
          ) : (
            <div className="divide-y divide-border">
              {recentSessions.map((session) => {
                const cfg = STATUS_CONFIG[session.status] ?? { color: "text-text-ghost", dot: "bg-text-ghost", bg: "bg-input-bg" };
                return (
                  <div
                    key={session.id}
                    className="flex items-center justify-between px-5 py-3.5 hover:bg-card-bg transition-colors duration-100 cursor-pointer group"
                  >
                    <div className="flex items-center gap-3.5">
                      <span className={`w-2 h-2 rounded-full ${cfg.dot}`} />
                      <div>
                        <div className="text-[13px] font-medium text-text-primary group-hover:text-accent-hover transition-colors">
                          {session.workspaceName}
                        </div>
                        <div className="text-[11px] text-text-ghost font-mono mt-0.5">
                          {session.agentType}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-4">
                      <span className={`text-[10px] font-mono uppercase tracking-wider px-1.5 py-0.5 rounded ${cfg.bg} ${cfg.color}`}>
                        {session.status}
                      </span>
                      <div className="text-right">
                        <div className="text-[12px] font-mono text-text-secondary">
                          ${session.costUsd.toFixed(2)}
                        </div>
                        <div className="text-[10px] text-text-ghost font-mono">
                          {new Date(session.startedAt).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })}
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
