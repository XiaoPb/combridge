import { create } from 'zustand';

export type NotificationType = 'success' | 'error' | 'info' | 'warning';

interface Notification {
  id: string;
  type: NotificationType;
  content: string;
  timestamp: number;
}

interface NotificationState {
  notifications: Notification[];
  addNotification: (type: NotificationType, content: string) => void;
  consumeNotifications: () => Notification[];
}

export const useNotificationStore = create<NotificationState>((set, get) => ({
  notifications: [],

  addNotification: (type, content) => {
    set((state) => ({
      notifications: [
        ...state.notifications,
        {
          id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
          type,
          content,
          timestamp: Date.now(),
        },
      ],
    }));
  },

  consumeNotifications: () => {
    const { notifications } = get();
    set({ notifications: [] });
    return notifications;
  },
}));
