import { useState } from "react";

export interface AgentPickerValue {
  agentTemplate: string;
  rolePreset: string;
}

const AGENT_TEMPLATES = [
  {
    value: "claude-code",
    label: "Claude Code",
    description: "Anthropic's coding agent. JSON streaming, resume support.",
    icon: "\uD83E\uDDE0",
  },
  {
    value: "codex",
    label: "Codex",
    description: "OpenAI's coding CLI. PTY-based input/output.",
    icon: "\uD83D\uDCBB",
  },
  {
    value: "gemini-cli",
    label: "Gemini CLI",
    description: "Google's Gemini coding agent. PTY-based.",
    icon: "\u2728",
  },
  {
    value: "aider",
    label: "Aider",
    description: "Open-source pair programmer. Chat history restore.",
    icon: "\uD83D\uDEE0\uFE0F",
  },
  {
    value: "custom",
    label: "Custom",
    description: "Your own CLI agent. Configure command and args.",
    icon: "\u2699\uFE0F",
  },
];

const ROLE_PRESETS = [
  {
    value: "architect",
    label: "Architect",
    description: "Think from first principles, design before coding.",
  },
  {
    value: "implementer",
    label: "Implementer",
    description: "Write production code, follow existing patterns.",
  },
  {
    value: "reviewer",
    label: "Reviewer",
    description: "Paranoid code review: security, race conditions, trust.",
  },
  {
    value: "tester",
    label: "Tester",
    description: "Comprehensive tests, edge cases, integration tests.",
  },
  {
    value: "shipper",
    label: "Shipper",
    description: "Final-mile: sync main, run tests, open PR.",
  },
  {
    value: "fixer",
    label: "Fixer",
    description: "Debug and fix: systematic root cause, minimal changes.",
  },
];

interface AgentPickerProps {
  value: AgentPickerValue;
  onChange: (value: AgentPickerValue) => void;
}

export function AgentPicker({ value, onChange }: AgentPickerProps) {
  const [tab, setTab] = useState<"agent" | "role">("agent");

  return (
    <div className="flex flex-col gap-3">
      {/* Tab switcher */}
      <div className="flex border-b border-gray-700">
        <button
          type="button"
          onClick={() => setTab("agent")}
          className={`px-3 py-1.5 text-sm ${
            tab === "agent"
              ? "text-blue-400 border-b-2 border-blue-400"
              : "text-gray-500 hover:text-gray-300"
          }`}
        >
          Agent
        </button>
        <button
          type="button"
          onClick={() => setTab("role")}
          className={`px-3 py-1.5 text-sm ${
            tab === "role"
              ? "text-blue-400 border-b-2 border-blue-400"
              : "text-gray-500 hover:text-gray-300"
          }`}
        >
          Role
        </button>
      </div>

      {/* Agent selection */}
      {tab === "agent" && (
        <div className="flex flex-col gap-1">
          {AGENT_TEMPLATES.map((agent) => (
            <button
              key={agent.value}
              type="button"
              onClick={() =>
                onChange({ ...value, agentTemplate: agent.value })
              }
              className={`
                w-full text-left px-3 py-2 rounded-md flex items-start gap-3 transition-colors
                ${
                  value.agentTemplate === agent.value
                    ? "bg-blue-500/15 border border-blue-500/50"
                    : "hover:bg-white/5 border border-transparent"
                }
              `}
            >
              <span className="text-lg mt-0.5">{agent.icon}</span>
              <div>
                <div className="text-sm font-medium text-gray-200">
                  {agent.label}
                </div>
                <div className="text-xs text-gray-500">{agent.description}</div>
              </div>
            </button>
          ))}
        </div>
      )}

      {/* Role selection */}
      {tab === "role" && (
        <div className="flex flex-col gap-1">
          {ROLE_PRESETS.map((role) => (
            <button
              key={role.value}
              type="button"
              onClick={() =>
                onChange({ ...value, rolePreset: role.value })
              }
              className={`
                w-full text-left px-3 py-2 rounded-md transition-colors
                ${
                  value.rolePreset === role.value
                    ? "bg-purple-500/15 border border-purple-500/50"
                    : "hover:bg-white/5 border border-transparent"
                }
              `}
            >
              <div className="text-sm font-medium text-gray-200">
                {role.label}
              </div>
              <div className="text-xs text-gray-500">{role.description}</div>
            </button>
          ))}
        </div>
      )}

      {/* Current selection summary */}
      <div className="text-xs text-gray-500 px-1">
        Selected:{" "}
        <span className="text-gray-300">
          {AGENT_TEMPLATES.find((a) => a.value === value.agentTemplate)?.label}
        </span>
        {" + "}
        <span className="text-purple-300">
          {ROLE_PRESETS.find((r) => r.value === value.rolePreset)?.label}
        </span>
      </div>
    </div>
  );
}
