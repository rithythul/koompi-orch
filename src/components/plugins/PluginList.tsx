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
  agent_type: "bg-blue-500/20 text-blue-400",
  pipeline_step: "bg-purple-500/20 text-purple-400",
  event_handler: "bg-green-500/20 text-green-400",
};

export function PluginList({ plugins, onToggle, onSelect }: PluginListProps) {
  if (plugins.length === 0) {
    return (
      <div className="text-sm text-gray-500 text-center py-8">
        No plugins installed. Add WASM plugins to{" "}
        <code className="text-xs bg-gray-800 px-1 py-0.5 rounded">
          ~/.koompi-orch/plugins/
        </code>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {plugins.map((plugin) => (
        <div
          key={plugin.name}
          className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg hover:bg-gray-800/80 transition-colors"
        >
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => onSelect(plugin.name)}
                className="text-sm font-medium text-gray-200 hover:text-blue-400 transition-colors"
              >
                {plugin.name}
              </button>
              <span className="text-[10px] text-gray-600">
                v{plugin.version}
              </span>
            </div>
            <div className="text-xs text-gray-500 mt-0.5 truncate">
              {plugin.description}
            </div>
            <div className="flex gap-1 mt-1.5">
              {plugin.capabilities.map((cap) => (
                <span
                  key={cap}
                  className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${
                    CAPABILITY_COLORS[cap] ?? "bg-gray-700 text-gray-400"
                  }`}
                >
                  {cap}
                </span>
              ))}
            </div>
          </div>

          <button
            type="button"
            role="switch"
            aria-checked={plugin.enabled}
            onClick={() => onToggle(plugin.name, !plugin.enabled)}
            className={`w-10 h-5 rounded-full transition-colors flex-shrink-0 ml-4 ${
              plugin.enabled ? "bg-blue-500" : "bg-gray-600"
            }`}
          >
            <span
              className={`block w-4 h-4 rounded-full bg-white transform transition-transform ${
                plugin.enabled ? "translate-x-5" : "translate-x-0.5"
              }`}
            />
          </button>
        </div>
      ))}
    </div>
  );
}
