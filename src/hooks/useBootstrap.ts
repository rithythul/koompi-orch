import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useWorkspaceStore } from "../stores/workspaceStore";
import { useSettingsStore } from "../stores/settingsStore";

/**
 * Bootstrap hook: loads workspaces, templates, and settings from the backend on mount.
 * Should be called once at the app root.
 */
export function useBootstrap() {
  const setWorkspaces = useWorkspaceStore((s) => s.setWorkspaces);
  const setTemplates = useSettingsStore((s) => s.setTemplates);
  const setApiKeys = useSettingsStore((s) => s.setApiKeys);
  const setTheme = useSettingsStore((s) => s.setTheme);
  const setMaxConcurrentAgents = useSettingsStore((s) => s.setMaxConcurrentAgents);
  const setDefaultAgent = useSettingsStore((s) => s.setDefaultAgent);
  const setDefaultRole = useSettingsStore((s) => s.setDefaultRole);
  const setAutoReview = useSettingsStore((s) => s.setAutoReview);
  const setAutoCheckpoint = useSettingsStore((s) => s.setAutoCheckpoint);

  useEffect(() => {
    // Load workspaces
    invoke<Record<string, unknown>[]>("list_workspaces")
      .then((ws) => {
        setWorkspaces(
          ws.map((w) => ({
            id: extractId(w.id),
            name: (w.name as string) || "",
            branch: (w.branch as string) || "",
            worktreePath: (w.worktree_path as string) || "",
            status: ((w.status as string) || "backlog").toLowerCase() as "backlog" | "active" | "review" | "done" | "failed",
            repoId: "",
            repoName: "",
            lockedBy: null,
            hasConflict: false,
            createdAt: (w.created_at as string) ?? new Date().toISOString(),
            updatedAt: (w.updated_at as string) ?? new Date().toISOString(),
          }))
        );
      })
      .catch((err) => console.warn("Failed to load workspaces:", err));

    // Load templates
    invoke<BackendTemplate[]>("list_agent_templates")
      .then((ts) => {
        setTemplates(
          ts.map((t) => ({
            id: t.id,
            name: t.name,
            command: t.command,
            args: t.args,
            inputMode: t.inputMode,
            outputMode: t.outputMode,
            builtIn: t.builtIn,
          }))
        );
      })
      .catch((err) => console.warn("Failed to load templates:", err));

    // Load settings
    invoke<BackendSettings>("get_settings")
      .then((s) => {
        if (s.app?.theme) setTheme(s.app.theme as "dark" | "light");
        if (s.app?.max_concurrent_agents) setMaxConcurrentAgents(s.app.max_concurrent_agents);
        if (s.defaults?.agent) setDefaultAgent(s.defaults.agent);
        if (s.defaults?.role) setDefaultRole(s.defaults.role);
        if (s.defaults?.auto_review !== undefined) setAutoReview(s.defaults.auto_review);
        if (s.defaults?.auto_checkpoint !== undefined) setAutoCheckpoint(s.defaults.auto_checkpoint);
      })
      .catch((err) => console.warn("Failed to load settings:", err));

    // Initialize default API keys display (credential files are opaque — we just show providers)
    setApiKeys([
      { provider: "anthropic", label: "Anthropic", hasKey: false },
      { provider: "openai", label: "OpenAI", hasKey: false },
      { provider: "google", label: "Google (Gemini)", hasKey: false },
    ]);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps
}

// Backend types
interface BackendTemplate {
  id: string;
  name: string;
  command: string;
  args: string[];
  inputMode: string;
  outputMode: string;
  builtIn: boolean;
}

interface BackendSettings {
  app?: {
    theme?: string;
    max_concurrent_agents?: number;
    data_dir?: string;
  };
  defaults?: {
    agent?: string;
    role?: string;
    auto_review?: boolean;
    auto_checkpoint?: boolean;
  };
}

/** Extract a string ID from SurrealDB Thing (handles various serialization formats) */
function extractId(thing: unknown): string {
  if (!thing) return "";
  if (typeof thing === "string") return thing;
  const t = thing as Record<string, unknown>;
  if (t.tb && t.id) {
    const inner = t.id;
    if (typeof inner === "string") return `${t.tb}:${inner}`;
    if (typeof inner === "object" && inner !== null) {
      const s = (inner as Record<string, unknown>).String;
      if (typeof s === "string") return `${t.tb}:${s}`;
    }
    return `${t.tb}:${JSON.stringify(t.id)}`;
  }
  return String(thing);
}
