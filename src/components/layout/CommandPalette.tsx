import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useWorkspaceStore } from "../../stores/workspaceStore";

export interface PaletteAction {
  id: string;
  label: string;
  description?: string;
  shortcut?: string;
  category: "workspace" | "agent" | "navigation" | "action";
  onExecute: () => void;
}

interface CommandPaletteProps {
  /** Additional actions beyond the built-in ones */
  actions?: PaletteAction[];
  /** Callback for workspace navigation */
  onNavigateWorkspace?: (workspaceId: string) => void;
  /** Callback for creating new workspace */
  onNewWorkspace?: () => void;
}

/** Simple fuzzy match: checks if all characters in query appear in target in order */
function fuzzyMatch(query: string, target: string): { match: boolean; score: number } {
  const q = query.toLowerCase();
  const t = target.toLowerCase();

  if (q.length === 0) return { match: true, score: 0 };

  let qi = 0;
  let score = 0;
  let prevMatchIndex = -1;

  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) {
      // Consecutive matches score higher
      if (prevMatchIndex === ti - 1) score += 2;
      // Word boundary matches score higher
      if (ti === 0 || t[ti - 1] === " " || t[ti - 1] === "-" || t[ti - 1] === "/") {
        score += 3;
      }
      score += 1;
      prevMatchIndex = ti;
      qi++;
    }
  }

  return { match: qi === q.length, score };
}

export function CommandPalette({
  actions: externalActions = [],
  onNavigateWorkspace,
  onNewWorkspace,
}: CommandPaletteProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const selectWorkspace = useWorkspaceStore((s) => s.selectWorkspace);

  // Built-in actions
  const builtInActions: PaletteAction[] = useMemo(
    () => [
      {
        id: "new-workspace",
        label: "New Workspace",
        description: "Create a new workspace with agent",
        shortcut: "Mod+N",
        category: "action",
        onExecute: () => onNewWorkspace?.(),
      },
      {
        id: "toggle-left-sidebar",
        label: "Toggle Left Sidebar",
        shortcut: "Mod+[",
        category: "navigation",
        onExecute: () => {
          document.dispatchEvent(
            new CustomEvent("koompi-orch:toggle-sidebar", {
              detail: { side: "left" },
            })
          );
        },
      },
      {
        id: "toggle-right-sidebar",
        label: "Toggle Right Sidebar",
        shortcut: "Mod+]",
        category: "navigation",
        onExecute: () => {
          document.dispatchEvent(
            new CustomEvent("koompi-orch:toggle-sidebar", {
              detail: { side: "right" },
            })
          );
        },
      },
      {
        id: "zen-mode",
        label: "Zen Mode",
        description: "Hide both sidebars",
        shortcut: "Mod+Shift+Z",
        category: "navigation",
        onExecute: () => {
          document.dispatchEvent(
            new CustomEvent("koompi-orch:zen-mode")
          );
        },
      },
      {
        id: "copy-chat-markdown",
        label: "Copy Chat as Markdown",
        shortcut: "Mod+Shift+C",
        category: "action",
        onExecute: () => {
          document.dispatchEvent(
            new CustomEvent("koompi-orch:copy-chat")
          );
        },
      },
      // Dynamic workspace entries
      ...workspaces.map((ws) => ({
        id: `ws-${ws.id}`,
        label: ws.name,
        description: `${ws.repoName} / ${ws.branch} [${ws.status}]`,
        category: "workspace" as const,
        onExecute: () => {
          selectWorkspace(ws.id);
          onNavigateWorkspace?.(ws.id);
        },
      })),
    ],
    [workspaces, selectWorkspace, onNavigateWorkspace, onNewWorkspace]
  );

  const allActions = useMemo(
    () => [...builtInActions, ...externalActions],
    [builtInActions, externalActions]
  );

  // Filter and sort by fuzzy match score
  const filteredActions = useMemo(() => {
    if (!query.trim()) return allActions;
    return allActions
      .map((action) => {
        const labelMatch = fuzzyMatch(query, action.label);
        const descMatch = action.description
          ? fuzzyMatch(query, action.description)
          : { match: false, score: 0 };
        const bestScore = Math.max(
          labelMatch.match ? labelMatch.score : 0,
          descMatch.match ? descMatch.score : 0
        );
        return { action, match: labelMatch.match || descMatch.match, score: bestScore };
      })
      .filter((r) => r.match)
      .sort((a, b) => b.score - a.score)
      .map((r) => r.action);
  }, [allActions, query]);

  // Reset selected index when results change
  useEffect(() => {
    setSelectedIndex(0);
  }, [filteredActions.length]);

  // Global Mod+K listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setOpen((prev) => !prev);
        setQuery("");
        setSelectedIndex(0);
      }
      if (e.key === "Escape" && open) {
        e.preventDefault();
        setOpen(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open]);

  // Focus input when opened
  useEffect(() => {
    if (open) {
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // Scroll selected item into view
  useEffect(() => {
    const list = listRef.current;
    if (!list) return;
    const selected = list.children[selectedIndex] as HTMLElement | undefined;
    selected?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  const executeAction = useCallback(
    (action: PaletteAction) => {
      setOpen(false);
      setQuery("");
      action.onExecute();
    },
    []
  );

  const handleInputKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) =>
        prev < filteredActions.length - 1 ? prev + 1 : 0
      );
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) =>
        prev > 0 ? prev - 1 : filteredActions.length - 1
      );
    } else if (e.key === "Enter") {
      e.preventDefault();
      const action = filteredActions[selectedIndex];
      if (action) executeAction(action);
    }
  };

  if (!open) return null;

  // Group by category
  const categoryOrder: PaletteAction["category"][] = [
    "action",
    "workspace",
    "navigation",
    "agent",
  ];
  const categoryLabels: Record<string, string> = {
    action: "Actions",
    workspace: "Workspaces",
    navigation: "Navigation",
    agent: "Agents",
  };

  let globalIndex = 0;

  return (
    <div className="fixed inset-0 z-[100] flex items-start justify-center pt-[15vh]">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={() => setOpen(false)}
        onKeyDown={() => {}}
        role="presentation"
      />

      {/* Palette */}
      <div className="relative w-full max-w-lg bg-gray-800 border border-gray-700 rounded-xl shadow-2xl overflow-hidden">
        {/* Search input */}
        <div className="flex items-center px-4 py-3 border-b border-gray-700">
          <span className="text-gray-500 mr-2 text-sm">&#x1F50D;</span>
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleInputKeyDown}
            placeholder="Search actions, workspaces..."
            className="flex-1 bg-transparent text-sm text-gray-200 outline-none placeholder:text-gray-600"
          />
          <kbd className="text-[10px] text-gray-600 bg-gray-900 px-1.5 py-0.5 rounded border border-gray-700">
            ESC
          </kbd>
        </div>

        {/* Results */}
        <div ref={listRef} className="max-h-[50vh] overflow-y-auto py-1">
          {filteredActions.length === 0 ? (
            <div className="text-sm text-gray-500 text-center py-6">
              No results for &quot;{query}&quot;
            </div>
          ) : (
            categoryOrder.map((category) => {
              const items = filteredActions.filter(
                (a) => a.category === category
              );
              if (items.length === 0) return null;

              return (
                <div key={category}>
                  <div className="px-4 py-1 text-[10px] font-semibold uppercase text-gray-600">
                    {categoryLabels[category]}
                  </div>
                  {items.map((action) => {
                    const idx = globalIndex++;
                    return (
                      <button
                        key={action.id}
                        type="button"
                        onClick={() => executeAction(action)}
                        onMouseEnter={() => setSelectedIndex(idx)}
                        className={`
                          w-full text-left flex items-center justify-between px-4 py-2 text-sm
                          ${idx === selectedIndex ? "bg-blue-500/15 text-blue-300" : "text-gray-300 hover:bg-white/5"}
                        `}
                      >
                        <div className="flex-1 min-w-0">
                          <span className="truncate">{action.label}</span>
                          {action.description && (
                            <span className="ml-2 text-xs text-gray-500 truncate">
                              {action.description}
                            </span>
                          )}
                        </div>
                        {action.shortcut && (
                          <kbd className="text-[10px] text-gray-600 bg-gray-900 px-1.5 py-0.5 rounded border border-gray-700 ml-2 whitespace-nowrap">
                            {action.shortcut}
                          </kbd>
                        )}
                      </button>
                    );
                  })}
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center gap-3 px-4 py-2 border-t border-gray-700 text-[10px] text-gray-600">
          <span>
            <kbd className="bg-gray-900 px-1 py-0.5 rounded border border-gray-700">
              &#x2191;&#x2193;
            </kbd>{" "}
            navigate
          </span>
          <span>
            <kbd className="bg-gray-900 px-1 py-0.5 rounded border border-gray-700">
              &#x23CE;
            </kbd>{" "}
            select
          </span>
          <span>
            <kbd className="bg-gray-900 px-1 py-0.5 rounded border border-gray-700">
              esc
            </kbd>{" "}
            close
          </span>
        </div>
      </div>
    </div>
  );
}
