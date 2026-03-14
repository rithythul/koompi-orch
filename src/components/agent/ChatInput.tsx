import { useState, useRef, useCallback, type KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAgentStore, type ChatMessage } from "../../stores/agentStore";

const AGENT_OPTIONS = [
  { value: "claude-code", label: "Claude Code" },
  { value: "codex", label: "Codex" },
  { value: "gemini-cli", label: "Gemini CLI" },
  { value: "aider", label: "Aider" },
];

const MODEL_OPTIONS: Record<string, string[]> = {
  "claude-code": ["opus-4.6", "sonnet-4.5", "haiku-3.5"],
  codex: ["o3", "o4-mini"],
  "gemini-cli": ["gemini-2.5-pro", "gemini-2.5-flash"],
  aider: ["opus-4.6", "sonnet-4.5"],
};

interface ChatInputProps {
  sessionId: string;
  workspaceId: string;
  disabled?: boolean;
}

export function ChatInput({
  sessionId,
  workspaceId,
  disabled = false,
}: ChatInputProps) {
  const appendMessage = useAgentStore((s) => s.appendMessage);
  const session = useAgentStore((s) => s.sessions[sessionId]);

  const [message, setMessage] = useState("");
  const [selectedAgent, setSelectedAgent] = useState(
    session?.agentType ?? "claude-code"
  );
  const [selectedModel, setSelectedModel] = useState(
    session?.model ?? MODEL_OPTIONS["claude-code"][0]
  );
  const [sending, setSending] = useState(false);
  const [showDropdown, setShowDropdown] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleAgentChange = (agent: string) => {
    setSelectedAgent(agent);
    const models = MODEL_OPTIONS[agent] ?? [];
    setSelectedModel(models[0] ?? "");
    setShowDropdown(false);
  };

  const sendMessage = useCallback(async () => {
    const trimmed = message.trim();
    if (!trimmed || sending || disabled) return;

    setSending(true);

    const userMsg: ChatMessage = {
      id: `msg-${Date.now()}`,
      role: "user",
      content: trimmed,
      turn: (session?.messages.length ?? 0) + 1,
      timestamp: new Date().toISOString(),
    };
    appendMessage(sessionId, userMsg);
    setMessage("");

    try {
      await invoke("send_message_to_agent", {
        sessionId,
        workspaceId,
        message: trimmed,
        agentType: selectedAgent,
        model: selectedModel,
      });
    } catch (err) {
      console.error("Failed to send message:", err);
    } finally {
      setSending(false);
      textareaRef.current?.focus();
    }
  }, [message, sending, disabled, sessionId, workspaceId, selectedAgent, selectedModel, session, appendMessage]);

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  const handleInput = (value: string) => {
    setMessage(value);
    const el = textareaRef.current;
    if (el) {
      el.style.height = "auto";
      el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
    }
  };

  return (
    <div className="border-t border-gray-700 p-3">
      <div className="flex items-end gap-2">
        {/* Agent/Model selector */}
        <div className="relative">
          <button
            type="button"
            onClick={() => setShowDropdown(!showDropdown)}
            className="flex items-center gap-1 px-2 py-1.5 text-xs bg-gray-800 border border-gray-700 rounded-md text-gray-300 hover:bg-gray-750 whitespace-nowrap"
          >
            <span>{AGENT_OPTIONS.find((a) => a.value === selectedAgent)?.label}</span>
            <span className="text-gray-500">/</span>
            <span className="text-blue-400">{selectedModel}</span>
            <span className="text-gray-500 ml-1">{"\u25BE"}</span>
          </button>

          {showDropdown && (
            <div className="absolute bottom-full mb-1 left-0 bg-gray-800 border border-gray-700 rounded-md shadow-lg z-10 min-w-[200px]">
              {AGENT_OPTIONS.map((agent) => (
                <div key={agent.value}>
                  <button
                    type="button"
                    onClick={() => handleAgentChange(agent.value)}
                    className={`w-full text-left px-3 py-1.5 text-sm hover:bg-gray-700 ${
                      selectedAgent === agent.value
                        ? "text-blue-400"
                        : "text-gray-300"
                    }`}
                  >
                    {agent.label}
                  </button>
                  {selectedAgent === agent.value && (
                    <div className="pl-4">
                      {(MODEL_OPTIONS[agent.value] ?? []).map((model) => (
                        <button
                          key={model}
                          type="button"
                          onClick={() => {
                            setSelectedModel(model);
                            setShowDropdown(false);
                          }}
                          className={`w-full text-left px-3 py-1 text-xs hover:bg-gray-700 ${
                            selectedModel === model
                              ? "text-blue-300"
                              : "text-gray-500"
                          }`}
                        >
                          {model}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Textarea */}
        <textarea
          ref={textareaRef}
          value={message}
          onChange={(e) => handleInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a message... (Enter to send, Shift+Enter for newline)"
          disabled={disabled || sending}
          rows={1}
          className="flex-1 bg-gray-900 border border-gray-700 rounded-md px-3 py-2 text-sm text-gray-200 resize-none focus:outline-none focus:border-blue-500 disabled:opacity-50 min-h-[38px] max-h-[200px]"
        />

        {/* Send button */}
        <button
          type="button"
          onClick={sendMessage}
          disabled={!message.trim() || sending || disabled}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-md"
        >
          {sending ? "..." : "Send"}
        </button>
      </div>
    </div>
  );
}
