import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAgentStore } from "../stores/agentStore";

interface AgentEventPayload {
  sessionId: string;
  event: {
    type: "text" | "tool_use" | "tool_result" | "usage" | "error" | "completed" | "needs_input";
    content?: string;
    name?: string;
    input?: unknown;
    output?: string;
    tokens_in?: number;
    tokens_out?: number;
    cost_usd?: number;
    message?: string;
  };
}

let listenerSetup = false;

/**
 * Hook that subscribes to Tauri "agent-event" events and routes them
 * into the agentStore. Should be mounted once at the app root.
 */
export function useAgentEvents() {
  const appendMessage = useAgentStore((s) => s.appendMessage);
  const updateSessionStatus = useAgentStore((s) => s.updateSessionStatus);
  const updateMetrics = useAgentStore((s) => s.updateMetrics);

  useEffect(() => {
    if (listenerSetup) return;
    listenerSetup = true;

    let unlisten: (() => void) | undefined;

    listen<AgentEventPayload>("agent-event", (tauriEvent) => {
      const { sessionId, event } = tauriEvent.payload;
      const now = new Date().toISOString();
      const msgId = `${sessionId}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

      switch (event.type) {
        case "text":
          appendMessage(sessionId, {
            id: msgId,
            role: "assistant",
            content: event.content ?? "",
            turn: 0,
            timestamp: now,
          });
          break;

        case "tool_use":
          appendMessage(sessionId, {
            id: msgId,
            role: "tool",
            content: JSON.stringify(event.input, null, 2),
            turn: 0,
            timestamp: now,
            toolName: event.name,
            collapsed: true,
          });
          break;

        case "tool_result":
          appendMessage(sessionId, {
            id: msgId,
            role: "tool",
            content: event.output ?? "",
            turn: 0,
            timestamp: now,
            toolName: `${event.name} (result)`,
            collapsed: true,
          });
          break;

        case "usage":
          updateMetrics(sessionId, {
            tokensIn: event.tokens_in ?? 0,
            tokensOut: event.tokens_out ?? 0,
            costUsd: event.cost_usd ?? 0,
          });
          break;

        case "error":
          appendMessage(sessionId, {
            id: msgId,
            role: "assistant",
            content: `Error: ${event.message ?? "unknown error"}`,
            turn: 0,
            timestamp: now,
          });
          updateSessionStatus(sessionId, "crashed");
          break;

        case "completed":
          updateSessionStatus(sessionId, "completed");
          break;

        case "needs_input":
          // Session is waiting for user input — no status change needed
          break;
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      listenerSetup = false;
      unlisten?.();
    };
  }, [appendMessage, updateSessionStatus, updateMetrics]);
}
