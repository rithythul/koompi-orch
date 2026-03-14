import { useEffect, useRef } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useAgentStore, type ChatMessage } from "../../stores/agentStore";

/** Render a single tool-use block (collapsible) */
function ToolUseBlock({
  message,
  sessionId,
}: {
  message: ChatMessage;
  sessionId: string;
}) {
  const toggleCollapse = useAgentStore((s) => s.toggleToolCollapse);

  return (
    <div className="border border-gray-700 rounded-md overflow-hidden my-1">
      <button
        type="button"
        onClick={() => toggleCollapse(sessionId, message.id)}
        className="w-full flex items-center gap-2 px-3 py-1.5 bg-gray-800 hover:bg-gray-750 text-left text-sm"
      >
        <span className="text-gray-500 text-xs">
          {message.collapsed ? "\u25B8" : "\u25BE"}
        </span>
        <span className="text-blue-400 font-mono text-xs">
          {message.toolName ?? "Tool"}
        </span>
        {message.collapsed && (
          <span className="text-gray-500 text-xs truncate flex-1">
            {message.content.slice(0, 80)}
            {message.content.length > 80 ? "..." : ""}
          </span>
        )}
      </button>
      {!message.collapsed && (
        <div className="px-3 py-2 bg-gray-900/50 text-xs font-mono text-gray-300 whitespace-pre-wrap overflow-x-auto max-h-64 overflow-y-auto">
          {message.content}
        </div>
      )}
    </div>
  );
}

/** Render a single chat message bubble */
function MessageBubble({
  message,
  sessionId,
}: {
  message: ChatMessage;
  sessionId: string;
}) {
  if (message.role === "tool") {
    return <ToolUseBlock message={message} sessionId={sessionId} />;
  }

  const isUser = message.role === "user";

  return (
    <div
      className={`flex ${isUser ? "justify-end" : "justify-start"} mb-3`}
    >
      <div
        className={`
          max-w-[85%] rounded-lg px-4 py-2.5 text-sm
          ${isUser ? "bg-blue-600 text-white" : "bg-gray-800 text-gray-200"}
        `}
      >
        {isUser ? (
          <p className="whitespace-pre-wrap">{message.content}</p>
        ) : (
          <div className="prose prose-sm prose-invert max-w-none [&_pre]:bg-gray-900 [&_pre]:rounded-md [&_pre]:p-3 [&_code]:text-blue-300 [&_a]:text-blue-400">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>
              {message.content}
            </ReactMarkdown>
          </div>
        )}
        <div className="text-[10px] mt-1 opacity-50 text-right">
          {new Date(message.timestamp).toLocaleTimeString()}
        </div>
      </div>
    </div>
  );
}

interface ChatViewProps {
  sessionId: string;
}

export function ChatView({ sessionId }: ChatViewProps) {
  const session = useAgentStore((s) => s.sessions[sessionId]);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [session?.messages.length]);

  if (!session) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500 text-sm">
        No active session. Create a workspace and start an agent.
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-3 px-4 py-2 border-b border-gray-700">
        <span className="text-sm font-medium text-gray-200">
          {session.agentType}
        </span>
        {session.model && (
          <span className="text-xs text-gray-500 font-mono">
            {session.model}
          </span>
        )}
        {session.rolePreset && (
          <span className="text-xs px-1.5 py-0.5 bg-purple-500/20 text-purple-300 rounded">
            {session.rolePreset}
          </span>
        )}
        <div className="flex-1" />
        <AgentStatusDot status={session.status} />
      </div>

      {/* Messages */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto px-4 py-3 space-y-1"
      >
        {session.messages.length === 0 ? (
          <div className="text-center text-gray-500 text-sm mt-8">
            Waiting for agent output...
          </div>
        ) : (
          session.messages.map((msg) => (
            <MessageBubble
              key={msg.id}
              message={msg}
              sessionId={sessionId}
            />
          ))
        )}
      </div>
    </div>
  );
}

/** Small colored dot indicating agent status */
function AgentStatusDot({
  status,
}: {
  status: string;
}) {
  const colors: Record<string, string> = {
    running: "bg-green-500 animate-pulse",
    paused: "bg-yellow-500",
    completed: "bg-gray-500",
    crashed: "bg-red-500",
  };

  const labels: Record<string, string> = {
    running: "Running",
    paused: "Paused",
    completed: "Completed",
    crashed: "Crashed",
  };

  return (
    <div className="flex items-center gap-1.5">
      <span className={`w-2 h-2 rounded-full ${colors[status] ?? "bg-gray-500"}`} />
      <span className="text-xs text-gray-400">
        {labels[status] ?? status}
      </span>
    </div>
  );
}
