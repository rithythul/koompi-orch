interface SidebarProps {
  collapsed: boolean;
}

export function Sidebar({ collapsed }: SidebarProps) {
  if (collapsed) return null;

  return (
    <aside className="w-64 bg-secondary border-r border-border flex flex-col h-full">
      <div className="p-4 border-b border-border">
        <h1 className="text-lg font-bold text-text-primary">koompi-orch</h1>
        <p className="text-xs text-text-secondary mt-1">Agent Orchestrator</p>
      </div>

      <nav className="flex-1 p-2 space-y-1">
        <SidebarItem label="Workspaces" icon="📋" active />
        <SidebarItem label="Pipelines" icon="🔗" />
        <SidebarItem label="Templates" icon="🤖" />
        <SidebarItem label="Settings" icon="⚙️" />
      </nav>

      <div className="p-3 border-t border-border text-xs text-text-secondary">
        v0.1.0
      </div>
    </aside>
  );
}

function SidebarItem({
  label,
  icon,
  active = false,
}: {
  label: string;
  icon: string;
  active?: boolean;
}) {
  return (
    <button
      className={`w-full text-left px-3 py-2 rounded text-sm flex items-center gap-2 transition-colors ${
        active
          ? "bg-accent/20 text-accent"
          : "text-text-secondary hover:bg-tertiary hover:text-text-primary"
      }`}
    >
      <span>{icon}</span>
      <span>{label}</span>
    </button>
  );
}
