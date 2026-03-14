import { invoke } from "@tauri-apps/api/core";

// -- Types matching Rust schema --

export interface Repo {
  id?: string;
  path: string;
  name: string;
  remote_url?: string;
  added_at?: string;
}

export interface Workspace {
  id?: string;
  name: string;
  branch: string;
  worktree_path: string;
  status: "backlog" | "active" | "review" | "done" | "failed";
  locked_by?: string;
  created_at?: string;
  updated_at?: string;
}

export interface Session {
  id?: string;
  agent_type: string;
  model?: string;
  pid?: number;
  role_preset?: string;
  status: "running" | "paused" | "completed" | "crashed";
  started_at?: string;
  ended_at?: string;
  config: Record<string, unknown>;
}

export interface AgentTemplate {
  id?: string;
  name: string;
  command: string;
  default_args: string[];
  env?: Record<string, string>;
  input_mode: "pty_stdin" | "flag_message" | "file_prompt";
  output_mode: "json_stream" | "text_markers" | "raw_pty";
  resume_support: boolean;
  builtin: boolean;
}

export interface RolePreset {
  id?: string;
  name: string;
  system_prompt: string;
  description: string;
  injection_method: "flag" | "env_var" | "config_file" | "first_message";
  builtin: boolean;
}

export interface Metric {
  id?: string;
  tokens_in: number;
  tokens_out: number;
  cost_usd: number;
  duration_ms: number;
  turn_number: number;
  recorded_at?: string;
}

export interface AppConfig {
  app: {
    theme: string;
    data_dir: string;
    max_concurrent_agents: number;
    handoff_retention_days: number;
  };
  defaults: {
    agent: string;
    role: string;
    auto_review: boolean;
    auto_checkpoint: boolean;
  };
  notifications: {
    agent_completed: boolean;
    agent_failed: boolean;
    agent_needs_input: boolean;
    ci_status: boolean;
  };
}

// -- IPC wrappers --

export const ipc = {
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<void>("save_config", { config }),

  listRepos: () => invoke<Repo[]>("list_repos"),
  addRepo: (path: string, name: string, remote_url?: string) =>
    invoke<Repo>("add_repo", { path, name, remote_url }),

  listWorkspaces: () => invoke<Workspace[]>("list_workspaces"),
  listWorkspacesByStatus: (status: string) =>
    invoke<Workspace[]>("list_workspaces_by_status", { status }),
  createWorkspace: (name: string, branch: string, worktree_path: string, repo_id: string) =>
    invoke<Workspace>("create_workspace", { name, branch, worktree_path, repo_id }),
  updateWorkspaceStatus: (id: string, status: string) =>
    invoke<void>("update_workspace_status", { id, status }),

  listSessions: (workspace_id: string) =>
    invoke<Session[]>("list_sessions", { workspace_id }),

  listTemplates: () => invoke<AgentTemplate[]>("list_templates"),
  listPresets: () => invoke<RolePreset[]>("list_presets"),

  getSessionMetrics: (session_id: string) =>
    invoke<Metric[]>("get_session_metrics", { session_id }),
};
