import { useState, useCallback, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import { useNotificationStore } from "../../stores/notificationStore";

interface WorkspaceCreateProps {
  open: boolean;
  onClose: () => void;
}

interface RepoOption {
  id: string;
  name: string;
  path: string;
}

const AGENT_TEMPLATES = [
  { value: "claude-code", label: "Claude Code" },
  { value: "codex", label: "Codex" },
  { value: "gemini-cli", label: "Gemini CLI" },
  { value: "aider", label: "Aider" },
  { value: "custom", label: "Custom" },
];

const ROLE_PRESETS = [
  { value: "architect", label: "Architect" },
  { value: "implementer", label: "Implementer" },
  { value: "reviewer", label: "Reviewer" },
  { value: "tester", label: "Tester" },
  { value: "shipper", label: "Shipper" },
  { value: "fixer", label: "Fixer" },
];

export function WorkspaceCreate({ open, onClose }: WorkspaceCreateProps) {
  const addWorkspace = useWorkspaceStore((s) => s.addWorkspace);
  const addNotification = useNotificationStore((s) => s.addNotification);

  const [repos, setRepos] = useState<RepoOption[]>([]);
  const [selectedRepoId, setSelectedRepoId] = useState("");
  const [workspaceName, setWorkspaceName] = useState("");
  const [branchName, setBranchName] = useState("");
  const [agentTemplate, setAgentTemplate] = useState("claude-code");
  const [rolePreset, setRolePreset] = useState("implementer");
  const [initialPrompt, setInitialPrompt] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [reposLoaded, setReposLoaded] = useState(false);

  const loadRepos = useCallback(async () => {
    if (reposLoaded) return;
    try {
      const result = await invoke<RepoOption[]>("list_repos");
      setRepos(result);
      setReposLoaded(true);
    } catch (err) {
      console.error("Failed to load repos:", err);
    }
  }, [reposLoaded]);

  if (open && !reposLoaded) {
    loadRepos();
  }

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!selectedRepoId || !workspaceName || !branchName) return;

    setIsSubmitting(true);
    try {
      const workspace = await invoke<{
        id: string;
        worktree_path: string;
        repo_name: string;
      }>("create_workspace", {
        repoId: selectedRepoId,
        name: workspaceName,
        branch: branchName,
        agentTemplate,
        rolePreset,
        initialPrompt: initialPrompt || null,
      });

      addWorkspace({
        id: workspace.id,
        name: workspaceName,
        branch: branchName,
        worktreePath: workspace.worktree_path,
        status: initialPrompt ? "active" : "backlog",
        repoId: selectedRepoId,
        repoName: workspace.repo_name,
        lockedBy: null,
        hasConflict: false,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      });

      addNotification({
        type: "success",
        title: "Workspace created",
        message: `${workspaceName} on ${branchName}`,
        autoCloseMs: 3000,
      });

      setWorkspaceName("");
      setBranchName("");
      setInitialPrompt("");
      onClose();
    } catch (err) {
      addNotification({
        type: "error",
        title: "Failed to create workspace",
        message: String(err),
        autoCloseMs: 5000,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-gray-800 rounded-xl border border-gray-700 shadow-xl w-full max-w-md p-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-100">
            New Workspace
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="text-gray-500 hover:text-gray-300 text-xl leading-none"
          >
            &times;
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          {/* Repository */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Repository
            </label>
            <select
              value={selectedRepoId}
              onChange={(e) => setSelectedRepoId(e.target.value)}
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              required
            >
              <option value="">Select a repository...</option>
              {repos.map((repo) => (
                <option key={repo.id} value={repo.id}>
                  {repo.name} — {repo.path}
                </option>
              ))}
            </select>
          </div>

          {/* Workspace Name */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Workspace Name
            </label>
            <input
              type="text"
              value={workspaceName}
              onChange={(e) => setWorkspaceName(e.target.value)}
              placeholder="feat-auth"
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              required
            />
          </div>

          {/* Branch Name */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Branch Name
            </label>
            <input
              type="text"
              value={branchName}
              onChange={(e) => setBranchName(e.target.value)}
              placeholder="feat/auth-jwt"
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              required
            />
          </div>

          {/* Agent Template */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Agent
            </label>
            <select
              value={agentTemplate}
              onChange={(e) => setAgentTemplate(e.target.value)}
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
            >
              {AGENT_TEMPLATES.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </select>
          </div>

          {/* Role Preset */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Role
            </label>
            <select
              value={rolePreset}
              onChange={(e) => setRolePreset(e.target.value)}
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
            >
              {ROLE_PRESETS.map((r) => (
                <option key={r.value} value={r.value}>
                  {r.label}
                </option>
              ))}
            </select>
          </div>

          {/* Initial Prompt (optional) */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Initial Prompt{" "}
              <span className="text-gray-600">(optional — starts agent immediately)</span>
            </label>
            <textarea
              value={initialPrompt}
              onChange={(e) => setInitialPrompt(e.target.value)}
              placeholder="Implement JWT auth module with refresh tokens..."
              rows={3}
              className="w-full bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500 resize-none"
            />
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-2 mt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm text-gray-400 hover:text-gray-200 rounded-md"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !selectedRepoId || !workspaceName || !branchName}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-md"
            >
              {isSubmitting ? "Creating..." : "Create Workspace"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
