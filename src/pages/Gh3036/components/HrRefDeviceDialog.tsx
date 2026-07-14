import React, { useState, useEffect, useCallback } from 'react';
import { Modal, Table, Button, Space, Tag, Progress, Typography, Empty, message, Spin } from 'antd';
import { HeartOutlined, CheckCircleOutlined, ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { bleApi } from '../../../api/tauri';
import { BLE_SERVICE_UUID } from '../../../types/ble';
import type { BleConnection } from '../../../types';
import { formatErrorMessage } from '../../../utils/errorMessage';

const { Text } = Typography;

interface HrRefDeviceDialogProps {
  open: boolean;
  onSelect: (deviceAddress: string) => Promise<void>;
  onCancel: () => void;
  isMonitoring?: boolean;
  currentHrValue?: number | null;
  collectedCount?: number;
}

const HEART_RATE_SERVICE_UUID = BLE_SERVICE_UUID.HEART_RATE.toLowerCase();

const HrRefDeviceDialog: React.FC<HrRefDeviceDialogProps> = ({
  open,
  onSelect,
  onCancel,
  isMonitoring = false,
  currentHrValue = null,
  collectedCount = 0,
}) => {
  const { t } = useTranslation('gh3036');
  const [devices, setDevices] = useState<BleConnection[]>([]);
  const [loading, setLoading] = useState(false);
  const [connecting, setConnecting] = useState<string | null>(null);
  const [selectedDevice, setSelectedDevice] = useState<BleConnection | null>(null);

  const filterHrDevices = useCallback((connections: BleConnection[]): BleConnection[] => {
    return connections.filter((conn) => {
      if (conn.services && conn.services.length > 0) {
        return conn.services.some(
          (svc) => svc.uuid.toLowerCase().includes(HEART_RATE_SERVICE_UUID.replace(/-/g, ''))
        );
      }
      return false;
    });
  }, []);

  const loadConnectedDevices = useCallback(async () => {
    setLoading(true);
    try {
      const connections = await bleApi.getConnections();
      const hrDevices = filterHrDevices(connections);
      setDevices(hrDevices);
    } catch (err) {
      const errorMsg = formatErrorMessage(err, t('monitor.hrRefLoadFailed'));
      message.error(errorMsg);
    } finally {
      setLoading(false);
    }
  }, [filterHrDevices, t]);

  const handleSelectDevice = useCallback(async (device: BleConnection) => {
    setSelectedDevice(device);
    setConnecting(device.address);
    try {
      await onSelect(device.address);
    } catch (err) {
      const errorMsg = formatErrorMessage(err, t('monitor.hrRefConnectFailed'));
      message.error(errorMsg);
      setSelectedDevice(null);
    } finally {
      setConnecting(null);
    }
  }, [onSelect, t]);

  const columns = [
    {
      title: t('monitor.hrRefDeviceName'),
      dataIndex: 'name',
      key: 'name',
      render: (name?: string) => name || <Text type="secondary">{t('monitor.hrRefUnnamed')}</Text>,
    },
    {
      title: t('monitor.hrRefDeviceAddress'),
      dataIndex: 'address',
      key: 'address',
      width: 180,
      render: (address: string) => (
        <Text code style={{ fontSize: '12px' }}>
          {address}
        </Text>
      ),
    },
    {
      title: t('monitor.hrRefAction'),
      key: 'action',
      width: 120,
      render: (_: unknown, record: BleConnection) => {
        const isSelected = selectedDevice?.address === record.address;
        const isConnecting = connecting === record.address;
        return (
          <Button
            type={isSelected ? 'default' : 'primary'}
            size="small"
            icon={isSelected ? <CheckCircleOutlined /> : undefined}
            loading={isConnecting}
            disabled={isMonitoring || isConnecting}
            onClick={() => handleSelectDevice(record)}
          >
            {isSelected ? t('monitor.hrRefSelected') : t('monitor.hrRefSelect')}
          </Button>
        );
      },
    },
  ];

  useEffect(() => {
    if (open) {
      loadConnectedDevices();
    }
  }, [open, loadConnectedDevices]);

  return (
    <Modal
      open={open}
      title={
        <Space>
          <HeartOutlined style={{ color: '#ff4d4f' }} />
          <span>{t('monitor.hrRefConfig')}</span>
        </Space>
      }
      onCancel={onCancel}
      footer={null}
      width={500}
      maskClosable={!isMonitoring}
      closable={!isMonitoring}
    >
      <div style={{ marginBottom: 16 }}>
        <Text type="secondary">{t('monitor.hrRefHintConnected')}</Text>
      </div>

      {isMonitoring && (
        <div style={{ marginBottom: 16, padding: 12, background: '#f5f5f5', borderRadius: 8 }}>
          <Space direction="vertical" style={{ width: '100%' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Text strong>{t('monitor.hrRefMonitoring')}</Text>
              <Tag color="processing">{t('monitor.hrRefCollecting')}</Tag>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Text>{t('monitor.hrRefCurrentHr')}</Text>
              <Text strong style={{ fontSize: 20, color: '#ff4d4f' }}>
                {currentHrValue !== null ? `${currentHrValue} bpm` : '--'}
              </Text>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Text>{t('monitor.hrRefCollected')}</Text>
              <Text>{collectedCount} / 4</Text>
            </div>
            <Progress
              percent={(collectedCount / 4) * 100}
              size="small"
              status={collectedCount >= 4 ? 'success' : 'active'}
            />
          </Space>
        </div>
      )}

      <div style={{ marginBottom: 16 }}>
        <Button
          icon={<ReloadOutlined />}
          onClick={loadConnectedDevices}
          disabled={loading || isMonitoring}
          loading={loading}
        >
          {t('monitor.hrRefRefresh')}
        </Button>
      </div>

      <Spin spinning={loading}>
        {devices.length > 0 ? (
          <Table
            dataSource={devices}
            columns={columns}
            rowKey="address"
            size="small"
            pagination={false}
            style={{ maxHeight: 300, overflow: 'auto' }}
          />
        ) : (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t('monitor.hrRefNoConnectedDevices')}
            style={{ padding: '40px 0' }}
          />
        )}
      </Spin>

      <div style={{ marginTop: 16, display: 'flex', justifyContent: 'flex-end' }}>
        <Button onClick={onCancel} disabled={isMonitoring}>
          {t('common:close')}
        </Button>
      </div>
    </Modal>
  );
};

export default HrRefDeviceDialog;
