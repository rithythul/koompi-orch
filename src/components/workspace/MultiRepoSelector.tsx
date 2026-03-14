import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Repo {
  id: string;
  name: string;
  path: string;
  remoteUrl: string | null;
}

interface MultiRepoSelectorProps {
  /** Currently selected repo IDs */
  selectedIds: string[];
  /** Callback when selection changes */
  onChange: (selectedIds: string[]) => void;
  /** Allow multiple selection (default: false) */
  multiple?: boolean;
}

export function MultiRepoSelector({
  selectedIds,
  onChange,
  multiple = false,
}: MultiRepoSelectorProps) {
  const [repos, setRepos] = useState<Repo[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [addingPath, setAddingPath] = useState("");

  useEffect(() => {
    invoke<Repo[]>("list_repos")
      .then(setRepos)
      .catch((err: unknown) => console.error("Failed to load repos:", err))
      .finally(() => setLoading(false));
  }, []);

  const filtered = repos.filter(
    (r) =>
      r.name.toLowerCase().includes(search.toLowerCase()) ||
      r.path.toLowerCase().includes(search.toLowerCase())
  );

  const toggleRepo = (id: string) => {
    if (multiple) {
      if (selectedIds.includes(id)) {
        onChange(selectedIds.filter((s) => s !== id));
      } else {
        onChange([...selectedIds, id]);
      }
    } else {
      onChange(selectedIds.includes(id) ? [] : [id]);
    }
  };

  const addRepo = async () => {
    if (!addingPath.trim()) return;
    try {
      const repo = await invoke<Repo>("add_repo", { path: addingPath.trim() });
      setRepos((prev) => [...prev, repo]);
      setAddingPath("");
      onChange([...selectedIds, repo.id]);
    } catch (err) {
      console.error("Failed to add repo:", err);
    }
  };

  if (loading) {
    return (
      <div className="text-sm text-gray-500 py-4 text-center">
        Loading repositories...
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {/* Search */}
      <input
        type="text"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Search repositories..."
        className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-1.5 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
      />

      {/* Repo list */}
      <div className="max-h-48 overflow-y-auto border border-gray-700 rounded-md">
        {filtered.length === 0 ? (
          <div className="text-sm text-gray-500 py-3 text-center">
            No repositories found
          </div>
        ) : (
          filtered.map((repo) => (
            <button
              key={repo.id}
              type="button"
              onClick={() => toggleRepo(repo.id)}
              className={`
                w-full text-left px-3 py-2 text-sm flex items-center gap-2
                border-b border-gray-800 last:border-b-0 transition-colors
                ${selectedIds.includes(repo.id) ? "bg-blue-500/15 text-blue-300" : "text-gray-300 hover:bg-white/5"}
              `}
            >
              <span
                className={`w-4 h-4 rounded border flex items-center justify-center text-[10px] ${
                  selectedIds.includes(repo.id)
                    ? "border-blue-500 bg-blue-500 text-white"
                    : "border-gray-600"
                }`}
              >
                {selectedIds.includes(repo.id) && "\u2713"}
              </span>
              <div className="flex-1 min-w-0">
                <div className="font-medium truncate">{repo.name}</div>
                <div className="text-xs text-gray-500 truncate">{repo.path}</div>
              </div>
            </button>
          ))
        )}
      </div>

      {/* Add new repo */}
      <div className="flex gap-2">
        <input
          type="text"
          value={addingPath}
          onChange={(e) => setAddingPath(e.target.value)}
          placeholder="/path/to/repo"
          className="flex-1 bg-gray-900 border border-gray-700 rounded-md px-3 py-1.5 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              addRepo();
            }
          }}
        />
        <button
          type="button"
          onClick={addRepo}
          disabled={!addingPath.trim()}
          className="px-3 py-1.5 text-sm bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-200 rounded-md"
        >
          Add
        </button>
      </div>
    </div>
  );
}
