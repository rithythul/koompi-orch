import { useNotificationStore } from "../../stores/notificationStore";
import { Toast } from "./Toast";

export function ToastContainer() {
  const visibleNotifications = useNotificationStore(
    (s) => s.visibleNotifications
  );
  const dismissNotification = useNotificationStore(
    (s) => s.dismissNotification
  );

  const toasts = visibleNotifications();

  return (
    <div className="fixed bottom-4 right-4 z-[200] flex flex-col-reverse gap-2 pointer-events-none">
      {toasts.map((notification) => (
        <div key={notification.id} className="pointer-events-auto">
          <Toast notification={notification} onDismiss={dismissNotification} />
        </div>
      ))}
    </div>
  );
}
