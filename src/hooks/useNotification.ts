import { useCallback } from 'react';
import { notification, message } from 'antd';
import { useTranslation } from 'react-i18next';

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
  contextHolder: React.ReactNode;
}

export const useNotification = (): UseNotificationReturn => {
  const { t } = useTranslation('common');
  const [api, contextHolder] = notification.useNotification();

  const success = useCallback((content: string, options?: NotificationOptions) => {
    api.success({
      message: options?.title || t('common.success'),
      description: content,
      duration: options?.duration || 3,
    });
  }, [api, t]);

  const error = useCallback((content: string, options?: NotificationOptions) => {
    api.error({
      message: options?.title || t('common.error'),
      description: content,
      duration: options?.duration || 5,
    });
  }, [api, t]);

  const warning = useCallback((content: string, options?: NotificationOptions) => {
    api.warning({
      message: options?.title || t('common.warning'),
      description: content,
      duration: options?.duration || 4,
    });
  }, [api, t]);

  const info = useCallback((content: string, options?: NotificationOptions) => {
    api.info({
      message: options?.title || t('common.info'),
      description: content,
      duration: options?.duration || 3,
    });
  }, [api, t]);

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
