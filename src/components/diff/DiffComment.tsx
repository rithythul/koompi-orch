interface DiffCommentProps {
  author: string;
  content: string;
  lineNumber: number;
  timestamp: string;
  resolved?: boolean;
  onResolve?: () => void;
}

export function DiffComment({
  author,
  content,
  lineNumber,
  timestamp,
  resolved = false,
  onResolve,
}: DiffCommentProps) {
  const formattedTime = new Date(timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  return (
    <div
      className={`border rounded-lg p-3 text-sm ${
        resolved
          ? "border-green-800/50 bg-green-900/10"
          : "border-gray-700 bg-gray-800/50"
      }`}
    >
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center gap-2">
          <span className="font-medium text-gray-200">{author}</span>
          <span className="text-[10px] font-mono text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">
            L{lineNumber}
          </span>
          {resolved && (
            <span
              data-testid="comment-resolved-badge"
              className="text-[10px] font-semibold text-green-400 bg-green-900/30 px-1.5 py-0.5 rounded"
            >
              Resolved
            </span>
          )}
        </div>
        <span className="text-[10px] text-gray-600">{formattedTime}</span>
      </div>
      <p className="text-gray-300 leading-relaxed">{content}</p>
      {!resolved && onResolve && (
        <button
          type="button"
          onClick={onResolve}
          className="mt-2 text-xs text-green-400 hover:text-green-300"
        >
          Mark resolved
        </button>
      )}
    </div>
  );
}
