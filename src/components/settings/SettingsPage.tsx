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
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="h-[48px] px-6 flex items-center justify-between border-b border-border shrink-0">
        <div className="flex items-center gap-3">
          <h2 className="text-[13px] font-semibold text-text-primary">Settings</h2>
          <span className="text-text-ghost">·</span>
          <span className="text-[11px] text-text-tertiary">Preferences & configuration</span>
        </div>
      </div>

      <div className="flex-1 overflow-auto">
        <div className="max-w-2xl mx-auto py-6 px-6 flex flex-col gap-8 stagger-children">

          {/* Appearance */}
          <SettingsSection label="Appearance">
            <SettingsRow label="Theme" description="Switch between dark and light mode">
              <ThemeToggle theme={theme} onToggle={handleThemeToggle} />
            </SettingsRow>
          </SettingsSection>

          {/* General */}
          <SettingsSection label="General">
            <SettingsRow label="Max concurrent agents" description="Maximum number of agents running in parallel">
              <input
                type="number"
                min={1}
                max={50}
                value={maxConcurrentAgents}
                onChange={(e) => setMaxConcurrentAgents(Number(e.target.value) || 10)}
                className="w-16 bg-[rgba(255,255,255,0.03)] border border-border rounded-md px-2.5 py-1.5 text-[13px] font-mono text-text-primary text-center focus:outline-none focus:border-accent transition-colors"
              />
            </SettingsRow>
            <SettingsRow label="Default agent" description="Agent used when creating new sessions">
              <SelectInput value={defaultAgent} onChange={setDefaultAgent} options={[
                { value: "claude-code", label: "Claude Code" },
                { value: "codex", label: "Codex" },
                { value: "gemini-cli", label: "Gemini CLI" },
                { value: "aider", label: "Aider" },
                { value: "custom", label: "Custom" },
              ]} />
            </SettingsRow>
            <SettingsRow label="Default role" description="Role assigned to new pipeline steps">
              <SelectInput value={defaultRole} onChange={setDefaultRole} options={[
                { value: "architect", label: "Architect" },
                { value: "implementer", label: "Implementer" },
                { value: "reviewer", label: "Reviewer" },
                { value: "tester", label: "Tester" },
                { value: "shipper", label: "Shipper" },
                { value: "fixer", label: "Fixer" },
              ]} />
            </SettingsRow>
            <SettingsRow label="Auto-review" description="Automatically trigger review after implementation">
              <ToggleSwitch checked={autoReview} onChange={setAutoReview} />
            </SettingsRow>
            <SettingsRow label="Auto-checkpoint" description="Create git checkpoints between pipeline steps">
              <ToggleSwitch checked={autoCheckpoint} onChange={setAutoCheckpoint} />
            </SettingsRow>
          </SettingsSection>

          {/* API Keys */}
          <SettingsSection label="API Keys" description="Stored securely via OS keychain (Stronghold). Never written to config files.">
            <ApiKeyManager
              keys={apiKeys.length > 0 ? apiKeys : DEFAULT_API_KEYS}
              onSaveKey={handleSaveKey}
              onDeleteKey={handleDeleteKey}
            />
          </SettingsSection>

          {/* Agent Templates */}
          <SettingsSection label="Agent Templates">
            <AgentTemplates
              templates={templates}
              onSave={handleSaveTemplate}
              onDelete={handleDeleteTemplate}
            />
          </SettingsSection>
        </div>
      </div>
    </div>
  );
}

/* — Shared settings primitives — */

function SettingsSection({ label, description, children }: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <section>
      <div className="mb-3">
        <h2 className="text-[10px] font-semibold font-mono text-text-ghost uppercase tracking-widest">
          {label}
        </h2>
        {description && (
          <p className="text-[11px] text-text-ghost mt-1">{description}</p>
        )}
      </div>
      <div className="flex flex-col gap-1 card-glass rounded-lg overflow-hidden">
        {children}
      </div>
    </section>
  );
}

function SettingsRow({ label, description, children }: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between px-4 py-3 border-b border-border last:border-b-0">
      <div className="flex-1 min-w-0 mr-4">
        <span className="text-[13px] text-text-primary">{label}</span>
        {description && (
          <p className="text-[11px] text-text-ghost mt-0.5">{description}</p>
        )}
      </div>
      {children}
    </div>
  );
}

function SelectInput({ value, onChange, options }: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="bg-[rgba(255,255,255,0.03)] border border-border rounded-md px-2.5 py-1.5 text-[13px] text-text-primary focus:outline-none focus:border-accent transition-colors cursor-pointer"
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>{o.label}</option>
      ))}
    </select>
  );
}

function ToggleSwitch({ checked, onChange }: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      className={`relative w-9 h-5 rounded-full transition-colors duration-200 ${
        checked ? "bg-accent" : "bg-[rgba(255,255,255,0.1)]"
      }`}
    >
      <span
        className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow-sm transform transition-transform duration-200 ${
          checked ? "translate-x-4" : "translate-x-0"
        }`}
      />
    </button>
  );
}
