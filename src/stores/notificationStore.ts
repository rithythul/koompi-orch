import { create } from "zustand";

export type NotificationType = "info" | "success" | "warning" | "error";

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  /** Workspace ID if notification is workspace-specific */
  workspaceId?: string;
  /** Auto-dismiss after this many ms (0 = manual dismiss) */
  autoCloseMs: number;
  createdAt: number;
}

interface NotificationState {
  notifications: Notification[];
  maxVisible: number;

  // Actions
  addNotification: (
    notification: Omit<Notification, "id" | "createdAt">
  ) => string;
  dismissNotification: (id: string) => void;
  clearAll: () => void;

  // Derived
  visibleNotifications: () => Notification[];
}

let notificationCounter = 0;

export const useNotificationStore = create<NotificationState>((set, get) => ({
  notifications: [],
  maxVisible: 5,

  addNotification: (notification) => {
    const id = `notif-${++notificationCounter}-${Date.now()}`;
    const full: Notification = {
      ...notification,
      id,
      createdAt: Date.now(),
    };
    set((state) => ({
      notifications: [...state.notifications, full],
    }));

    // Auto-dismiss
    if (notification.autoCloseMs > 0) {
      setTimeout(() => {
        get().dismissNotification(id);
      }, notification.autoCloseMs);
    }

    return id;
  },

  dismissNotification: (id) =>
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    })),

  clearAll: () => set({ notifications: [] }),

  visibleNotifications: () => {
    const { notifications, maxVisible } = get();
    return notifications.slice(-maxVisible);
  },
}));
