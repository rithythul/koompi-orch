import { useNavigate, useLocation } from "react-router-dom";

interface SidebarProps {
  collapsed: boolean;
}

const navItems = [
  { label: "Workspaces", icon: WorkspaceIcon, path: "/" },
  { label: "Dashboard", icon: DashboardIcon, path: "/dashboard" },
  { label: "Pipelines", icon: PipelineIcon, path: "/pipelines" },
  { label: "Templates", icon: TemplateIcon, path: "/templates" },
  { label: "Plugins", icon: PluginIcon, path: "/plugins" },
  { label: "Settings", icon: SettingsIcon, path: "/settings" },
];

export function Sidebar({ collapsed }: SidebarProps) {
  const navigate = useNavigate();
  const location = useLocation();

  if (collapsed) return null;

  return (
    <aside className="w-[240px] bg-secondary/80 backdrop-blur-xl border-r border-border flex flex-col h-full relative">
      {/* Brand */}
      <div className="px-5 pt-5 pb-4">
        <div className="flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-lg bg-accent flex items-center justify-center">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path d="M2 4L8 1L14 4V12L8 15L2 12V4Z" stroke="white" strokeWidth="1.5" fill="none"/>
              <path d="M8 1V15M2 4L14 12M14 4L2 12" stroke="white" strokeWidth="1" opacity="0.4"/>
            </svg>
          </div>
          <div>
            <h1 className="text-[13px] font-semibold text-text-primary tracking-tight">koompi-orch</h1>
            <p className="text-[10px] text-text-tertiary font-mono uppercase tracking-wider">orchestrator</p>
          </div>
        </div>
      </div>

      {/* Search hint */}
      <div className="px-3 mb-3">
        <button
          onClick={() => {/* command palette */}}
          className="w-full flex items-center gap-2 px-3 py-2 rounded-lg bg-input-bg border border-border hover:border-border-strong text-text-tertiary text-xs transition-all duration-150"
        >
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
            <circle cx="7" cy="7" r="5.5"/>
            <path d="M11 11L14.5 14.5"/>
          </svg>
          <span className="flex-1 text-left">Search</span>
          <kbd className="text-[10px] text-text-ghost font-mono bg-card-bg-hover px-1.5 py-0.5 rounded border border-border">
            /
          </kbd>
        </button>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 space-y-0.5">
        <div className="text-[10px] font-mono uppercase tracking-widest text-text-ghost px-3 mb-2">Navigation</div>
        {navItems.map((item) => {
          const isActive = location.pathname === item.path;
          const Icon = item.icon;
          return (
            <button
              key={item.path}
              onClick={() => navigate(item.path)}
              className={`w-full text-left px-3 py-[7px] rounded-md text-[13px] flex items-center gap-2.5 transition-all duration-150 group ${
                isActive
                  ? "bg-accent-muted text-accent-hover font-medium"
                  : "text-text-secondary hover:bg-card-bg-hover hover:text-text-primary"
              }`}
            >
              <Icon active={isActive} />
              <span>{item.label}</span>
              {isActive && (
                <div className="ml-auto w-1.5 h-1.5 rounded-full bg-accent" />
              )}
            </button>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="px-5 py-4 border-t border-border">
        <div className="flex items-center justify-between">
          <span className="text-[10px] font-mono text-text-ghost">v0.1.0</span>
          <span className="text-[10px] font-mono text-text-ghost">dev</span>
        </div>
      </div>
    </aside>
  );
}

/* — Inline SVG icons — crisp at 15px, no emoji fallback — */

function WorkspaceIcon({ active }: { active: boolean }) {
  return (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4"
      className={active ? "text-accent" : "text-text-tertiary group-hover:text-text-secondary"}>
      <rect x="1.5" y="1.5" width="5" height="5" rx="1"/>
      <rect x="9.5" y="1.5" width="5" height="5" rx="1"/>
      <rect x="1.5" y="9.5" width="5" height="5" rx="1"/>
      <rect x="9.5" y="9.5" width="5" height="5" rx="1"/>
    </svg>
  );
}

function DashboardIcon({ active }: { active: boolean }) {
  return (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4"
      className={active ? "text-accent" : "text-text-tertiary group-hover:text-text-secondary"}>
      <rect x="1.5" y="1.5" width="13" height="13" rx="2"/>
      <path d="M4 11V8M8 11V5M12 11V7"/>
    </svg>
  );
}

function PipelineIcon({ active }: { active: boolean }) {
  return (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4"
      className={active ? "text-accent" : "text-text-tertiary group-hover:text-text-secondary"}>
      <circle cx="3" cy="8" r="2"/>
      <circle cx="13" cy="4" r="2"/>
      <circle cx="13" cy="12" r="2"/>
      <path d="M5 7.5L11 4.5M5 8.5L11 11.5"/>
    </svg>
  );
}

function TemplateIcon({ active }: { active: boolean }) {
  return (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4"
      className={active ? "text-accent" : "text-text-tertiary group-hover:text-text-secondary"}>
      <rect x="2" y="1.5" width="12" height="13" rx="2"/>
      <path d="M5 5H11M5 8H9M5 11H7"/>
    </svg>
  );
}

function PluginIcon({ active }: { active: boolean }) {
  return (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4"
      className={active ? "text-accent" : "text-text-tertiary group-hover:text-text-secondary"}>
      <rect x="3" y="5" width="10" height="9" rx="2"/>
      <path d="M6 5V3a2 2 0 1 1 4 0v2"/>
      <circle cx="8" cy="10" r="1.5"/>
    </svg>
  );
}

function SettingsIcon({ active }: { active: boolean }) {
  return (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4"
      className={active ? "text-accent" : "text-text-tertiary group-hover:text-text-secondary"}>
      <circle cx="8" cy="8" r="2.5"/>
      <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.3 3.3l1.4 1.4M11.3 11.3l1.4 1.4M3.3 12.7l1.4-1.4M11.3 4.7l1.4-1.4"/>
    </svg>
  );
}
