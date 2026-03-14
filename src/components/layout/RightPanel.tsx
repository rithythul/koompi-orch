import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAgentStore, type ChatMessage } from "../../stores/agentStore";

interface RightPanelProps {
  collapsed: boolean;
}

export function RightPanel({ collapsed }: RightPanelProps) {
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const activeSession = useAgentStore((s) => s.activeSession)();
  const appendMessage = useAgentStore((s) => s.appendMessage);

  const isConnected = activeSession?.status === "running";
  const statusColor = isConnected ? "bg-success" : "bg-text-ghost";
  const statusText = activeSession
    ? activeSession.status
    : "offline";

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [activeSession?.messages.length]);

  const handleSend = async () => {
    if (!input.trim() || !activeSession) return;

    const message = input.trim();
    setInput("");

    // Add user message to chat
    appendMessage(activeSession.id, {
      id: `user-${Date.now()}`,
      role: "user",
      content: message,
      turn: 0,
      timestamp: new Date().toISOString(),
    });

    // If no session is running, spawn one
    if (activeSession.status !== "running") {
      try {
        await invoke("spawn_agent", {
          agentType: activeSession.agentType,
          workspacePath: ".",
          task: message,
        });
      } catch (err) {
        console.error("Failed to spawn agent:", err);
      }
    }
  };

  if (collapsed) return null;

  return (
    <aside className="w-[320px] bg-secondary/80 backdrop-blur-xl border-l border-border flex flex-col h-full">
      {/* Header */}
      <div className="h-[48px] px-4 flex items-center justify-between border-b border-border">
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${statusColor}`} />
          <h2 className="text-[13px] font-medium text-text-primary">Agent Chat</h2>
        </div>
        <span className="text-[10px] font-mono text-text-ghost">{statusText}</span>
      </div>

      {/* Messages or empty state */}
      {activeSession && activeSession.messages.length > 0 ? (
        <div className="flex-1 overflow-auto p-3 flex flex-col gap-2">
          {activeSession.messages.map((msg) => (
            <MessageBubble key={msg.id} message={msg} />
          ))}
          <div ref={messagesEndRef} />
        </div>
      ) : (
        <div className="flex-1 flex flex-col items-center justify-center px-6">
          <div className="w-10 h-10 rounded-xl bg-input-bg border border-border flex items-center justify-center mb-4">
            <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2" className="text-text-ghost">
              <path d="M2 4L8 1L14 4V12L8 15L2 12V4Z"/>
              <path d="M8 7V10M8 12V12.5" strokeLinecap="round"/>
            </svg>
          </div>
          <p className="text-[13px] text-text-tertiary text-center leading-relaxed">
            Select a workspace to start an agent session
          </p>
          <p className="text-[11px] text-text-ghost text-center mt-1">
            Agents will stream output here in real-time
          </p>
        </div>
      )}

      {/* Metrics bar */}
      {activeSession && activeSession.metrics.costUsd > 0 && (
        <div className="px-3 py-1.5 border-t border-border flex items-center gap-3 text-[10px] font-mono text-text-ghost">
          <span>${activeSession.metrics.costUsd.toFixed(4)}</span>
          <span>{activeSession.metrics.tokensIn + activeSession.metrics.tokensOut} tok</span>
        </div>
      )}

      {/* Input */}
      <div className="p-3 border-t border-border">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            handleSend();
          }}
          className="relative"
        >
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={activeSession ? "Send a message..." : "Select a workspace first..."}
            disabled={!activeSession}
            className="w-full px-3.5 py-2.5 bg-input-bg border border-border rounded-lg text-[13px] text-text-primary placeholder:text-text-ghost disabled:opacity-40 focus:outline-none focus:border-accent transition-colors"
          />
          <button
            type="submit"
            disabled={!activeSession || !input.trim()}
            className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded text-text-ghost hover:text-accent disabled:opacity-30 transition-colors"
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M14 2L7 9M14 2L10 14L7 9M14 2L2 6L7 9"/>
            </svg>
          </button>
        </form>
      </div>
    </aside>
  );
}

function MessageBubble({ message }: { message: ChatMessage }) {
  const isUser = message.role === "user";
  const isTool = message.role === "tool";

  if (isTool) {
    return (
      <div className="rounded-md bg-card-bg border border-border px-3 py-2">
        <div className="flex items-center gap-1.5 mb-1">
          <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-text-ghost">
            <rect x="2" y="2" width="12" height="12" rx="2"/>
            <path d="M6 6L10 10M10 6L6 10"/>
          </svg>
          <span className="text-[10px] font-mono text-text-ghost">{message.toolName}</span>
        </div>
        {!message.collapsed && (
          <pre className="text-[11px] text-text-tertiary whitespace-pre-wrap break-words max-h-32 overflow-auto">
            {message.content}
          </pre>
        )}
      </div>
    );
  }

  return (
    <div className={`rounded-lg px-3 py-2 text-[13px] leading-relaxed ${
      isUser
        ? "bg-accent/10 text-text-primary ml-6"
        : "bg-card-bg text-text-secondary mr-2"
    }`}>
      {message.content}
    </div>
  );
}
