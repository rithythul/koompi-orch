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
    <div className="flex flex-col gap-1">
      {keys.map((entry) => (
        <div
          key={entry.provider}
          className="flex items-center justify-between px-4 py-3 border-b border-border last:border-b-0"
        >
          <div className="flex items-center gap-3">
            <span className="text-[13px] font-medium text-text-primary">
              {entry.label}
            </span>
            {entry.hasKey ? (
              <span className="text-[10px] font-semibold text-success bg-success-muted px-1.5 py-0.5 rounded">
                Configured
              </span>
            ) : (
              <span className="text-[10px] font-semibold text-text-ghost bg-card-bg-hover px-1.5 py-0.5 rounded">
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
                  className="w-48 bg-input-bg border border-border rounded-md px-2.5 py-1.5 text-[12px] text-text-primary placeholder:text-text-ghost focus:outline-none focus:border-accent transition-colors"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleSave(entry.provider);
                    if (e.key === "Escape") {
                      setEditingProvider(null);
                      setKeyValue("");
                    }
                  }}
                  autoFocus
                />
                <button
                  type="button"
                  onClick={() => handleSave(entry.provider)}
                  className="px-2 py-1 text-[12px] text-success hover:text-success/80 transition-colors"
                >
                  Save
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setEditingProvider(null);
                    setKeyValue("");
                  }}
                  className="px-2 py-1 text-[12px] text-text-ghost hover:text-text-secondary transition-colors"
                >
                  Cancel
                </button>
              </>
            ) : (
              <>
                <button
                  type="button"
                  onClick={() => setEditingProvider(entry.provider)}
                  className="px-2 py-1 text-[12px] text-accent hover:text-accent-hover transition-colors"
                >
                  {entry.hasKey ? "Update" : "Add"}
                </button>
                {entry.hasKey && (
                  <button
                    type="button"
                    onClick={() => onDeleteKey(entry.provider)}
                    className="px-2 py-1 text-[12px] text-error hover:text-error/80 transition-colors"
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
