import React, { useEffect, useMemo, useState } from 'react';
import {
  Button,
  Card,
  Collapse,
  Form,
  Input,
  InputNumber,
  Space,
  Tag,
  Typography,
  message,
  theme,
} from 'antd';
import { DisconnectOutlined, LinkOutlined, SearchOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { formatErrorMessage } from '../../utils/errorMessage';
import { bleApi } from '../../api/tauri';
import { gh3036Api } from '../../api/gh3036';
import type { BleCharacteristic, BleConnection, BleDeviceInfo, BleService } from '../../types';
import { preferencesApi } from '../../api/tauri';
import { formatMacAddress, useBleStore } from '../../stores/bleStore';
import { useGh3036Store } from '../../stores/gh3036Store';

const { Text } = Typography;

const DEFAULT_TX_UUID = '00000004-0000-1000-8000-00805f9b34fb';
const DEFAULT_RX_UUID = '00000003-0000-1000-8000-00805f9b34fb';

const normalizeAddress = (value: string) => value.replace(/[^a-fA-F0-9]/g, '').toLowerCase();
const normalizeUuid = (value: string) => value.trim().toLowerCase();

const matchesTarget = (
  device: Pick<BleDeviceInfo, 'address' | 'name'>,
  targetName: string,
  targetMac: string
) => {
  const nameHit =
    targetName.trim().length > 0 &&
    (device.name || '').toLowerCase().includes(targetName.trim().toLowerCase());
  const macHit =
    targetMac.trim().length > 0 &&
    normalizeAddress(device.address).includes(normalizeAddress(targetMac));
  return nameHit || macHit;
};

const flattenCharacteristics = (services: BleService[]) =>
  services.flatMap((service) => service.characteristics || []);

const findCharacteristic = (characteristics: BleCharacteristic[], uuid: string) => {
  const target = normalizeUuid(uuid);
  return characteristics.find((char) => normalizeUuid(char.uuid) === target);
};

const FactoryQuickConnect: React.FC = () => {
  const { t } = useTranslation('gh3036');
  const { token } = theme.useToken();
  const [form] = Form.useForm();
  const addConnection = useBleStore((state) => state.addConnection);
  const setCurrentDevice = useBleStore((state) => state.setCurrentDevice);
  const setServices = useBleStore((state) => state.setServices);
  const setCharacteristics = useBleStore((state) => state.setCharacteristics);
  const setDeviceTab = useBleStore((state) => state.setDeviceTab);
  const clearDisconnectedDevice = useBleStore((state) => state.clearDisconnectedDevice);
  const existingDeviceTabs = useBleStore((state) => state.deviceTabs);
  const connections = useBleStore((state) => state.connections);
  const channelConfig = useGh3036Store((state) => state.channelConfig);
  const updateChannelConfig = useGh3036Store((state) => state.updateChannelConfig);
  const [running, setRunning] = useState(false);
  const [status, setStatus] = useState<string>('');
  const [matchedDevice, setMatchedDevice] = useState<BleDeviceInfo | null>(null);

  const connectedDevice = useMemo(() => {
    if (!matchedDevice) return null;
    return (
      connections.find(
        (conn) => conn.isConnected && conn.address === matchedDevice.address
      ) || null
    );
  }, [connections, matchedDevice]);

  useEffect(() => {
    preferencesApi
      .get()
      .then((prefs) => {
        const channel = prefs.gh3036_channel;
        form.setFieldsValue({
          txUuid: channel?.tx_char || DEFAULT_TX_UUID,
          rxUuid: channel?.rx_char || DEFAULT_RX_UUID,
          scanSeconds: 10,
        });
      })
      .catch(() => {
        form.setFieldsValue({
          txUuid: DEFAULT_TX_UUID,
          rxUuid: DEFAULT_RX_UUID,
          scanSeconds: 10,
        });
      });
  }, [form]);

  useEffect(() => {
    if (running || channelConfig.connectionType !== 'ble' || !channelConfig.bleDevice) {
      return;
    }

    const connection = connections.find(
      (conn) => conn.isConnected && conn.address === channelConfig.bleDevice
    );
    if (!connection) {
      return;
    }

    setMatchedDevice({
      address: connection.address,
      name: connection.name,
      isConnectable: true,
      discoveredAt: connection.connectedAt || Date.now(),
    });
    setStatus(t('factory.quickConnectSuccess'));
  }, [channelConfig.bleDevice, channelConfig.connectionType, connections, running, t]);

  const statusTag = useMemo(() => {
    if (running) return <Tag color="processing">{t('factory.quickConnectRunning')}</Tag>;
    if (connectedDevice) return <Tag color="success">{t('factory.quickConnectReady')}</Tag>;
    return <Tag>{t('factory.quickConnectIdle')}</Tag>;
  }, [connectedDevice, running, t]);

  const scanUntilMatched = async (
    targetName: string,
    targetMac: string,
    timeoutMs: number
  ): Promise<BleDeviceInfo | null> => {
    const startedAt = Date.now();
    setStatus(t('factory.quickConnectScanning'));
    while (Date.now() - startedAt < timeoutMs) {
      const remainingMs = timeoutMs - (Date.now() - startedAt);
      const chunkMs = Math.min(1000, Math.max(300, remainingMs));
      const devices = await bleApi.scanBleDevices({ timeout: chunkMs });
      const matched = devices.find((device) => matchesTarget(device, targetName, targetMac));
      if (matched) {
        await bleApi.stopBleScan().catch(() => undefined);
        return matched;
      }
    }

    const devices = await bleApi.stopBleScan();
    return devices.find((device) => matchesTarget(device, targetName, targetMac)) || null;
  };

  const subscribeNotify = async (deviceAddress: string, characteristics: BleCharacteristic[]) => {
    const notifyChars = characteristics.filter(
      (char) => char.properties.notify || char.properties.indicate
    );
    const subscribedUuids: string[] = [];
    for (const char of notifyChars) {
      const subscribed = await bleApi
        .subscribeBleNotify(deviceAddress, char.uuid)
        .then(() => true)
        .catch(() => false);
      if (subscribed) {
        subscribedUuids.push(char.uuid);
      }
    }
    return subscribedUuids;
  };

  const syncBleConnectionState = (
    connection: BleConnection,
    services: BleService[],
    subscribedUuids: string[]
  ) => {
    const address = connection.address;
    const subscribedSet = new Set(subscribedUuids.map(normalizeUuid));
    const characteristics = flattenCharacteristics(services).map((char) => ({
      ...char,
      subscribed: char.subscribed || subscribedSet.has(normalizeUuid(char.uuid)),
    }));
    const existingTab = existingDeviceTabs[address];

    addConnection({
      ...connection,
      isConnected: true,
      services,
    });
    setCurrentDevice(address);
    setServices(services);
    setCharacteristics(characteristics);
    setDeviceTab(address, {
      deviceId: address,
      name: connection.name || formatMacAddress(address),
      address,
      services,
      characteristics,
      logs: existingTab?.logs || [],
      selectedCharacteristic: existingTab?.selectedCharacteristic || null,
      discoveringServices: false,
      subscribedUuids,
    });
  };

  const findExistingConnection = async (targetName: string, targetMac: string) => {
    const connections = await bleApi.getConnections().catch(() => []);
    return (
      connections.find(
        (conn) =>
          conn.isConnected &&
          matchesTarget({ address: conn.address, name: conn.name }, targetName, targetMac)
      ) || null
    );
  };

  const connectOrReuse = async (
    device: BleDeviceInfo,
    targetName: string,
    targetMac: string
  ): Promise<BleConnection> => {
    try {
      return await bleApi.connectBle(device.address);
    } catch (err) {
      const existing = await findExistingConnection(targetName, targetMac);
      if (existing && normalizeAddress(existing.address) === normalizeAddress(device.address)) {
        return existing;
      }
      throw err;
    }
  };

  const handleQuickConnect = async () => {
    const values = await form.validateFields();
    const targetName = values.deviceName || '';
    const targetMac = values.deviceMac || '';
    const txUuid = values.txUuid || DEFAULT_TX_UUID;
    const rxUuid = values.rxUuid || DEFAULT_RX_UUID;
    const scanSeconds = values.scanSeconds || 10;

    if (!targetName.trim() && !targetMac.trim()) {
      message.warning(t('factory.quickConnectTargetRequired'));
      return;
    }

    setRunning(true);
    setMatchedDevice(null);
    try {
      setStatus(t('factory.quickConnectConfiguring'));
      await bleApi.configureBle('native');

      let connection = await findExistingConnection(targetName, targetMac);
      let device: BleDeviceInfo | null = connection
        ? {
            address: connection.address,
            name: connection.name,
            isConnectable: true,
            discoveredAt: Date.now(),
          }
        : null;

      if (!connection) {
        device = await scanUntilMatched(targetName, targetMac, scanSeconds * 1000);
      }
      if (!device) {
        setStatus(t('factory.quickConnectNotFound'));
        message.warning(t('factory.quickConnectNotFound'));
        return;
      }

      setMatchedDevice(device);
      if (!connection) {
        setStatus(t('factory.quickConnectConnecting'));
        connection = await connectOrReuse(device, targetName, targetMac);
      }

      setStatus(t('factory.quickConnectDiscovering'));
      const services = await bleApi.discoverBleServices(connection.address);
      const characteristics = flattenCharacteristics(services);

      const subscribedUuids = await subscribeNotify(connection.address, characteristics);

      const txChar = findCharacteristic(characteristics, txUuid);
      const rxChar = findCharacteristic(characteristics, rxUuid);
      if (!txChar || !rxChar) {
        throw new Error(t('factory.quickConnectUuidMissing'));
      }

      setStatus(t('factory.quickConnectBinding'));
      await gh3036Api.configureTxChannel('ble', connection.address, txUuid);
      await gh3036Api.configureRxChannel('ble', connection.address, rxUuid);
      await preferencesApi.updateGh3036Channel({
        connection_type: 'ble',
        serial_port: '',
        ble_device: connection.address,
        tx_char: txUuid,
        rx_char: rxUuid,
      });
      await updateChannelConfig({
        connectionType: 'ble',
        serialPort: '',
        bleDevice: connection.address,
        txChar: txUuid,
        rxChar: rxUuid,
      });
      syncBleConnectionState(connection, services, subscribedUuids);

      setStatus(t('factory.quickConnectSuccess'));
      message.success(t('factory.quickConnectSuccess'));
    } catch (err) {
      const errorMsg = formatErrorMessage(err, t('errors.quickConnect'));
      setStatus(errorMsg);
      message.error(errorMsg);
    } finally {
      await bleApi.stopBleScan().catch(() => undefined);
      setRunning(false);
    }
  };

  const handleDisconnect = async () => {
    if (!connectedDevice) return;

    setRunning(true);
    try {
      await bleApi.disconnectBle(connectedDevice.address);
      clearDisconnectedDevice(connectedDevice.address);
      await updateChannelConfig({
        connectionType: 'ble',
        bleDevice: '',
      });
      setMatchedDevice(null);
      setStatus(t('factory.quickConnectDisconnected'));
      message.success(t('factory.quickConnectDisconnected'));
    } catch (err) {
      const errorMsg = formatErrorMessage(err, t('errors.quickDisconnect'));
      setStatus(errorMsg);
      message.error(errorMsg);
    } finally {
      setRunning(false);
    }
  };

  return (
    <Card
      size="small"
      title={
        <Space>
          <span>{t('factory.quickConnect')}</span>
          {statusTag}
        </Space>
      }
      style={{ background: token.colorBgContainer, borderRadius: token.borderRadius }}
      extra={
        <Button
          type={connectedDevice ? 'default' : 'primary'}
          danger={!!connectedDevice}
          icon={running ? <SearchOutlined /> : connectedDevice ? <DisconnectOutlined /> : <LinkOutlined />}
          loading={running}
          onClick={connectedDevice ? handleDisconnect : handleQuickConnect}
          size="small"
        >
          {connectedDevice ? t('factory.quickDisconnectAction') : t('factory.quickConnectAction')}
        </Button>
      }
    >
      <Form form={form} layout="vertical" size="small">
        <Space.Compact style={{ width: '100%' }}>
          <Form.Item name="deviceName" style={{ flex: 1, marginBottom: 6 }}>
            <Input
              allowClear
              disabled={running}
              placeholder={t('factory.quickConnectName')}
            />
          </Form.Item>
          <Form.Item name="deviceMac" style={{ flex: 1, marginBottom: 6 }}>
            <Input
              allowClear
              disabled={running}
              placeholder={t('factory.quickConnectMac')}
            />
          </Form.Item>
          <Form.Item
            name="scanSeconds"
            style={{ width: 112, marginBottom: 6 }}
          >
            <InputNumber
              min={1}
              max={60}
              disabled={running}
              addonAfter={t('factory.quickConnectSecondsUnit')}
              style={{ width: '100%' }}
            />
          </Form.Item>
        </Space.Compact>

        <Collapse
          size="small"
          ghost
          items={[
            {
              key: 'advanced',
              label: t('factory.quickConnectAdvanced'),
              children: (
                <Space.Compact style={{ width: '100%' }}>
                  <Form.Item name="txUuid" label={t('factory.quickConnectTxUuid')} style={{ flex: 1 }}>
                    <Input disabled={running} />
                  </Form.Item>
                  <Form.Item name="rxUuid" label={t('factory.quickConnectRxUuid')} style={{ flex: 1 }}>
                    <Input disabled={running} />
                  </Form.Item>
                </Space.Compact>
              ),
            },
          ]}
        />
      </Form>

      <Space style={{ marginTop: 8 }}>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {status || t('factory.quickConnectHint')}
        </Text>
        {matchedDevice && (
          <Text code style={{ fontSize: 12 }}>
            {matchedDevice.name || t('factory.quickConnectUnnamed')} / {formatMacAddress(matchedDevice.address)}
          </Text>
        )}
      </Space>
    </Card>
  );
};

export default FactoryQuickConnect;
