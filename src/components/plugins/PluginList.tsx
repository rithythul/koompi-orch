interface Plugin {
  name: string;
  version: string;
  description: string;
  author: string;
  capabilities: string[];
  enabled: boolean;
}

interface PluginListProps {
  plugins: Plugin[];
  onToggle: (name: string, enabled: boolean) => void;
  onSelect: (name: string) => void;
}

const CAPABILITY_COLORS: Record<string, string> = {
  git: "bg-accent-muted text-accent",
  pr: "bg-accent-muted text-accent",
  issues: "bg-accent-muted text-accent",
  notify: "bg-warning-muted text-warning",
  webhook: "bg-warning-muted text-warning",
  container: "bg-success-muted text-success",
  build: "bg-success-muted text-success",
  agent_type: "bg-info-muted text-info",
  pipeline_step: "bg-accent-muted text-accent",
  event_handler: "bg-success-muted text-success",
};

export function PluginList({ plugins, onToggle, onSelect }: PluginListProps) {
  if (plugins.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16">
        <div className="w-12 h-12 rounded-xl bg-[rgba(255,255,255,0.03)] border border-border flex items-center justify-center mb-4">
          <svg width="20" height="20" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2" className="text-text-ghost">
            <rect x="3" y="5" width="10" height="9" rx="2"/>
            <path d="M6 5V3a2 2 0 1 1 4 0v2"/>
          </svg>
        </div>
        <p className="text-[13px] text-text-tertiary">No plugins installed</p>
        <p className="text-[11px] text-text-ghost mt-1">
          Add WASM plugins to <code className="font-mono text-[10px] bg-[rgba(255,255,255,0.04)] px-1.5 py-0.5 rounded border border-border">~/.koompi-orch/plugins/</code>
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2 stagger-children">
      {plugins.map((plugin) => (
        <div
          key={plugin.name}
          className="card-glass rounded-lg px-4 py-3.5 flex items-center gap-4 transition-all duration-150 group"
        >
          {/* Icon */}
          <div className={`w-9 h-9 rounded-lg flex items-center justify-center text-[13px] font-bold font-mono shrink-0 ${
            plugin.enabled ? "bg-accent-muted text-accent" : "bg-[rgba(255,255,255,0.04)] text-text-ghost"
          }`}>
            {plugin.name.charAt(0)}
          </div>

          {/* Info */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => onSelect(plugin.name)}
                className="text-[13px] font-medium text-text-primary hover:text-accent-hover transition-colors"
              >
                {plugin.name}
              </button>
              <span className="text-[10px] font-mono text-text-ghost">
                v{plugin.version}
              </span>
            </div>
            {plugin.description && (
              <p className="text-[11px] text-text-ghost mt-0.5 truncate">{plugin.description}</p>
            )}
            <div className="flex gap-1.5 mt-2">
              {plugin.capabilities.map((cap) => (
                <span
                  key={cap}
                  className={`text-[9px] font-mono uppercase tracking-wider px-1.5 py-0.5 rounded ${
                    CAPABILITY_COLORS[cap] ?? "bg-[rgba(255,255,255,0.04)] text-text-ghost"
                  }`}
                >
                  {cap}
                </span>
              ))}
            </div>
          </div>

          {/* Toggle */}
          <button
            type="button"
            role="switch"
            aria-checked={plugin.enabled}
            onClick={() => onToggle(plugin.name, !plugin.enabled)}
            className={`relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0 ${
              plugin.enabled ? "bg-accent" : "bg-[rgba(255,255,255,0.1)]"
            }`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow-sm transform transition-transform duration-200 ${
                plugin.enabled ? "translate-x-4" : "translate-x-0"
              }`}
            />
          </button>
        </div>
      ))}
    </div>
  );
}
