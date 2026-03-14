interface RightPanelProps {
  collapsed: boolean;
}

export function RightPanel({ collapsed }: RightPanelProps) {
  if (collapsed) return null;

  return (
    <aside className="w-[320px] bg-secondary/80 backdrop-blur-xl border-l border-border flex flex-col h-full">
      {/* Header */}
      <div className="h-[48px] px-4 flex items-center justify-between border-b border-border">
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-text-ghost" />
          <h2 className="text-[13px] font-medium text-text-primary">Agent Chat</h2>
        </div>
        <span className="text-[10px] font-mono text-text-ghost">offline</span>
      </div>

      {/* Empty state */}
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

      {/* Input */}
      <div className="p-3 border-t border-border">
        <div className="relative">
          <input
            type="text"
            placeholder="Send a message..."
            disabled
            className="w-full px-3.5 py-2.5 bg-input-bg border border-border rounded-lg text-[13px] text-text-primary placeholder:text-text-ghost disabled:opacity-40 focus:outline-none focus:border-accent transition-colors"
          />
          <button
            disabled
            className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded text-text-ghost disabled:opacity-30"
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M14 2L7 9M14 2L10 14L7 9M14 2L2 6L7 9"/>
            </svg>
          </button>
        </div>
      </div>
    </aside>
  );
}
