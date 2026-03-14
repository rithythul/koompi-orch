import { useNavigate, useLocation } from "react-router-dom";

interface SidebarProps {
  collapsed: boolean;
}

const navItems = [
  { label: "Workspaces", icon: "📋", path: "/" },
  { label: "Dashboard", icon: "📊", path: "/dashboard" },
  { label: "Pipelines", icon: "🔗", path: "/pipelines" },
  { label: "Templates", icon: "🤖", path: "/templates" },
  { label: "Plugins", icon: "🧩", path: "/plugins" },
  { label: "Settings", icon: "⚙️", path: "/settings" },
];

export function Sidebar({ collapsed }: SidebarProps) {
  const navigate = useNavigate();
  const location = useLocation();

  if (collapsed) return null;

  return (
    <aside className="w-64 bg-secondary border-r border-border flex flex-col h-full">
      <div className="p-4 border-b border-border">
        <h1 className="text-lg font-bold text-text-primary">koompi-orch</h1>
        <p className="text-xs text-text-secondary mt-1">Agent Orchestrator</p>
      </div>

      <nav className="flex-1 p-2 space-y-1">
        {navItems.map((item) => (
          <SidebarItem
            key={item.path}
            label={item.label}
            icon={item.icon}
            active={location.pathname === item.path}
            onClick={() => navigate(item.path)}
          />
        ))}
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
  onClick,
}: {
  label: string;
  icon: string;
  active?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      onClick={onClick}
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
