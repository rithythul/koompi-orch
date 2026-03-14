type Theme = "dark" | "light";

interface ThemeToggleProps {
  theme: Theme;
  onToggle: (theme: Theme) => void;
}

export function ThemeToggle({ theme, onToggle }: ThemeToggleProps) {
  return (
    <div className="flex items-center gap-1 bg-gray-800 rounded-lg p-1">
      <button
        type="button"
        onClick={() => onToggle("dark")}
        className={`px-3 py-1.5 text-xs font-medium rounded-md transition-colors ${
          theme === "dark"
            ? "bg-blue-500 text-white"
            : "text-gray-400 hover:text-gray-200"
        }`}
      >
        Dark
      </button>
      <button
        type="button"
        onClick={() => onToggle("light")}
        className={`px-3 py-1.5 text-xs font-medium rounded-md transition-colors ${
          theme === "light"
            ? "bg-blue-500 text-white"
            : "text-gray-400 hover:text-gray-200"
        }`}
      >
        Light
      </button>
    </div>
  );
}
