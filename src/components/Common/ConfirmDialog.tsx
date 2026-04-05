import React from 'react';
import { Modal, Button, Space, Typography } from 'antd';
import { ExclamationCircleOutlined, WarningOutlined, InfoCircleOutlined, CheckCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';

const { Text } = Typography;

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  content: string | React.ReactNode;
  type?: 'info' | 'warning' | 'error' | 'success';
  confirmText?: string;
  cancelText?: string;
  onConfirm: () => void;
  onCancel: () => void;
  loading?: boolean;
  danger?: boolean;
}

const typeConfig = {
  info: {
    icon: <InfoCircleOutlined style={{ color: '#1677ff' }} />,
    okType: 'primary' as const,
  },
  warning: {
    icon: <WarningOutlined style={{ color: '#faad14' }} />,
    okType: 'primary' as const,
  },
  error: {
    icon: <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />,
    okType: 'primary' as const,
  },
  success: {
    icon: <CheckCircleOutlined style={{ color: '#52c41a' }} />,
    okType: 'primary' as const,
  },
};

const ConfirmDialog: React.FC<ConfirmDialogProps> = ({
  open,
  title,
  content,
  type = 'info',
  confirmText,
  cancelText,
  onConfirm,
  onCancel,
  loading = false,
  danger = false,
}) => {
  const { t } = useTranslation('common');
  const config = typeConfig[type];

  return (
    <Modal
      open={open}
      title={
        <Space>
          {config.icon}
          <span>{title}</span>
        </Space>
      }
      onCancel={onCancel}
      footer={
        <Space>
          <Button onClick={onCancel} disabled={loading}>
            {cancelText || t('cancel')}
          </Button>
          <Button
            type={danger ? 'primary' : config.okType}
            danger={danger}
            loading={loading}
            onClick={onConfirm}
          >
            {confirmText || t('confirm')}
          </Button>
        </Space>
      }
      centered
      maskClosable={!loading}
      closable={!loading}
    >
      {typeof content === 'string' ? <Text>{content}</Text> : content}
    </Modal>
  );
};

export default ConfirmDialog;
