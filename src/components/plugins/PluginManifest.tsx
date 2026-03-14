interface ConfigSchemaEntry {
  type: string;
  required: boolean;
  secret: boolean;
}

interface ManifestData {
  name: string;
  version: string;
  description: string;
  author: string;
  capabilities: string[];
  wasmPath: string;
  configSchema: Record<string, ConfigSchemaEntry>;
}

interface PluginManifestProps {
  manifest: ManifestData;
}

export function PluginManifest({ manifest }: PluginManifestProps) {
  const configEntries = Object.entries(manifest.configSchema);

  return (
    <div className="flex flex-col gap-4 p-4 bg-gray-800/50 border border-gray-700 rounded-lg">
      <div>
        <div className="flex items-center gap-3">
          <h3 className="text-lg font-semibold text-gray-100">
            {manifest.name}
          </h3>
          <span className="text-xs text-gray-500 bg-gray-800 px-2 py-0.5 rounded">
            {manifest.version}
          </span>
        </div>
        <p className="text-sm text-gray-400 mt-1">{manifest.description}</p>
      </div>

      <div className="grid grid-cols-2 gap-4 text-sm">
        <div>
          <span className="text-xs text-gray-500 uppercase">Author</span>
          <div className="text-gray-300 mt-0.5">{manifest.author}</div>
        </div>
        <div>
          <span className="text-xs text-gray-500 uppercase">WASM Path</span>
          <div className="text-gray-300 mt-0.5 font-mono text-xs truncate">
            {manifest.wasmPath}
          </div>
        </div>
      </div>

      <div>
        <span className="text-xs text-gray-500 uppercase">Capabilities</span>
        <div className="flex gap-1.5 mt-1">
          {manifest.capabilities.map((cap) => (
            <span
              key={cap}
              className="text-xs px-2 py-1 rounded bg-gray-700 text-gray-300"
            >
              {cap}
            </span>
          ))}
        </div>
      </div>

      {configEntries.length > 0 && (
        <div>
          <span className="text-xs text-gray-500 uppercase">
            Configuration Schema
          </span>
          <div className="mt-2 border border-gray-700 rounded-lg overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-gray-900/50">
                  <th className="text-left px-3 py-2 text-xs font-medium text-gray-500">
                    Key
                  </th>
                  <th className="text-left px-3 py-2 text-xs font-medium text-gray-500">
                    Type
                  </th>
                  <th className="text-left px-3 py-2 text-xs font-medium text-gray-500">
                    Required
                  </th>
                  <th className="text-left px-3 py-2 text-xs font-medium text-gray-500">
                    Flags
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-800">
                {configEntries.map(([key, schema]) => (
                  <tr key={key} className="hover:bg-white/5">
                    <td className="px-3 py-2 font-mono text-gray-300">
                      {key}
                    </td>
                    <td className="px-3 py-2 text-gray-400">{schema.type}</td>
                    <td className="px-3 py-2 text-gray-400">
                      {schema.required ? "yes" : "no"}
                    </td>
                    <td className="px-3 py-2">
                      {schema.secret && (
                        <span className="text-[10px] font-semibold text-yellow-400 bg-yellow-900/30 px-1.5 py-0.5 rounded">
                          secret
                        </span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
