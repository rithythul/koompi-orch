export function CenterPanel() {
  return (
    <main className="flex-1 bg-primary flex flex-col h-full overflow-hidden">
      <div className="border-b border-border p-3 flex items-center justify-between">
        <h2 className="text-sm font-medium text-text-primary">Workspaces</h2>
        <button className="px-3 py-1 text-xs bg-accent hover:bg-accent-hover text-white rounded transition-colors">
          + New Workspace
        </button>
      </div>

      <div className="flex-1 p-4 overflow-auto">
        <div className="grid grid-cols-4 gap-3">
          {(["backlog", "active", "review", "done"] as const).map((status) => (
            <KanbanColumn key={status} status={status} />
          ))}
        </div>
      </div>
    </main>
  );
}

function KanbanColumn({ status }: { status: string }) {
  const colors: Record<string, string> = {
    backlog: "text-text-secondary",
    active: "text-accent",
    review: "text-warning",
    done: "text-success",
  };

  return (
    <div className="bg-secondary rounded-lg p-3 min-h-[200px]">
      <h3 className={`text-xs font-semibold uppercase mb-3 ${colors[status] ?? "text-text-secondary"}`}>
        {status}
      </h3>
      <p className="text-xs text-text-secondary italic">No workspaces</p>
    </div>
  );
}
