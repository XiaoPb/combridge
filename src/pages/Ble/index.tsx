import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Tabs, Alert, Space, Button, Tree, Tag, Typography, Empty, Spin } from 'antd';
import { MenuFoldOutlined, MenuUnfoldOutlined, ClearOutlined, SaveOutlined } from '@ant-design/icons';
import { useBle } from '../../hooks/useBle';
import { useSerialStore } from '../../stores/serialStore';
import { serialApi, bleApi } from '../../api/tauri';
import type { CacheData } from '../../api/types';
import { formatBleData, getShortUuid, formatMacAddress } from '../../stores/bleStore';
import { getServiceName, getCharacteristicName } from '../../types/ble';
import BleModeSelector from './BleModeSelector';
import BleScanner from './BleScanner';
import CharacteristicPanel from './CharacteristicPanel';
import AtConfigPanel from './AtConfigPanel';
import type { BleCharacteristic, BleService } from '../../types';

const { Text } = Typography;

interface DeviceLogEntry {
  id: string;
  timestamp: number;
  direction: 'READ' | 'WRITE' | 'NOTIFY' | 'SUBSCRIBE' | 'UNSUBSCRIBE';
  data?: number[];
  text?: string;
  characteristicUuid?: string;
}

interface DeviceTabData {
  deviceId: string;
  name: string;
  address: string;
  services: BleService[];
  characteristics: BleCharacteristic[];
  logs: DeviceLogEntry[];
  selectedCharacteristic: BleCharacteristic | null;
  discoveringServices: boolean;
}

const SCAN_TAB_KEY = 'scan';

const BlePage: React.FC = () => {
  const {
    mode,
    serialPort,
    devices,
    connections,
    currentDevice,
    services,
    notifications,
    isScanning,
    isConnecting,
    error,
    configure,
    scanDevices,
    stopScan,
    connectDevice,
    disconnectDevice,
    discoverServices,
    discoverCharacteristics,
    readCharacteristic,
    writeCharacteristic,
    subscribeNotify,
    unsubscribeNotify,
    restoreConnections,
    restoreSubscriptions,
  } = useBle();

  const { ports, setPorts } = useSerialStore();
  const [activeTabKey, setActiveTabKey] = useState<string>(SCAN_TAB_KEY);
  const [deviceTabs, setDeviceTabs] = useState<Record<string, DeviceTabData>>({});
  const [configCollapsed, setConfigCollapsed] = useState(false);
  const [gattCollapsed, setGattCollapsed] = useState(false);
  const processedNotificationIds = useRef<Set<string>>(new Set());

  useEffect(() => {
    serialApi.listPorts().then(setPorts).catch(console.error);
  }, [setPorts]);

  useEffect(() => {
    const restoreConnectedDevices = async () => {
      try {
        const connectionList = await restoreConnections();
        if (!connectionList || connectionList.length === 0) return;

        const tabs: Record<string, DeviceTabData> = {};
        for (const conn of connectionList) {
          if (!conn.isConnected) continue;

          const allCharacteristics: BleCharacteristic[] = [];
          for (const svc of conn.services || []) {
            if (svc.characteristics) {
              allCharacteristics.push(...svc.characteristics);
            }
          }

          tabs[conn.address] = {
            deviceId: conn.address,
            name: conn.name || formatMacAddress(conn.address),
            address: conn.address,
            services: conn.services || [],
            characteristics: allCharacteristics,
            logs: [],
            selectedCharacteristic: null,
            discoveringServices: false,
          };

          restoreSubscriptions(conn.address).catch((err) => {
            console.error('[BlePage] 恢复订阅失败:', { deviceId: conn.address, error: err });
          });
        }

        if (Object.keys(tabs).length > 0) {
          setDeviceTabs(tabs);
          setActiveTabKey(Object.keys(tabs)[0]);
        }
      } catch (err) {
        console.error('[BlePage] 恢复连接设备失败:', err);
      }
    };

    restoreConnectedDevices();
  }, [restoreConnections, restoreSubscriptions]);

  useEffect(() => {
    if (!currentDevice) return;
    const conn = connections.find((c) => c.address === currentDevice && c.isConnected);
    if (!conn) return;

    setDeviceTabs((prev) => {
      const existing = prev[currentDevice];
      if (existing && !existing.discoveringServices && existing.services.length === 0) {
        return {
          ...prev,
          [currentDevice]: { ...existing, discoveringServices: true },
        };
      }
      if (!existing) {
        return {
          ...prev,
          [currentDevice]: {
            deviceId: currentDevice,
            name: conn.name || formatMacAddress(conn.address),
            address: conn.address,
            services: [],
            characteristics: [],
            logs: [],
            selectedCharacteristic: null,
            discoveringServices: true,
          },
        };
      }
      return prev;
    });

    discoverServices(currentDevice)
      .then((svcList) => {
        setDeviceTabs((prev) => {
          const tab = prev[currentDevice];
          if (!tab) return prev;
          return {
            ...prev,
            [currentDevice]: {
              ...tab,
              services: svcList || [],
              discoveringServices: false,
            },
          };
        });
      })
      .catch(() => {
        setDeviceTabs((prev) => {
          const tab = prev[currentDevice];
          if (!tab) return prev;
          return {
            ...prev,
            [currentDevice]: { ...tab, discoveringServices: false },
          };
        });
      });
  }, [currentDevice, connections, discoverServices]);

  useEffect(() => {
    if (!currentDevice || services.length === 0) return;
    setDeviceTabs((prev) => {
      const tab = prev[currentDevice];
      if (!tab) return prev;
      if (JSON.stringify(tab.services) !== JSON.stringify(services)) {
        return {
          ...prev,
          [currentDevice]: { ...tab, services },
        };
      }
      return prev;
    });
  }, [currentDevice, services]);

  const handleModeChange = async (newMode: 'native' | 'at') => {
    await configure(newMode, serialPort || undefined);
  };

  const handleSerialPortChange = async (port: string) => {
    await configure(mode, port);
  };

  const handleSendAtCommand = (_command: string) => {
  };

  const addLogToDevice = useCallback((deviceId: string, entry: Omit<DeviceLogEntry, 'id'>) => {
    setDeviceTabs((prev) => {
      const tab = prev[deviceId];
      if (!tab) return prev;
      return {
        ...prev,
        [deviceId]: {
          ...tab,
          logs: [...tab.logs, { ...entry, id: `${Date.now()}-${Math.random()}` }].slice(-500),
        },
      };
    });
  }, []);

  useEffect(() => {
    if (!notifications.length || !currentDevice) return;
    
    for (const notif of notifications) {
      if (processedNotificationIds.current.has(notif.id)) continue;
      processedNotificationIds.current.add(notif.id);
      
      const targetAddress = connections.find(c => c.address === currentDevice)?.address;
      if (notif.deviceId === currentDevice || notif.deviceId === targetAddress) {
        addLogToDevice(currentDevice, {
          timestamp: notif.timestamp,
          direction: 'NOTIFY',
          data: notif.data,
          characteristicUuid: notif.characteristicUuid,
        });
      }
    }
  }, [notifications, currentDevice, connections, addLogToDevice]);

  const handleConnect = async (address: string) => {
    const existingConn = connections.find((c) => c.address === address);
    if (existingConn) {
      await handleDisconnect(address);
      return;
    }

    try {
      await connectDevice(address);
      setActiveTabKey(address);
    } catch (err) {
      console.error('[BlePage] 连接失败:', err);
    }
  };

  const handleDisconnect = async (deviceId: string) => {
    try {
      await disconnectDevice(deviceId);
    } catch {
      // error already handled by hook
    }
    setDeviceTabs((prev) => {
      const next = { ...prev };
      delete next[deviceId];
      return next;
    });
    if (activeTabKey === deviceId) {
      setActiveTabKey(SCAN_TAB_KEY);
    }
  };

  const handleTabEdit = useCallback(
    (targetKey: string | React.MouseEvent | React.KeyboardEvent, action: 'add' | 'remove') => {
      if (action === 'remove') {
        const key = targetKey as string;
        if (key === SCAN_TAB_KEY) return;
        handleDisconnect(key);
      }
    },
    [handleDisconnect]
  );

  const handleServiceSelectForDevice = useCallback(
    async (serviceUuid: string, deviceId: string) => {
      try {
        const charList = await discoverCharacteristics(serviceUuid, deviceId);
        setDeviceTabs((prev) => {
          const tab = prev[deviceId];
          if (!tab) return prev;
          return {
            ...prev,
            [deviceId]: { ...tab, characteristics: charList || [] },
          };
        });
      } catch (err) {
        console.error('发现特征失败:', err);
      }
    },
    [discoverCharacteristics]
  );

  const handleCharacteristicSelectForDevice = useCallback(
    async (characteristic: BleCharacteristic, deviceId: string) => {
      setDeviceTabs((prev) => {
        const tab = prev[deviceId];
        if (!tab) return prev;
        return {
          ...prev,
          [deviceId]: { ...tab, selectedCharacteristic: characteristic },
        };
      });

      try {
        const cacheData: CacheData = await bleApi.getCache(characteristic.uuid);
        const cacheLogs: DeviceLogEntry[] = [];
        
        for (const entry of cacheData.tx || []) {
          cacheLogs.push({
            id: `cache-tx-${entry.timestamp}-${Math.random()}`,
            timestamp: entry.timestamp,
            direction: 'WRITE',
            data: entry.data,
            characteristicUuid: characteristic.uuid,
          });
        }
        
        for (const entry of cacheData.rx || []) {
          cacheLogs.push({
            id: `cache-rx-${entry.timestamp}-${Math.random()}`,
            timestamp: entry.timestamp,
            direction: 'NOTIFY',
            data: entry.data,
            characteristicUuid: characteristic.uuid,
          });
        }
        
        cacheLogs.sort((a, b) => a.timestamp - b.timestamp);
        
        if (cacheLogs.length > 0) {
          setDeviceTabs((prev) => {
            const tab = prev[deviceId];
            if (!tab) return prev;
            const existingIds = new Set(tab.logs.map(l => l.id));
            const newLogs = cacheLogs.filter(l => !existingIds.has(l.id));
            return {
              ...prev,
              [deviceId]: {
                ...tab,
                logs: [...tab.logs, ...newLogs].slice(-500),
              },
            };
          });
        }
      } catch (err) {
        console.debug('[BlePage] 获取缓存数据失败或无缓存:', err);
      }
    },
    []
  );

  const handleReadForDevice = useCallback(
    async (uuid: string, deviceId: string) => {
      try {
        const data = await readCharacteristic(uuid, deviceId);
        addLogToDevice(deviceId, {
          timestamp: Date.now(),
          direction: 'READ',
          data,
          characteristicUuid: uuid,
        });
      } catch {
        // handled by hook
      }
    },
    [readCharacteristic, addLogToDevice]
  );

  const handleWriteForDevice = useCallback(
    async (uuid: string, dataStr: string, format: 'hex' | 'text', withoutResponse: boolean, deviceId: string) => {
      try {
        await writeCharacteristic(uuid, dataStr, format, withoutResponse, deviceId);
        addLogToDevice(deviceId, {
          timestamp: Date.now(),
          direction: 'WRITE',
          text: dataStr,
          characteristicUuid: uuid,
        });
      } catch {
        // handled by hook
      }
    },
    [writeCharacteristic, addLogToDevice]
  );

  const handleSubscribeForDevice = useCallback(
    async (uuid: string, deviceId: string) => {
      try {
        await subscribeNotify(uuid, deviceId);
        addLogToDevice(deviceId, {
          timestamp: Date.now(),
          direction: 'SUBSCRIBE',
          characteristicUuid: uuid,
        });
      } catch {
        // handled by hook
      }
    },
    [subscribeNotify, addLogToDevice]
  );

  const handleUnsubscribeForDevice = useCallback(
    async (uuid: string, deviceId: string) => {
      try {
        await unsubscribeNotify(uuid, deviceId);
        addLogToDevice(deviceId, {
          timestamp: Date.now(),
          direction: 'UNSUBSCRIBE',
          characteristicUuid: uuid,
        });
      } catch {
        // handled by hook
      }
    },
    [unsubscribeNotify, addLogToDevice]
  );

  const handleClearLogs = useCallback((deviceId: string) => {
    setDeviceTabs((prev) => {
      const tab = prev[deviceId];
      if (!tab) return prev;
      return { ...prev, [deviceId]: { ...tab, logs: [] } };
    });
  }, []);

  const buildTreeData = (svcList: BleService[]) => {
    return svcList.map((svc) => ({
      key: svc.uuid,
      title: (
        <Space>
          <Text strong style={{ fontSize: 13 }}>{getServiceName(svc.uuid)} (0x{getShortUuid(svc.uuid)})</Text>
          {svc.isPrimary && <Tag color="blue" style={{ fontSize: 10 }}>Primary</Tag>}
        </Space>
      ),
      children: (svc.characteristics || []).map((char) => ({
        key: `${svc.uuid}-${char.uuid}`,
        title: (
          <Space>
            <Text style={{ fontSize: 13 }}>{getCharacteristicName(char.uuid)} (0x{getShortUuid(char.uuid)})</Text>
            {char.properties.read && <Tag color="green" style={{ fontSize: 10 }}>R</Tag>}
            {char.properties.write && <Tag color="blue" style={{ fontSize: 10 }}>W</Tag>}
            {char.properties.notify && <Tag color="orange" style={{ fontSize: 10 }}>N</Tag>}
            {char.properties.indicate && <Tag color="purple" style={{ fontSize: 10 }}>I</Tag>}
          </Space>
        ),
        isLeaf: true,
      })),
    }));
  };

  const renderScanTab = () => (
    <div style={{ display: 'flex', height: '100%', gap: 8, overflow: 'hidden' }}>
      <div style={{ flex: '1 1 50%', minWidth: 0, minHeight: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <BleScanner
          devices={devices}
          connections={connections}
          isScanning={isScanning}
          onScan={(timeout) => scanDevices(timeout ? { timeout } : undefined)}
          onStopScan={stopScan}
          onConnect={handleConnect}
        />
      </div>

      {!configCollapsed && (
        <div
          style={{
            flex: '0 0 320px',
            overflow: 'auto',
            background: 'var(--bg-secondary)',
            borderRadius: 8,
            padding: 8,
            transition: 'all 0.2s',
          }}
        >
          <Space style={{ width: '100%', justifyContent: 'space-between', marginBottom: 12 }}>
            <Text strong>配置面板</Text>
            <Button
              type="text"
              size="small"
              icon={<MenuFoldOutlined />}
              onClick={() => setConfigCollapsed(true)}
            />
          </Space>

          <Space vertical style={{ width: '100%' }} size="middle">
            <BleModeSelector
              mode={mode}
              serialPort={serialPort}
              ports={ports}
              onModeChange={handleModeChange}
              onSerialPortChange={handleSerialPortChange}
            />
            <AtConfigPanel
              ports={ports}
              selectedPort={serialPort}
              onPortChange={handleSerialPortChange}
              onSendCommand={handleSendAtCommand}
            />
          </Space>
        </div>
      )}

      {configCollapsed && (
        <Button
          type="text"
          icon={<MenuUnfoldOutlined />}
          onClick={() => setConfigCollapsed(false)}
          style={{ flexShrink: 0, alignSelf: 'flex-start' }}
        />
      )}
    </div>
  );

  const renderDeviceTabContent = (tabData: DeviceTabData) => {
    const treeData = buildTreeData(tabData.services);
    const logText = tabData.logs
      .map((entry) => {
        const ts = new Date(entry.timestamp).toLocaleTimeString();
        const dir = entry.direction;
        let content = '';
        if (entry.data) {
          content = `[${entry.data.length} byte] ${formatBleData(entry.data, 'hex')}`;
        } else if (entry.text) {
          content = entry.text;
        } else {
          content = '';
        }
        const charInfo = entry.characteristicUuid ? ` [${getShortUuid(entry.characteristicUuid)}]` : '';
        return `[${ts}][${dir}]${charInfo} ${content}`;
      })
      .join('\n');

    const handleSaveLog = async () => {
      if (!logText) return;
      try {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const { writeTextFile } = await import('@tauri-apps/plugin-fs');
        const filePath = await save({
          defaultPath: `ble-log-${new Date().toISOString().slice(0, 10)}.txt`,
          filters: [
            { name: 'Text', extensions: ['txt'] },
            { name: 'Log', extensions: ['log'] },
          ],
        });
        if (filePath) {
          await writeTextFile(filePath, logText);
        }
      } catch (err) {
        console.error('[BlePage] 保存日志失败:', err);
      }
    };

    return (
      <div style={{ display: 'flex', height: '100%', gap: 8, overflow: 'hidden' }}>
        {!gattCollapsed && (
          <div style={{ flex: '0 0 280px', minWidth: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden', background: 'var(--bg-secondary)', borderRadius: 8, padding: 8 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8, flexShrink: 0 }}>
              <Text strong style={{ fontSize: 13 }}>GATT 服务树</Text>
              <Button
                type="text"
                size="small"
                icon={<MenuFoldOutlined />}
                onClick={() => setGattCollapsed(true)}
                title="折叠服务树"
              />
            </div>
            <div style={{ flex: '1 1 0', overflow: 'auto', padding: '0 4px' }}>
              {tabData.discoveringServices ? (
                <div style={{ textAlign: 'center', padding: 40 }}>
                  <Spin description="正在发现服务..." />
                </div>
              ) : tabData.services.length === 0 ? (
                <Empty description="暂无服务数据" />
              ) : (
                <Tree
                  showLine
                  showIcon={false}
                  treeData={treeData}
                  onSelect={(keys) => {
                    if (keys.length === 0) return;
                    const key = keys[0] as string;
                    for (const svc of tabData.services) {
                      const char = (svc.characteristics || []).find((c) => `${svc.uuid}-${c.uuid}` === key);
                      if (char) {
                        handleCharacteristicSelectForDevice(char, tabData.deviceId);
                        return;
                      }
                    }
                    const svc = tabData.services.find((s) => s.uuid === key);
                    if (svc) {
                      handleServiceSelectForDevice(svc.uuid, tabData.deviceId);
                    }
                  }}
                  style={{ fontSize: 13 }}
                />
              )}
            </div>
          </div>
        )}

        {gattCollapsed && (
          <Button
            type="text"
            icon={<MenuUnfoldOutlined />}
            onClick={() => setGattCollapsed(false)}
            style={{ flexShrink: 0, alignSelf: 'flex-start' }}
            title="展开服务树"
          />
        )}

        <div style={{ flex: '1 1 0', minWidth: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <div style={{ flexShrink: 0, marginBottom: 8 }}>
            <CharacteristicPanel
              characteristic={tabData.selectedCharacteristic}
              onRead={(uuid) => handleReadForDevice(uuid, tabData.deviceId)}
              onWrite={(uuid, data, fmt, wnr) => handleWriteForDevice(uuid, data, fmt, wnr, tabData.deviceId)}
              onSubscribe={(uuid) => handleSubscribeForDevice(uuid, tabData.deviceId)}
              onUnsubscribe={(uuid) => handleUnsubscribeForDevice(uuid, tabData.deviceId)}
            />
          </div>

          <div style={{ flex: '1 1 0', minHeight: 400, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            <div style={{ marginBottom: 8, flexShrink: 0, display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
              <Button
                size="small"
                icon={<SaveOutlined />}
                onClick={handleSaveLog}
                disabled={tabData.logs.length === 0}
              >
                保存日志
              </Button>
              <Button
                size="small"
                icon={<ClearOutlined />}
                onClick={() => handleClearLogs(tabData.deviceId)}
                disabled={tabData.logs.length === 0}
              >
                清空日志
              </Button>
            </div>

            <div
              style={{
                flex: '1 1 0',
                overflow: 'auto',
                background: 'var(--bg-primary)',
                borderRadius: 4,
                padding: 8,
                fontFamily: 'Consolas, Monaco, monospace',
                fontSize: 13,
                lineHeight: 1.6,
                minHeight: 0,
                whiteSpace: 'pre',
              }}
            >
              {logText || '暂无交互日志...'}
            </div>
          </div>
        </div>
      </div>
    );
  };

  const tabItems = [
    {
      key: SCAN_TAB_KEY,
      label: (
        <span style={{ fontSize: 12 }}>
          扫描
          {isScanning && <Tag color="processing" style={{ marginLeft: 4, fontSize: 10 }} />}
        </span>
      ),
      closable: false,
      children: renderScanTab(),
    },
    ...Object.entries(deviceTabs).map(([key, tab]) => {
      return {
        key,
        label: (
          <span style={{ fontSize: 12 }}>
            {tab.name || formatMacAddress(key)}
            <Tag color="success" style={{ marginLeft: 4, fontSize: 10 }}>●</Tag>
          </span>
        ),
        closable: true,
        children: renderDeviceTabContent(tab),
      };
    }),
  ];

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {error && (
        <Alert
          title="错误"
          description={error}
          type="error"
          closable
          onClose={() => {}}
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      {isConnecting && (
        <Alert
          title="正在连接..."
          type="info"
          showIcon
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      <div style={{ flex: '1 1 0', minHeight: 0, overflow: 'hidden' }}>
        <Tabs
          type="editable-card"
          activeKey={activeTabKey}
          onChange={setActiveTabKey}
          onEdit={handleTabEdit}
          items={tabItems}
          size="small"
          style={{ height: '100%' }}
          tabBarStyle={{ marginBottom: 0, paddingLeft: 8, paddingRight: 8 }}
        />
      </div>
    </div>
  );
};

export default BlePage;
