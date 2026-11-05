import React, { useEffect, useMemo, useState } from 'react';
import {
  Button,
  Collapse,
  Descriptions,
  Input,
  InputNumber,
  Select,
  Space,
  Switch,
  Tag,
  Typography,
  message,
} from 'antd';
import {
  ApiOutlined,
  CodeOutlined,
  DisconnectOutlined,
  ReloadOutlined,
  SearchOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3220Store } from '../../stores/gh3220Store';
import { bleApi } from '../../api/tauri';
import type { BleDeviceInfo } from '../../types/ble';
import { formatErrorMessage } from '../../utils/errorMessage';
import DebugChannelCard from './DebugChannelCard';

const SERVICE_UUID = '0000190e-0000-1000-8000-00805f9b34fb';

const STATE_COLOR: Record<string, string> = {
  disconnected: 'default',
  connecting: 'processing',
  ready: 'success',
  capturing: 'warning',
};

/** 连接配置区块（原 ConnectTab 内容） */
const ConnectionSection: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const { status, connect, disconnect, loadDeviceInfo } = useGh3220Store();

  const [devices, setDevices] = useState<BleDeviceInfo[]>([]);
  const [selected, setSelected] = useState<string>();
  const [scanning, setScanning] = useState(false);
  const [loading, setLoading] = useState(false);

  const connected = status.state !== 'disconnected' && status.state !== 'connecting';

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
    setLoading(true);
    try {
      await connect({ kind: 'ble', address: selected });
      message.success(t('connect.connected'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('connect.connectFailed')));
    } finally {
      setLoading(false);
    }
  };

  const handleDisconnect = async () => {
    await disconnect();
    message.info(t('connect.disconnected'));
  };

  return (
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
          <Tag color={STATE_COLOR[status.state] ?? 'default'}>
            {t(`state.${status.state}`)}
          </Tag>
        </Descriptions.Item>
      </Descriptions>
    </Space>
  );
};

/** 采集与 CSV 偏好区块（原 CaptureTab 偏好卡片） */
const CapturePreferenceSection: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const { preferences, savePreferences } = useGh3220Store();

  const [csvEnabled, setCsvEnabled] = useState(true);
  const [outputDir, setOutputDir] = useState('');
  const [idleCloseSecs, setIdleCloseSecs] = useState(10);
  const [channelsOverride, setChannelsOverride] = useState<number | null>(null);
  const [autoStop, setAutoStop] = useState(false);

  useEffect(() => {
    if (!preferences) return;
    setCsvEnabled(preferences.csv.enabled);
    setOutputDir(preferences.csv.outputDir);
    setIdleCloseSecs(preferences.csv.idleCloseSecs);
    setChannelsOverride(preferences.capture.channelsOverride);
    setAutoStop(preferences.capture.autoStop);
  }, [preferences]);

  const handleSave = async () => {
    if (!preferences) return;
    try {
      await savePreferences({
        csv: { enabled: csvEnabled, outputDir, idleCloseSecs },
        capture: { channelsOverride, autoStop },
        debugTcp: preferences.debugTcp,
      });
      message.success(t('capture.preferencesSaved'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('capture.preferencesSaveFailed')));
    }
  };

  return (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
      <Space wrap size="middle">
        <Space>
          <Typography.Text>{t('capture.csvEnabled')}</Typography.Text>
          <Switch checked={csvEnabled} onChange={setCsvEnabled} />
        </Space>
        <Space>
          <Typography.Text>{t('capture.outputDir')}</Typography.Text>
          <Input
            style={{ width: 320 }}
            value={outputDir}
            onChange={(e) => setOutputDir(e.target.value)}
          />
        </Space>
        <Space>
          <Typography.Text>{t('capture.idleClose')}</Typography.Text>
          <InputNumber
            min={0}
            max={3600}
            value={idleCloseSecs}
            onChange={(v) => setIdleCloseSecs(v ?? 0)}
          />
        </Space>
      </Space>
      <Space wrap size="middle">
        <Space>
          <Typography.Text>{t('capture.channels')}</Typography.Text>
          <InputNumber
            min={1}
            max={32}
            value={channelsOverride}
            onChange={(v) => setChannelsOverride(v ?? null)}
            placeholder={t('capture.auto')}
          />
        </Space>
        <Space>
          <Typography.Text>{t('capture.autoStop')}</Typography.Text>
          <Switch checked={autoStop} onChange={setAutoStop} />
        </Space>
        <Button onClick={handleSave} disabled={!preferences}>
          {t('capture.savePreferences')}
        </Button>
      </Space>
    </Space>
  );
};

const ConfigTab: React.FC = () => {
  const { t } = useTranslation(['gh3220', 'common']);

  const collapseItems = [
    {
      key: 'connection',
      label: (
        <span>
          <ApiOutlined style={{ marginRight: 8 }} />
          {t('connect.title')}
        </span>
      ),
      children: <ConnectionSection />,
    },
    {
      key: 'preference',
      label: (
        <span>
          <SettingOutlined style={{ marginRight: 8 }} />
          {t('config.preferenceTitle')}
        </span>
      ),
      children: <CapturePreferenceSection />,
    },
    ...(import.meta.env.DEV
      ? [
          {
            key: 'debug',
            label: (
              <span>
                <CodeOutlined style={{ marginRight: 8 }} />
                {t('debug.channelTitle')}
              </span>
            ),
            children: <DebugChannelCard />,
          },
        ]
      : []),
  ];

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '8px 0' }}>
      <Collapse
        defaultActiveKey={['connection', 'preference']}
        items={collapseItems}
        size="small"
      />
    </div>
  );
};

export default ConfigTab;
