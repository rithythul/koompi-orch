import { useState } from "react";

interface ApiKeyEntry {
  provider: string;
  label: string;
  hasKey: boolean;
}

interface ApiKeyManagerProps {
  keys: ApiKeyEntry[];
  onSaveKey: (provider: string, key: string) => void;
  onDeleteKey: (provider: string) => void;
}

export function ApiKeyManager({
  keys,
  onSaveKey,
  onDeleteKey,
}: ApiKeyManagerProps) {
  const [editingProvider, setEditingProvider] = useState<string | null>(null);
  const [keyValue, setKeyValue] = useState("");

  const handleSave = (provider: string) => {
    if (keyValue.trim()) {
      onSaveKey(provider, keyValue.trim());
      setKeyValue("");
      setEditingProvider(null);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      {keys.map((entry) => (
        <div
          key={entry.provider}
          className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg"
        >
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-gray-200">
              {entry.label}
            </span>
            {entry.hasKey ? (
              <span className="text-[10px] font-semibold text-green-400 bg-green-900/30 px-1.5 py-0.5 rounded">
                Configured
              </span>
            ) : (
              <span className="text-[10px] font-semibold text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">
                Not configured
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {editingProvider === entry.provider ? (
              <>
                <input
                  type="password"
                  value={keyValue}
                  onChange={(e) => setKeyValue(e.target.value)}
                  placeholder="sk-..."
                  className="w-48 bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-gray-200 focus:outline-none focus:border-blue-500"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleSave(entry.provider);
                    if (e.key === "Escape") {
                      setEditingProvider(null);
                      setKeyValue("");
                    }
                  }}
                />
                <button
                  type="button"
                  onClick={() => handleSave(entry.provider)}
                  className="px-2 py-1 text-xs text-green-400 hover:text-green-300"
                >
                  Save
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setEditingProvider(null);
                    setKeyValue("");
                  }}
                  className="px-2 py-1 text-xs text-gray-500 hover:text-gray-300"
                >
                  Cancel
                </button>
              </>
            ) : (
              <>
                <button
                  type="button"
                  onClick={() => setEditingProvider(entry.provider)}
                  className="px-2 py-1 text-xs text-blue-400 hover:text-blue-300"
                >
                  {entry.hasKey ? "Update" : "Add"}
                </button>
                {entry.hasKey && (
                  <button
                    type="button"
                    onClick={() => onDeleteKey(entry.provider)}
                    className="px-2 py-1 text-xs text-red-400 hover:text-red-300"
                  >
                    Delete
                  </button>
                )}
              </>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
