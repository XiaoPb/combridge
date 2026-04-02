import { useCallback, useRef } from 'react';
import { notification, message } from 'antd';

type NotificationType = 'success' | 'error' | 'warning' | 'info';

interface NotificationOptions {
  title?: string;
  duration?: number;
}

interface UseNotificationReturn {
  success: (content: string, options?: NotificationOptions) => void;
  error: (content: string, options?: NotificationOptions) => void;
  warning: (content: string, options?: NotificationOptions) => void;
  info: (content: string, options?: NotificationOptions) => void;
  toast: (content: string, type?: NotificationType) => void;
  destroyAll: () => void;
}

export const useNotification = (): UseNotificationReturn => {
  const [api, contextHolder] = notification.useNotification();

  const success = useCallback((content: string, options?: NotificationOptions) => {
    api.success({
      message: options?.title || '成功',
      description: content,
      duration: options?.duration || 3,
    });
  }, [api]);

  const error = useCallback((content: string, options?: NotificationOptions) => {
    api.error({
      message: options?.title || '错误',
      description: content,
      duration: options?.duration || 5,
    });
  }, [api]);

  const warning = useCallback((content: string, options?: NotificationOptions) => {
    api.warning({
      message: options?.title || '警告',
      description: content,
      duration: options?.duration || 4,
    });
  }, [api]);

  const info = useCallback((content: string, options?: NotificationOptions) => {
    api.info({
      message: options?.title || '信息',
      description: content,
      duration: options?.duration || 3,
    });
  }, [api]);

  const toast = useCallback((content: string, type: NotificationType = 'info') => {
    message[type](content);
  }, []);

  const destroyAll = useCallback(() => {
    api.destroy();
  }, [api]);

  return {
    success,
    error,
    warning,
    info,
    toast,
    destroyAll,
    contextHolder,
  };
};

export default useNotification;
