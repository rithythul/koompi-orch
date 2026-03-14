import type { Notification } from "../../stores/notificationStore";

interface ToastProps {
  notification: Notification;
  onDismiss: (id: string) => void;
}

const TYPE_STYLES: Record<string, { border: string; icon: string; iconColor: string }> = {
  info: {
    border: "border-blue-500/50",
    icon: "i",
    iconColor: "text-blue-400 bg-blue-500/20",
  },
  success: {
    border: "border-green-500/50",
    icon: "\u2713",
    iconColor: "text-green-400 bg-green-500/20",
  },
  warning: {
    border: "border-yellow-500/50",
    icon: "!",
    iconColor: "text-yellow-400 bg-yellow-500/20",
  },
  error: {
    border: "border-red-500/50",
    icon: "\u2717",
    iconColor: "text-red-400 bg-red-500/20",
  },
};

export function Toast({ notification, onDismiss }: ToastProps) {
  const style = TYPE_STYLES[notification.type] ?? TYPE_STYLES.info;

  return (
    <div
      data-testid="toast"
      className={`flex items-start gap-3 px-4 py-3 bg-gray-800 border ${style.border} rounded-lg shadow-xl min-w-[300px] max-w-[420px] animate-slide-in`}
    >
      <span
        className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold flex-shrink-0 ${style.iconColor}`}
      >
        {style.icon}
      </span>

      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium text-gray-200">
          {notification.title}
        </div>
        <div className="text-xs text-gray-400 mt-0.5 leading-relaxed">
          {notification.message}
        </div>
      </div>

      <button
        type="button"
        title="Dismiss"
        onClick={() => onDismiss(notification.id)}
        className="text-gray-600 hover:text-gray-300 text-sm leading-none flex-shrink-0 mt-0.5"
      >
        &times;
      </button>
    </div>
  );
}
