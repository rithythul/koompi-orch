import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface SearchResult {
  id: string;
  type: "workspace" | "session" | "file";
  title: string;
  subtitle: string;
}

interface GlobalSearchProps {
  onSelect: (result: SearchResult) => void;
  results?: SearchResult[];
}

const TYPE_ICONS: Record<string, string> = {
  workspace: "W",
  session: "S",
  file: "F",
};

const TYPE_COLORS: Record<string, string> = {
  workspace: "bg-blue-500/20 text-blue-400",
  session: "bg-green-500/20 text-green-400",
  file: "bg-gray-500/20 text-gray-400",
};

export function GlobalSearch({ onSelect, results: staticResults }: GlobalSearchProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>(staticResults ?? []);
  const [loading, setLoading] = useState(false);

  const search = useCallback(
    async (q: string) => {
      setQuery(q);
      if (staticResults) {
        setResults(
          staticResults.filter(
            (r) =>
              r.title.toLowerCase().includes(q.toLowerCase()) ||
              r.subtitle.toLowerCase().includes(q.toLowerCase())
          )
        );
        return;
      }
      if (q.length < 2) {
        setResults([]);
        return;
      }
      setLoading(true);
      try {
        const res = await invoke<SearchResult[]>("global_search", {
          query: q,
        });
        setResults(res);
      } catch (err) {
        console.error("Search failed:", err);
        setResults([]);
      } finally {
        setLoading(false);
      }
    },
    [staticResults]
  );

  return (
    <div className="flex flex-col gap-2">
      <input
        type="text"
        value={query}
        onChange={(e) => search(e.target.value)}
        placeholder="Search all workspaces, sessions, files..."
        className="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm text-gray-200 focus:outline-none focus:border-blue-500 placeholder:text-gray-600"
      />

      {loading && (
        <div className="text-xs text-gray-500 px-2">Searching...</div>
      )}

      {results.length > 0 && (
        <div className="border border-gray-700 rounded-lg overflow-hidden bg-gray-800/80 max-h-80 overflow-y-auto">
          {results.map((result) => (
            <button
              key={`${result.type}-${result.id}`}
              type="button"
              onClick={() => onSelect(result)}
              className="w-full text-left flex items-center gap-3 px-3 py-2.5 text-sm hover:bg-white/5 transition-colors border-b border-gray-800 last:border-b-0"
            >
              <span
                className={`w-6 h-6 rounded flex items-center justify-center text-[10px] font-bold ${
                  TYPE_COLORS[result.type] ?? "bg-gray-700 text-gray-400"
                }`}
              >
                {TYPE_ICONS[result.type] ?? "?"}
              </span>
              <div className="flex-1 min-w-0">
                <div className="font-medium text-gray-200 truncate">
                  {result.title}
                </div>
                <div className="text-xs text-gray-500 truncate">
                  {result.subtitle}
                </div>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
