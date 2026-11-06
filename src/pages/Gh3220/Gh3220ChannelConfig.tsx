import React, { useEffect, useMemo, useState } from 'react';
import {
  Button,
  Col,
  Descriptions,
  Divider,
  Input,
  InputNumber,
  Row,
  Select,
  Space,
  Switch,
  Tag,
  Typography,
  message,
  theme,
} from 'antd';
import {
  ApiOutlined,
  DisconnectOutlined,
  FolderOpenOutlined,
  ReloadOutlined,
  SearchOutlined,
} from '@ant-design/icons';
import { open } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';
import { useGh3220Store } from '../../stores/gh3220Store';
import { bleApi } from '../../api/tauri';
import type { BleDeviceInfo } from '../../types/ble';
import type { Gh3220LinkConfig } from '../../types/gh3220';
import { formatErrorMessage } from '../../utils/errorMessage';

const { Text } = Typography;

/** 与后端 `src-tauri/src/gh3220/types.rs` 的默认 GATT 常量保持一致 */
const SERVICE_UUID = '0000190e-0000-1000-8000-00805f9b34fb';
const WRITE_UUID = '00000004-0000-1000-8000-00805f9b34fb';
const NOTIFY_UUID = '00000003-0000-1000-8000-00805f9b34fb';
const STORAGE_KEY = 'gh3220:channelConfig';

const STATE_COLOR: Record<string, string> = {
  disconnected: 'default',
  connecting: 'processing',
  ready: 'success',
  capturing: 'warning',
};

interface ChannelSnapshot {
  kind: 'ble' | 'tcp';
  address: string;
  writeUuid: string;
  notifyUuid: string;
  /** 留空表示由后端按 max_write_len 自动决定分片 */
  mtu: string;
  tcpEnabled: boolean;
}

const DEFAULT_SNAPSHOT: ChannelSnapshot = {
  kind: 'ble',
  address: '',
  writeUuid: WRITE_UUID,
  notifyUuid: NOTIFY_UUID,
  mtu: '',
  tcpEnabled: false,
};

function loadSnapshot(): ChannelSnapshot {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_SNAPSHOT };
    return { ...DEFAULT_SNAPSHOT, ...(JSON.parse(raw) as Partial<ChannelSnapshot>) };
  } catch {
    return { ...DEFAULT_SNAPSHOT };
  }
}

/**
 * GH3220 通道配置：对齐 GH3036 的配置页布局。
 * 左：连接类型 / 设备地址 / TX-RX UUID / MTU / 连接动作；
 * 右：CSV 与采集偏好（后端 `gh3220_update_preferences`）。
 * 链路参数保存在前端 localStorage，TCP 开关同时写入后端偏好（连接时后端会校验）。
 */
const Gh3220ChannelConfig: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const { token } = theme.useToken();
  const {
    status,
    preferences,
    connect,
    disconnect,
    loadDeviceInfo,
    savePreferences,
  } = useGh3220Store();

  const [snapshot, setSnapshot] = useState<ChannelSnapshot>(loadSnapshot);
  const [devices, setDevices] = useState<BleDeviceInfo[]>([]);
  const [scanning, setScanning] = useState(false);
  const [connecting, setConnecting] = useState(false);

  // CSV / 采集偏好（与右侧面板共用一份后端 preferences）
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
    setSnapshot((prev) => ({ ...prev, tcpEnabled: preferences.debugTcp.enabled }));
  }, [preferences]);

  const connected = status.state !== 'disconnected' && status.state !== 'connecting';

  const patch = (next: Partial<ChannelSnapshot>) =>
    setSnapshot((prev) => ({ ...prev, ...next }));

  const deviceOptions = useMemo(
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
      if (result.length === 0) message.warning(t('connect.noDevices'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('connect.scanFailed')));
    } finally {
      setScanning(false);
    }
  };

  const buildLinkConfig = (): Gh3220LinkConfig => {
    const address = snapshot.address.trim() || status.address || '';
    if (snapshot.kind === 'tcp') {
      return { kind: 'tcp', address };
    }
    return {
      kind: 'ble',
      address,
      writeUuid: snapshot.writeUuid.trim() || undefined,
      notifyUuid: snapshot.notifyUuid.trim() || undefined,
      mtu: snapshot.mtu.trim() ? Number(snapshot.mtu) : undefined,
    };
  };

  const persistTcpPreference = async (enabled: boolean, address: string) => {
    if (!preferences) return;
    await savePreferences({
      ...preferences,
      debugTcp: { enabled, address: address || preferences.debugTcp.address },
    });
  };

  const handleSave = async () => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(snapshot));
      if (snapshot.kind === 'tcp') {
        await persistTcpPreference(snapshot.tcpEnabled, snapshot.address.trim());
      }
      message.success(t('channel.channelSaved'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('channel.saveFailed')));
    }
  };

  const handleConnect = async () => {
    const config = buildLinkConfig();
    if (!config.address) {
      message.warning(t('channel.selectDevice'));
      return;
    }
    setConnecting(true);
    try {
      // TCP 链路后端要求先打开调试开关，这里自动落盘，避免用户多一步操作
      if (config.kind === 'tcp' && !snapshot.tcpEnabled) {
        await persistTcpPreference(true, config.address);
        patch({ tcpEnabled: true });
      }
      await connect(config);
      message.success(t('connect.connected'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('connect.connectFailed')));
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    await disconnect();
    message.info(t('connect.disconnected'));
  };

  const handleSelectCsvDir = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === 'string') setOutputDir(selected);
    } catch (err) {
      message.error(formatErrorMessage(err, t('channel.dirFailed')));
    }
  };

  const handleSaveCsv = async () => {
    if (!preferences) return;
    try {
      await savePreferences({
        csv: { enabled: csvEnabled, outputDir, idleCloseSecs },
        capture: { channelsOverride, autoStop },
        debugTcp: preferences.debugTcp,
      });
      message.success(t('channel.csvSaved'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('capture.preferencesSaveFailed')));
    }
  };

  const sectionStyle: React.CSSProperties = {
    padding: '12px 16px',
    background: token.colorFillSecondary,
    borderRadius: 8,
  };
  const labelStyle: React.CSSProperties = {
    fontSize: 13,
    fontWeight: 500,
    marginBottom: 8,
    color: token.colorTextSecondary,
  };

  return (
    <Row gutter={[12, 12]}>
      <Col span={16}>
        <div style={sectionStyle}>
          <div style={labelStyle}>{t('channel.connectionType')}</div>
          <Space direction="vertical" style={{ width: '100%' }} size={10}>
            <Select
              size="small"
              style={{ width: '100%' }}
              value={snapshot.kind}
              onChange={(kind: 'ble' | 'tcp') => patch({ kind })}
              options={[
                { label: t('channel.bleConnection'), value: 'ble' },
                { label: t('channel.tcpConnection'), value: 'tcp' },
              ]}
            />

            {snapshot.kind === 'ble' ? (
              <>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t('channel.bleHint')}
                </Text>
                <Space.Compact style={{ width: '100%' }}>
                  <Select
                    size="small"
                    style={{ flex: 1 }}
                    showSearch
                    optionFilterProp="label"
                    allowClear
                    placeholder={t('connect.placeholder')}
                    value={snapshot.address || status.address || undefined}
                    onChange={(address?: string) => patch({ address: address ?? '' })}
                    options={deviceOptions}
                  />
                  <Button
                    size="small"
                    icon={<SearchOutlined />}
                    loading={scanning}
                    disabled={connected}
                    onClick={() => void handleScan()}
                  >
                    {t('connect.scan')}
                  </Button>
                </Space.Compact>
                <Divider style={{ margin: '4px 0' }} />
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Text style={{ fontSize: 12, width: 92 }}>{t('channel.serviceUuid')}</Text>
                  <Input size="small" value={SERVICE_UUID} readOnly variant="filled" />
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Text style={{ fontSize: 12, width: 92 }}>{t('channel.txUuid')}</Text>
                  <Input
                    size="small"
                    value={snapshot.writeUuid}
                    onChange={(e) => patch({ writeUuid: e.target.value })}
                    placeholder={WRITE_UUID}
                  />
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Text style={{ fontSize: 12, width: 92 }}>{t('channel.rxUuid')}</Text>
                  <Input
                    size="small"
                    value={snapshot.notifyUuid}
                    onChange={(e) => patch({ notifyUuid: e.target.value })}
                    placeholder={NOTIFY_UUID}
                  />
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Text style={{ fontSize: 12, width: 92 }}>{t('channel.mtu')}</Text>
                  <InputNumber
                    size="small"
                    min={23}
                    max={517}
                    value={snapshot.mtu === '' ? null : Number(snapshot.mtu)}
                    onChange={(value) => patch({ mtu: value === null ? '' : String(value) })}
                    placeholder={t('channel.mtuAuto')}
                    style={{ width: 160 }}
                  />
                </div>
              </>
            ) : (
              <>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t('channel.tcpHint')}
                </Text>
                <Input
                  size="small"
                  value={snapshot.address}
                  onChange={(e) => patch({ address: e.target.value })}
                  placeholder="127.0.0.1:9000"
                />
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <Text style={{ fontSize: 12 }}>{t('channel.enableTcp')}</Text>
                  <Switch
                    size="small"
                    checked={snapshot.tcpEnabled}
                    onChange={(tcpEnabled) => patch({ tcpEnabled })}
                  />
                </div>
              </>
            )}

            <Space wrap size={8}>
              <Button size="small" onClick={() => void handleSave()}>
                {t('channel.saveChannel')}
              </Button>
              <Button
                size="small"
                type="primary"
                icon={<ApiOutlined />}
                loading={connecting}
                disabled={connected}
                onClick={() => void handleConnect()}
              >
                {t('connect.connect')}
              </Button>
              <Button
                size="small"
                danger
                icon={<DisconnectOutlined />}
                disabled={!connected}
                onClick={() => void handleDisconnect()}
              >
                {t('connect.disconnect')}
              </Button>
              <Button
                size="small"
                icon={<ReloadOutlined />}
                disabled={!connected}
                onClick={() => void loadDeviceInfo()}
              >
                {t('connect.refreshInfo')}
              </Button>
              <Button size="small" onClick={() => setSnapshot({ ...DEFAULT_SNAPSHOT })}>
                {t('channel.restoreDefaults')}
              </Button>
            </Space>
          </Space>

          <Divider style={{ margin: '12px 0 8px' }} />
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
        </div>
      </Col>

      <Col span={8}>
        <div style={sectionStyle}>
          <div style={labelStyle}>{t('channel.csvSave')}</div>
          <Space direction="vertical" style={{ width: '100%' }} size={8}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <Text style={{ fontSize: 12 }}>{t('capture.csvEnabled')}</Text>
              <Switch size="small" checked={csvEnabled} onChange={setCsvEnabled} />
            </div>
            <Space.Compact style={{ width: '100%' }}>
              <Input
                size="small"
                value={outputDir}
                onChange={(e) => setOutputDir(e.target.value)}
                placeholder={t('capture.outputDir')}
              />
              <Button
                size="small"
                icon={<FolderOpenOutlined />}
                onClick={() => void handleSelectCsvDir()}
              />
            </Space.Compact>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <Text style={{ fontSize: 12 }}>{t('capture.idleClose')}</Text>
              <InputNumber
                size="small"
                min={0}
                max={3600}
                value={idleCloseSecs}
                onChange={(value) => setIdleCloseSecs(value ?? 0)}
                style={{ width: 88 }}
              />
            </div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <Text style={{ fontSize: 12 }}>{t('capture.channels')}</Text>
              <InputNumber
                size="small"
                min={1}
                max={32}
                value={channelsOverride}
                onChange={(value) => setChannelsOverride(value ?? null)}
                placeholder={t('capture.auto')}
                style={{ width: 88 }}
              />
            </div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <Text style={{ fontSize: 12 }}>{t('capture.autoStop')}</Text>
              <Switch size="small" checked={autoStop} onChange={setAutoStop} />
            </div>
            <Button
              size="small"
              type="primary"
              block
              disabled={!preferences}
              onClick={() => void handleSaveCsv()}
            >
              {t('channel.saveCsvConfig')}
            </Button>
          </Space>
        </div>
      </Col>
    </Row>
  );
};

export default Gh3220ChannelConfig;
