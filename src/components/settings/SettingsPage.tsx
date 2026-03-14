import { useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore, type Theme } from "../../stores/settingsStore";
import { ThemeToggle } from "./ThemeToggle";
import { ApiKeyManager } from "./ApiKeyManager";
import { AgentTemplates } from "./AgentTemplates";

const DEFAULT_API_KEYS = [
  { provider: "anthropic", label: "Anthropic", hasKey: false },
  { provider: "openai", label: "OpenAI", hasKey: false },
  { provider: "google", label: "Google (Gemini)", hasKey: false },
];

export function SettingsPage() {
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);
  const maxConcurrentAgents = useSettingsStore((s) => s.maxConcurrentAgents);
  const setMaxConcurrentAgents = useSettingsStore(
    (s) => s.setMaxConcurrentAgents
  );
  const defaultAgent = useSettingsStore((s) => s.defaultAgent);
  const setDefaultAgent = useSettingsStore((s) => s.setDefaultAgent);
  const defaultRole = useSettingsStore((s) => s.defaultRole);
  const setDefaultRole = useSettingsStore((s) => s.setDefaultRole);
  const autoReview = useSettingsStore((s) => s.autoReview);
  const setAutoReview = useSettingsStore((s) => s.setAutoReview);
  const autoCheckpoint = useSettingsStore((s) => s.autoCheckpoint);
  const setAutoCheckpoint = useSettingsStore((s) => s.setAutoCheckpoint);
  const apiKeys = useSettingsStore((s) => s.apiKeys);
  const setApiKeys = useSettingsStore((s) => s.setApiKeys);
  const updateApiKey = useSettingsStore((s) => s.updateApiKey);
  const templates = useSettingsStore((s) => s.templates);
  const setTemplates = useSettingsStore((s) => s.setTemplates);

  useEffect(() => {
    invoke<{ apiKeys: typeof DEFAULT_API_KEYS }>("get_settings")
      .then((settings) => {
        if (settings.apiKeys) setApiKeys(settings.apiKeys);
      })
      .catch(() => {
        if (apiKeys.length === 0) setApiKeys(DEFAULT_API_KEYS);
      });

    invoke<typeof templates>("list_agent_templates")
      .then((t) => { if (Array.isArray(t)) setTemplates(t); })
      .catch(() => {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleThemeToggle = useCallback(
    (newTheme: Theme) => {
      setTheme(newTheme);
      invoke("set_setting", { key: "theme", value: newTheme }).catch(
        (err: unknown) => console.error("Failed to save theme:", err)
      );
      document.documentElement.classList.toggle("dark", newTheme === "dark");
      document.documentElement.classList.toggle("light", newTheme === "light");
    },
    [setTheme]
  );

  const handleSaveKey = useCallback(
    (provider: string, key: string) => {
      invoke("save_api_key", { provider, key })
        .then(() => updateApiKey(provider, true))
        .catch((err: unknown) =>
          console.error("Failed to save API key:", err)
        );
    },
    [updateApiKey]
  );

  const handleDeleteKey = useCallback(
    (provider: string) => {
      invoke("delete_api_key", { provider })
        .then(() => updateApiKey(provider, false))
        .catch((err: unknown) =>
          console.error("Failed to delete API key:", err)
        );
    },
    [updateApiKey]
  );

  const handleSaveTemplate = useCallback(
    (template: (typeof templates)[0]) => {
      invoke("save_agent_template", { template })
        .then(() => {
          const exists = templates.find((t) => t.id === template.id);
          if (exists) {
            setTemplates(
              templates.map((t) => (t.id === template.id ? template : t))
            );
          } else {
            setTemplates([...templates, template]);
          }
        })
        .catch((err: unknown) =>
          console.error("Failed to save template:", err)
        );
    },
    [templates, setTemplates]
  );

  const handleDeleteTemplate = useCallback(
    (id: string) => {
      invoke("delete_agent_template", { id })
        .then(() => setTemplates(templates.filter((t) => t.id !== id)))
        .catch((err: unknown) =>
          console.error("Failed to delete template:", err)
        );
    },
    [templates, setTemplates]
  );

  return (
    <div className="max-w-2xl mx-auto py-6 px-4 flex flex-col gap-8">
      <h1 className="text-xl font-bold text-gray-100">Settings</h1>

      <section>
        <h2 className="text-sm font-semibold text-gray-300 mb-3 uppercase tracking-wider">
          Appearance
        </h2>
        <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
          <span className="text-sm text-gray-300">Theme</span>
          <ThemeToggle theme={theme} onToggle={handleThemeToggle} />
        </div>
      </section>

      <section>
        <h2 className="text-sm font-semibold text-gray-300 mb-3 uppercase tracking-wider">
          General
        </h2>
        <div className="flex flex-col gap-3">
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">
              Max concurrent agents
            </span>
            <input
              type="number"
              min={1}
              max={50}
              value={maxConcurrentAgents}
              onChange={(e) =>
                setMaxConcurrentAgents(Number(e.target.value) || 10)
              }
              className="w-16 bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 text-center focus:outline-none focus:border-blue-500"
            />
          </div>
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">Default agent</span>
            <select
              value={defaultAgent}
              onChange={(e) => setDefaultAgent(e.target.value)}
              className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
            >
              <option value="claude-code">Claude Code</option>
              <option value="codex">Codex</option>
              <option value="gemini-cli">Gemini CLI</option>
              <option value="aider">Aider</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">Default role</span>
            <select
              value={defaultRole}
              onChange={(e) => setDefaultRole(e.target.value)}
              className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
            >
              <option value="architect">Architect</option>
              <option value="implementer">Implementer</option>
              <option value="reviewer">Reviewer</option>
              <option value="tester">Tester</option>
              <option value="shipper">Shipper</option>
              <option value="fixer">Fixer</option>
            </select>
          </div>
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">Auto-review</span>
            <button
              type="button"
              onClick={() => setAutoReview(!autoReview)}
              className={`w-10 h-5 rounded-full transition-colors ${
                autoReview ? "bg-blue-500" : "bg-gray-600"
              }`}
            >
              <span
                className={`block w-4 h-4 rounded-full bg-white transform transition-transform ${
                  autoReview ? "translate-x-5" : "translate-x-0.5"
                }`}
              />
            </button>
          </div>
          <div className="flex items-center justify-between px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-sm text-gray-300">Auto-checkpoint</span>
            <button
              type="button"
              onClick={() => setAutoCheckpoint(!autoCheckpoint)}
              className={`w-10 h-5 rounded-full transition-colors ${
                autoCheckpoint ? "bg-blue-500" : "bg-gray-600"
              }`}
            >
              <span
                className={`block w-4 h-4 rounded-full bg-white transform transition-transform ${
                  autoCheckpoint ? "translate-x-5" : "translate-x-0.5"
                }`}
              />
            </button>
          </div>
        </div>
      </section>

      <section>
        <h2 className="text-sm font-semibold text-gray-300 mb-3 uppercase tracking-wider">
          API Keys
        </h2>
        <p className="text-xs text-gray-500 mb-3">
          Keys are stored securely via OS keychain (Stronghold). They are never
          written to config files.
        </p>
        <ApiKeyManager
          keys={apiKeys.length > 0 ? apiKeys : DEFAULT_API_KEYS}
          onSaveKey={handleSaveKey}
          onDeleteKey={handleDeleteKey}
        />
      </section>

      <section>
        <h2 className="text-sm font-semibold text-gray-300 mb-3 uppercase tracking-wider">
          Agent Templates
        </h2>
        <AgentTemplates
          templates={templates}
          onSave={handleSaveTemplate}
          onDelete={handleDeleteTemplate}
        />
      </section>
    </div>
  );
}
