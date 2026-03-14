interface RightPanelProps {
  collapsed: boolean;
}

export function RightPanel({ collapsed }: RightPanelProps) {
  if (collapsed) return null;

  return (
    <aside className="w-80 bg-secondary border-l border-border flex flex-col h-full">
      <div className="p-3 border-b border-border">
        <h2 className="text-sm font-medium text-text-primary">Agent Chat</h2>
      </div>

      <div className="flex-1 p-4 flex items-center justify-center">
        <p className="text-sm text-text-secondary text-center">
          Select a workspace to view agent sessions
        </p>
      </div>

      <div className="p-3 border-t border-border">
        <input
          type="text"
          placeholder="Send a message..."
          disabled
          className="w-full px-3 py-2 bg-tertiary border border-border rounded text-sm text-text-primary placeholder:text-text-secondary disabled:opacity-50"
        />
      </div>
    </aside>
  );
}
