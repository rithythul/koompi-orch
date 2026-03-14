interface TurnFile {
  path: string;
  status: string;
}

interface TurnDiffProps {
  turn: number;
  files: TurnFile[];
  onSelectFile: (filePath: string) => void;
  active?: boolean;
}

const STATUS_COLORS: Record<string, string> = {
  M: "text-yellow-400",
  A: "text-green-400",
  D: "text-red-400",
  R: "text-blue-400",
};

const STATUS_LABELS: Record<string, string> = {
  M: "Modified",
  A: "Added",
  D: "Deleted",
  R: "Renamed",
};

export function TurnDiff({
  turn,
  files,
  onSelectFile,
  active = false,
}: TurnDiffProps) {
  return (
    <div
      className={`border rounded-lg overflow-hidden ${
        active ? "border-blue-500/50" : "border-gray-700"
      }`}
    >
      <div
        className={`flex items-center justify-between px-3 py-2 ${
          active ? "bg-blue-500/10" : "bg-gray-800/50"
        }`}
      >
        <span className="text-sm font-medium text-gray-200">Turn {turn}</span>
        <span className="text-xs text-gray-500">
          {files.length} file{files.length !== 1 ? "s" : ""}
        </span>
      </div>

      <div className="divide-y divide-gray-800">
        {files.map((file) => (
          <button
            key={file.path}
            type="button"
            onClick={() => onSelectFile(file.path)}
            className="w-full text-left flex items-center gap-2 px-3 py-1.5 text-sm hover:bg-white/5 transition-colors"
          >
            <span
              className={`font-mono text-[10px] font-bold w-4 text-center ${
                STATUS_COLORS[file.status] ?? "text-gray-500"
              }`}
              title={STATUS_LABELS[file.status] ?? file.status}
            >
              {file.status}
            </span>
            <span className="font-mono text-gray-300 truncate">
              {file.path}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
