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
import { useGh3220CommandConfig } from '../../stores/gh3220CommandConfig';
import { useBleStore } from '../../stores/bleStore';
import { useConnectedDevices } from '../../hooks/useConnectedDevices';
import { bleApi } from '../../api/tauri';
import type { BleDeviceInfo } from '../../types/ble';
import type { Gh3220LinkConfig } from '../../types/gh3220';
import { formatErrorMessage } from '../../utils/errorMessage';
import { GH3220_SELECT_OPTIONS, encodeBoundedNumber } from '../../constants/gh3220CommandSchema';
import {
  channelOptions,
  customOption,
  customValue,
  GH3220_DEFAULT_UUIDS,
  isCustom,
  resolveGh3220Lang,
  type Gh3220Option,
} from '../../constants/gh3220Functions';

const { Text } = Typography;

const SERVICE_UUID = GH3220_DEFAULT_UUIDS.service;
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
  tcpEnabled: boolean;
}

interface LegacySnapshot extends ChannelSnapshot {
  writeUuid?: string;
  notifyUuid?: string;
  mtu?: string;
}

const DEFAULT_SNAPSHOT: ChannelSnapshot = {
  kind: 'ble',
  address: '',
  tcpEnabled: false,
};

function loadSnapshot(): LegacySnapshot {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_SNAPSHOT };
    return { ...DEFAULT_SNAPSHOT, ...(JSON.parse(raw) as Partial<LegacySnapshot>) };
  } catch {
    return { ...DEFAULT_SNAPSHOT };
  }
}

/**
 * GH3220 通道配置（V3）：对齐 GH3036 的 16:8 两栏布局，
 * 设备 / UUID / MTU / TCP / 秒数 / 通道数全部改「选择或自动填充」；
 * 参数语义值统一存 `gh3220CommandConfig`，与命令控制台、调试页共享。
 */
const Gh3220ChannelConfig: React.FC = () => {
  const { t, i18n } = useTranslation('gh3220');
  const lang = resolveGh3220Lang(i18n.language);
  const { token } = theme.useToken();
  const {
    status,
    preferences,
    connect,
    disconnect,
    loadDeviceInfo,
    savePreferences,
  } = useGh3220Store();
  const cfg = useGh3220CommandConfig();
  const connectedDevices = useConnectedDevices();
  const bleConnections = useBleStore((state) => state.connections);

  const [snapshot, setSnapshot] = useState<ChannelSnapshot>(() => {
    const legacy = loadSnapshot();
    return { kind: legacy.kind, address: legacy.address, tcpEnabled: legacy.tcpEnabled };
  });
  const [legacyApplied, setLegacyApplied] = useState(false);
  const [devices, setDevices] = useState<BleDeviceInfo[]>([]);
  const [scanning, setScanning] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [outputDir, setOutputDir] = useState('');

  /** 只改语义值，同时标记「用户手动改过」，避免被真机值回填覆盖 */
  const patchConfig = (partial: Partial<typeof cfg>) => {
    const manual = { ...cfg.manualEdited };
    for (const key of Object.keys(partial)) manual[key] = true;
    cfg.setConfig({ ...partial, manualEdited: manual });
  };

  // 旧版本 localStorage 里保存的 UUID / MTU 迁移到集中状态（仅一次、仅在未手动改过时）
  useEffect(() => {
    if (legacyApplied) return;
    const legacy = loadSnapshot();
    const patch: Partial<typeof cfg> = {};
    if (legacy.writeUuid && !cfg.manualEdited.writeUuid) patch.writeUuid = legacy.writeUuid;
    if (legacy.notifyUuid && !cfg.manualEdited.notifyUuid) patch.notifyUuid = legacy.notifyUuid;
    if (legacy.mtu !== undefined && !cfg.manualEdited.mtu) {
      patch.mtu = legacy.mtu === '' ? 'auto' : legacy.mtu;
    }
    if (Object.keys(patch).length > 0) cfg.setConfig(patch);
    setLegacyApplied(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [legacyApplied]);

  useEffect(() => {
    if (preferences) setOutputDir(preferences.csv.outputDir);
  }, [preferences]);

  const connected = status.state !== 'disconnected' && status.state !== 'connecting';

  const patchSnapshot = (next: Partial<ChannelSnapshot>) =>
    setSnapshot((prev) => ({ ...prev, ...next }));

  /** 已连接设备 ∪ 扫描结果 ∪ 当前真机地址（去重） */
  const deviceOptions = useMemo<Gh3220Option[]>(() => {
    const seen = new Set<string>();
    const options: Gh3220Option[] = [];
    const add = (value: string, label: string) => {
      if (!value || seen.has(value)) return;
      seen.add(value);
      options.push({ value, label });
    };
    for (const device of connectedDevices.filter((item) => item.type === 'ble')) {
      add(device.id, `${device.name} (${device.id}) · 已连接`);
    }
    if (status.address) add(status.address, `${status.name || status.address} (${status.address}) · 当前链路`);
    for (const device of devices) {
      add(
        device.address,
        `${device.name || t('connect.unknownDevice')} (${device.address})${
          device.rssi === undefined ? '' : ` ${device.rssi} dBm`
        }`
      );
    }
    return options;
  }, [connectedDevices, devices, status.address, status.name, t]);

  /** 从已连接 BLE 设备的 GATT 特征里发现可写 / 可通知 UUID */
  const discoveredUuids = useMemo(() => {
    const all = bleConnections.flatMap((conn) =>
      conn.services.flatMap((service) => service.characteristics ?? [])
    );
    return {
      write: all
        .filter((char) => char.properties.write || char.properties.writeWithoutResponse)
        .map((char) => char.uuid.toLowerCase()),
      notify: all
        .filter((char) => char.properties.notify || char.properties.indicate)
        .map((char) => char.uuid.toLowerCase()),
    };
  }, [bleConnections]);

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

  /** 选设备后自动回填写/通知 UUID（对齐 GH3036 preferCharacteristic） */
  const handleDeviceChange = (address: string) => {
    patchSnapshot({ address });
    const connection = bleConnections.find((conn) => conn.address === address);
    if (!connection) return;
    const chars = connection.services.flatMap((service) => service.characteristics ?? []);
    const prefer = (list: typeof chars, part: string) =>
      list.find((char) => char.uuid.toLowerCase().includes(part))?.uuid ?? list[0]?.uuid;
    const patch: Partial<typeof cfg> = {};
    const write = chars.filter((c) => c.properties.write || c.properties.writeWithoutResponse);
    const notify = chars.filter((c) => c.properties.notify || c.properties.indicate);
    if (!cfg.manualEdited.writeUuid && write.length > 0) {
      patch.writeUuid = prefer(write, '00000004') ?? GH3220_DEFAULT_UUIDS.write;
    }
    if (!cfg.manualEdited.notifyUuid && notify.length > 0) {
      patch.notifyUuid = prefer(notify, '00000003') ?? GH3220_DEFAULT_UUIDS.notify;
    }
    if (Object.keys(patch).length > 0) cfg.setConfig(patch);
  };

  const resolved = (value: string): string =>
    isCustom(value) ? customValue(value) : value;

  const buildLinkConfig = (): Gh3220LinkConfig => {
    const address = snapshot.address.trim() || status.address || '';
    if (snapshot.kind === 'tcp') {
      return { kind: 'tcp', address: resolved(cfg.tcpAddress) };
    }
    const mtu = encodeBoundedNumber(cfg.mtu);
    return {
      kind: 'ble',
      address,
      writeUuid: resolved(cfg.writeUuid) || undefined,
      notifyUuid: resolved(cfg.notifyUuid) || undefined,
      mtu: mtu === null ? undefined : mtu,
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
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          ...snapshot,
          writeUuid: cfg.writeUuid,
          notifyUuid: cfg.notifyUuid,
          mtu: cfg.mtu === 'auto' ? '' : cfg.mtu,
        })
      );
      if (snapshot.kind === 'tcp') {
        await persistTcpPreference(snapshot.tcpEnabled, resolved(cfg.tcpAddress));
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
      if (config.kind === 'tcp' && !snapshot.tcpEnabled) {
        await persistTcpPreference(true, config.address);
        patchSnapshot({ tcpEnabled: true });
      }
      await connect(config);
      // 触发 deviceInfo / preferences 更新，命令控制台会自动回填功能选择
      await loadDeviceInfo();
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
        csv: {
          enabled: cfg.csvEnabled === 'true',
          outputDir,
          idleCloseSecs: encodeBoundedNumber(cfg.idleCloseSecs) ?? 10,
        },
        capture: {
          channelsOverride: encodeBoundedNumber(cfg.channelsOverride),
          autoStop: cfg.autoStop === 'true',
        },
        debugTcp: preferences.debugTcp,
      });
      message.success(t('channel.csvSaved'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('capture.preferencesSaveFailed')));
    }
  };

  /** 通用「自定义…」编辑器：数值档位用 InputNumber，地址/UUID 用 Input */
  const customControl = (
    value: string,
    onChange: (next: string) => void,
    numeric: boolean,
    min = 0,
    max = 65535
  ): React.ReactNode => {
    if (!isCustom(value)) return null;
    return numeric ? (
      <InputNumber
        size="small"
        min={min}
        max={max}
        value={Number(customValue(value, '0'))}
        onChange={(next) => onChange(customOption(next ?? 0))}
        style={{ width: 96 }}
      />
    ) : (
      <Input
        size="small"
        value={customValue(value)}
        onChange={(e) => onChange(customOption(e.target.value))}
        style={{ width: 180 }}
        placeholder={t('channel.customValue')}
      />
    );
  };

  const selectWithCustom = (
    value: string,
    options: Gh3220Option[],
    onChange: (next: string) => void,
    numeric = false,
    min = 0,
    max = 65535,
    width = 220
  ): React.ReactNode => (
    <>
      <Select
        size="small"
        style={{ width }}
        value={isCustom(value) ? 'custom' : value}
        options={options}
        onChange={(next: string) =>
          onChange(next === 'custom' ? customOption(customValue(value) || '0') : next)
        }
      />
      {customControl(value, onChange, numeric, min, max)}
    </>
  );

  const uuidField = (label: string, value: string, discovered: string[], field: 'writeUuid' | 'notifyUuid') => (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
      <Text style={{ fontSize: 12, width: 92 }}>{label}</Text>
      {selectWithCustom(
        value,
        GH3220_SELECT_OPTIONS.uuid([isCustom(value) ? customValue(value) : value, GH3220_DEFAULT_UUIDS[field === 'writeUuid' ? 'write' : 'notify']], discovered, lang),
        (next) => patchConfig({ [field]: next } as Partial<typeof cfg>),
        false
      )}
    </div>
  );

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
              onChange={(kind: 'ble' | 'tcp') => patchSnapshot({ kind })}
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
                    onChange={(address?: string) =>
                      address ? handleDeviceChange(address) : patchSnapshot({ address: '' })
                    }
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
                {uuidField(t('channel.txUuid'), cfg.writeUuid, discoveredUuids.write, 'writeUuid')}
                {uuidField(t('channel.rxUuid'), cfg.notifyUuid, discoveredUuids.notify, 'notifyUuid')}
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
                  <Text style={{ fontSize: 12, width: 92 }}>{t('channel.mtu')}</Text>
                  {selectWithCustom(
                    cfg.mtu,
                    GH3220_SELECT_OPTIONS.mtu(lang),
                    (next) => patchConfig({ mtu: next }),
                    true,
                    20,
                    517,
                    180
                  )}
                </div>
              </>
            ) : (
              <>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t('channel.tcpHint')}
                </Text>
                {selectWithCustom(
                  cfg.tcpAddress,
                  GH3220_SELECT_OPTIONS.tcpAddress(lang),
                  (next) => patchConfig({ tcpAddress: next }),
                  false
                )}
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <Text style={{ fontSize: 12 }}>{t('channel.enableTcp')}</Text>
                  <Switch
                    size="small"
                    checked={snapshot.tcpEnabled}
                    onChange={(tcpEnabled) => patchSnapshot({ tcpEnabled })}
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
              <Switch
                size="small"
                checked={cfg.csvEnabled === 'true'}
                onChange={(checked) => patchConfig({ csvEnabled: String(checked) })}
              />
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
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 4 }}>
              <Text style={{ fontSize: 12 }}>{t('capture.idleClose')}</Text>
              {selectWithCustom(
                cfg.idleCloseSecs,
                GH3220_SELECT_OPTIONS.idleCloseSecs(lang),
                (next) => patchConfig({ idleCloseSecs: next }),
                true,
                0,
                3600,
                130
              )}
            </div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 4 }}>
              <Text style={{ fontSize: 12 }}>{t('capture.channels')}</Text>
              {selectWithCustom(
                cfg.channelsOverride,
                channelOptions(cfg.selectedFunctions, lang),
                (next) => patchConfig({ channelsOverride: next }),
                true,
                1,
                32,
                160
              )}
            </div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <Text style={{ fontSize: 12 }}>{t('capture.autoStop')}</Text>
              <Switch
                size="small"
                checked={cfg.autoStop === 'true'}
                onChange={(checked) => patchConfig({ autoStop: String(checked) })}
              />
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
