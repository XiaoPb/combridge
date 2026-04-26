import React, { useState, useEffect } from 'react';
import { Modal, InputNumber, Button, Space, Typography, message } from 'antd';
import { MinusOutlined, PlusOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';

const { Text } = Typography;

interface Spo2RefInputDialogProps {
  open: boolean;
  initialValue?: number;
  onConfirm: (value: number) => Promise<void>;
  onCancel: () => void;
}

const QUICK_ADJUSTMENTS = [
  { label: '-5', value: -5 },
  { label: '-2', value: -2 },
  { label: '-1', value: -1 },
  { label: '+1', value: 1 },
  { label: '+2', value: 2 },
  { label: '+5', value: 5 },
];

const Spo2RefInputDialog: React.FC<Spo2RefInputDialogProps> = ({
  open,
  initialValue = 95,
  onConfirm,
  onCancel,
}) => {
  const { t } = useTranslation('gh3036');
  const [value, setValue] = useState<number>(initialValue);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (open) {
      setValue(initialValue);
    }
  }, [open, initialValue]);

  const handleQuickAdjust = (adjustment: number) => {
    setValue((prev) => {
      const newValue = prev + adjustment;
      return Math.max(0, Math.min(100, newValue));
    });
  };

  const handleConfirm = async () => {
    if (value < 0 || value > 100) {
      message.error(t('monitor.spo2RefRangeError'));
      return;
    }
    
    setLoading(true);
    try {
      await onConfirm(value);
      message.success(t('monitor.spo2RefSetSuccess'));
      onCancel();
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : t('monitor.spo2RefSetFailed');
      message.error(errorMsg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      open={open}
      title={t('monitor.spo2RefConfig')}
      onCancel={onCancel}
      footer={null}
      width={320}
      maskClosable={!loading}
      closable={!loading}
    >
      <div style={{ marginBottom: 16 }}>
        <Text type="secondary">{t('monitor.spo2RefHint')}</Text>
      </div>
      
      <div style={{ marginBottom: 16 }}>
        <InputNumber
          min={0}
          max={100}
          value={value}
          onChange={(v) => setValue(v ?? 0)}
          style={{ width: '100%' }}
          size="large"
          addonAfter="%"
          disabled={loading}
        />
      </div>

      <div style={{ marginBottom: 16 }}>
        <Text type="secondary" style={{ fontSize: 12 }}>{t('monitor.quickAdjust')}</Text>
      </div>
      
      <Space wrap style={{ marginBottom: 24 }}>
        {QUICK_ADJUSTMENTS.map((adj) => (
          <Button
            key={adj.label}
            size="small"
            icon={adj.value < 0 ? <MinusOutlined /> : <PlusOutlined />}
            onClick={() => handleQuickAdjust(adj.value)}
            disabled={loading}
          >
            {adj.label}
          </Button>
        ))}
      </Space>

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
        <Button onClick={onCancel} disabled={loading}>
          {t('common:cancel')}
        </Button>
        <Button type="primary" onClick={handleConfirm} loading={loading}>
          {t('common:confirm')}
        </Button>
      </div>
    </Modal>
  );
};

export default Spo2RefInputDialog;
