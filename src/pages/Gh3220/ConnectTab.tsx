import React, { useEffect, useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Select,
  Space,
  Tag,
  Typography,
  message,
} from 'antd';
import { ApiOutlined, DisconnectOutlined, ReloadOutlined, SearchOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3220Store } from '../../stores/gh3220Store';
import { bleApi } from '../../api/tauri';
import type { BleDeviceInfo } from '../../types/ble';
import type { Gh3220State } from '../../types/gh3220';
import { formatErrorMessage } from '../../utils/errorMessage';

const SERVICE_UUID = '0000190e-0000-1000-8000-00805f9b34fb';

const STATE_COLOR: Record<Gh3220State, string> = {
  disconnected: 'default',
  connecting: 'processing',
  ready: 'success',
  capturing: 'warning',
};

const ConnectTab: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const {
    status,
    deviceInfo,
    loading,
    error,
    connect,
    disconnect,
    loadDeviceInfo,
  } = useGh3220Store();

  const [devices, setDevices] = useState<BleDeviceInfo[]>([]);
  const [selected, setSelected] = useState<string>();
  const [scanning, setScanning] = useState(false);

  useEffect(() => {
    if (status.state === 'ready' && !deviceInfo) {
      void loadDeviceInfo();
    }
  }, [status.state, deviceInfo, loadDeviceInfo]);

  const options = useMemo(
    () =>
      devices.map((device) => ({
        value: device.address,
        label: `${device.name || t('connect.unknownDevice')} (${device.address})${
          device.rssi === undefined ? '' : ` ${device.rssi} dBm`
        }`,
      })),
    [devices, t]
  );

  const handleScan = async () => {
    setScanning(true);
    try {
      const list = await bleApi.scanBleDevices({ timeout: 6000 });
      const gh = list.filter(
        (device) =>
          (device.name || '').toUpperCase().includes('GH3220') ||
          (device.services || []).some((uuid) => uuid.toLowerCase() === SERVICE_UUID)
      );
      const result = gh.length > 0 ? gh : list;
      setDevices(result);
      if (result.length === 0) {
        message.warning(t('connect.noDevices'));
      }
    } catch (err) {
      message.error(formatErrorMessage(err, t('connect.scanFailed')));
    } finally {
      setScanning(false);
    }
  };

  const handleConnect = async () => {
    if (!selected) {
      message.warning(t('connect.selectDevice'));
      return;
    }
    try {
      await connect({ kind: 'ble', address: selected });
      message.success(t('connect.connected'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('connect.connectFailed')));
    }
  };

  const handleDisconnect = async () => {
    await disconnect();
    message.info(t('connect.disconnected'));
  };

  const connected = status.state !== 'disconnected' && status.state !== 'connecting';

  return (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
      {error && <Alert type="error" showIcon message={error} />}

      <Card
        title={t('connect.title')}
        extra={
          <Space>
            <Tag color={STATE_COLOR[status.state]}>{t(`state.${status.state}`)}</Tag>
            {status.link && <Tag>{status.link.toUpperCase()}</Tag>}
          </Space>
        }
      >
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Space.Compact style={{ width: '100%' }}>
            <Select
              style={{ flex: 1 }}
              placeholder={t('connect.placeholder')}
              value={selected}
              onChange={setSelected}
              options={options}
              disabled={connected || scanning}
              showSearch
              optionFilterProp="label"
            />
            <Button
              icon={<SearchOutlined />}
              loading={scanning}
              onClick={handleScan}
              disabled={connected}
            >
              {t('connect.scan')}
            </Button>
          </Space.Compact>

          <Space>
            <Button
              type="primary"
              icon={<ApiOutlined />}
              loading={loading}
              disabled={connected}
              onClick={handleConnect}
            >
              {t('connect.connect')}
            </Button>
            <Button
              danger
              icon={<DisconnectOutlined />}
              disabled={!connected}
              onClick={handleDisconnect}
            >
              {t('connect.disconnect')}
            </Button>
            <Button
              icon={<ReloadOutlined />}
              disabled={!connected}
              loading={loading}
              onClick={() => void loadDeviceInfo()}
            >
              {t('connect.refreshInfo')}
            </Button>
          </Space>

          <Descriptions size="small" column={2} bordered>
            <Descriptions.Item label={t('connect.address')}>
              {status.address || '--'}
            </Descriptions.Item>
            <Descriptions.Item label={t('connect.name')}>
              {status.name || '--'}
            </Descriptions.Item>
            <Descriptions.Item label={t('connect.chunkSize')}>
              {status.chunkSize > 0 ? `${status.chunkSize} B` : '--'}
            </Descriptions.Item>
            <Descriptions.Item label={t('connect.capturing')}>
              {status.capturing ? t('common.yes') : t('common.no')}
            </Descriptions.Item>
          </Descriptions>

          {status.error && (
            <Typography.Text type="danger">{status.error}</Typography.Text>
          )}
        </Space>
      </Card>

      <Card title={t('info.title')}>
        {deviceInfo ? (
          <Descriptions size="small" column={2} bordered>
            <Descriptions.Item label={t('info.fw')}>{deviceInfo.fwVersion || '--'}</Descriptions.Item>
            <Descriptions.Item label={t('info.protocol')}>
              {deviceInfo.protocolVersion || '--'}
            </Descriptions.Item>
            <Descriptions.Item label={t('info.chip')}>
              {deviceInfo.chipVersion || '--'}
            </Descriptions.Item>
            <Descriptions.Item label={t('info.driver')}>
              {deviceInfo.driverVersion || '--'}
            </Descriptions.Item>
            <Descriptions.Item label={t('info.maxLen')}>{deviceInfo.maxLen}</Descriptions.Item>
            <Descriptions.Item label={t('info.openedFunc')}>
              {`0x${(deviceInfo.openedFunc >>> 0).toString(16)}`}
            </Descriptions.Item>
            <Descriptions.Item label={t('info.link')}>
              <Tag color={deviceInfo.linkOk ? 'success' : 'error'}>
                {deviceInfo.linkOk ? t('info.linkOk') : t('info.linkDown')}
              </Tag>
            </Descriptions.Item>
          </Descriptions>
        ) : (
          <Typography.Text type="secondary">{t('info.empty')}</Typography.Text>
        )}
      </Card>
    </Space>
  );
};

export default ConnectTab;
