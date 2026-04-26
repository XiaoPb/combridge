import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Modal, Table, Button, Space, Tag, Progress, Typography, Empty, message } from 'antd';
import { SearchOutlined, StopOutlined, HeartOutlined, CheckCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { bleApi } from '../../../api/tauri';
import { BLE_SERVICE_UUID } from '../../../types/ble';
import type { BleDeviceInfo } from '../../../types';

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
const SCAN_INTERVAL_MS = 1000;
const MAX_SCAN_TIME_MS = 30000;

const HrRefDeviceDialog: React.FC<HrRefDeviceDialogProps> = ({
  open,
  onSelect,
  onCancel,
  isMonitoring = false,
  currentHrValue = null,
  collectedCount = 0,
}) => {
  const { t } = useTranslation('gh3036');
  const [devices, setDevices] = useState<BleDeviceInfo[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [connecting, setConnecting] = useState<string | null>(null);
  const [selectedDevice, setSelectedDevice] = useState<BleDeviceInfo | null>(null);
  const [scanProgress, setScanProgress] = useState(0);
  
  const scanAbortRef = useRef<boolean>(false);
  const scanTimerRef = useRef<NodeJS.Timeout | null>(null);

  const filterHrDevices = useCallback((scannedDevices: BleDeviceInfo[]): BleDeviceInfo[] => {
    return scannedDevices.filter((device) => {
      if (device.services && device.services.length > 0) {
        return device.services.some(
          (svc) => svc.toLowerCase().includes(HEART_RATE_SERVICE_UUID.replace(/-/g, ''))
        );
      }
      return false;
    });
  }, []);

  const handleScan = useCallback(async () => {
    setIsScanning(true);
    setDevices([]);
    setScanProgress(0);
    scanAbortRef.current = false;
    
    const startTime = Date.now();
    
    try {
      while (!scanAbortRef.current && (Date.now() - startTime) < MAX_SCAN_TIME_MS) {
        const scannedDevices = await bleApi.scanBleDevices({ timeout: SCAN_INTERVAL_MS });
        
        if (scanAbortRef.current) break;
        
        const hrDevices = filterHrDevices(scannedDevices);
        setDevices(prev => {
          const merged = new Map<string, BleDeviceInfo>();
          prev.forEach(d => merged.set(d.address, d));
          hrDevices.forEach(d => merged.set(d.address, d));
          return Array.from(merged.values());
        });
        
        const elapsed = Date.now() - startTime;
        setScanProgress(Math.min(100, (elapsed / MAX_SCAN_TIME_MS) * 100));
      }
    } catch (err) {
      if (!scanAbortRef.current) {
        const errorMsg = err instanceof Error ? err.message : t('monitor.hrRefScanFailed');
        message.error(errorMsg);
      }
    } finally {
      setIsScanning(false);
    }
  }, [filterHrDevices, t]);

  const handleStopScan = useCallback(() => {
    scanAbortRef.current = true;
    if (scanTimerRef.current) {
      clearTimeout(scanTimerRef.current);
      scanTimerRef.current = null;
    }
    setIsScanning(false);
  }, []);

  const handleSelectDevice = useCallback(async (device: BleDeviceInfo) => {
    setSelectedDevice(device);
    setConnecting(device.address);
    try {
      await onSelect(device.address);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : t('monitor.hrRefConnectFailed');
      message.error(errorMsg);
      setSelectedDevice(null);
    } finally {
      setConnecting(null);
    }
  }, [onSelect, t]);

  const getRssiColor = (rssi?: number): string => {
    if (!rssi) return 'default';
    if (rssi >= -50) return 'green';
    if (rssi >= -70) return 'blue';
    if (rssi >= -90) return 'orange';
    return 'red';
  };

  const columns = [
    {
      title: t('monitor.hrRefDeviceName'),
      dataIndex: 'name',
      key: 'name',
      render: (name: string) => name || <Text type="secondary">{t('monitor.hrRefUnnamed')}</Text>,
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
      title: t('monitor.hrRefRssi'),
      dataIndex: 'rssi',
      key: 'rssi',
      width: 120,
      render: (rssi?: number) =>
        rssi ? (
          <Tag color={getRssiColor(rssi)}>{rssi} dBm</Tag>
        ) : (
          <Text type="secondary">-</Text>
        ),
    },
    {
      title: t('monitor.hrRefAction'),
      key: 'action',
      width: 120,
      render: (_: unknown, record: BleDeviceInfo) => {
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
    if (open && devices.length === 0 && !isScanning) {
      handleScan();
    }
    if (!open) {
      scanAbortRef.current = true;
      setIsScanning(false);
    }
  }, [open, devices.length, isScanning, handleScan]);

  useEffect(() => {
    return () => {
      scanAbortRef.current = true;
      if (scanTimerRef.current) {
        clearTimeout(scanTimerRef.current);
      }
    };
  }, []);

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
      width={600}
      maskClosable={!isMonitoring}
      closable={!isMonitoring}
    >
      <div style={{ marginBottom: 16 }}>
        <Text type="secondary">{t('monitor.hrRefHint')}</Text>
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
        <Space>
          {isScanning ? (
            <Button
              type="primary"
              danger
              icon={<StopOutlined />}
              onClick={handleStopScan}
            >
              {t('monitor.hrRefStopScan')}
            </Button>
          ) : (
            <Button
              type="primary"
              icon={<SearchOutlined />}
              onClick={handleScan}
              disabled={isMonitoring}
            >
              {t('monitor.hrRefScan')}
            </Button>
          )}
        </Space>
      </div>

      {isScanning && (
        <div style={{ marginBottom: 16 }}>
          <Progress percent={Math.round(scanProgress)} status="active" showInfo={false} />
          <Text type="secondary">{t('monitor.hrRefScanning')} ({Math.round(scanProgress)}%)</Text>
        </div>
      )}

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
          description={
            isScanning ? t('monitor.hrRefScanning') : t('monitor.hrRefNoDevices')
          }
          style={{ padding: '40px 0' }}
        />
      )}

      <div style={{ marginTop: 16, display: 'flex', justifyContent: 'flex-end' }}>
        <Button onClick={onCancel} disabled={isMonitoring}>
          {t('common:close')}
        </Button>
      </div>
    </Modal>
  );
};

export default HrRefDeviceDialog;
