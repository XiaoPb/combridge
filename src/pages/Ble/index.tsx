import React, { useEffect, useCallback, useRef } from 'react';
import { Alert, Space, Button, Tree, Tag, Typography, Empty, Spin, Card, Input, Segmented, Tooltip } from 'antd';
import { MenuFoldOutlined, MenuUnfoldOutlined, ClearOutlined, DownloadOutlined, VerticalAlignBottomOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useBle } from '../../hooks/useBle';
import { useSerialStore } from '../../stores/serialStore';
import { useBleStore, formatBleData, getShortUuid, formatMacAddress, type DeviceLogEntry, type DeviceTabData } from '../../stores/bleStore';
import { serialApi, bleApi } from '../../api/tauri';
import type { CacheData } from '../../api/types';
import { getServiceName, getCharacteristicName } from '../../types/ble';
import BleModeSelector from './BleModeSelector';
import BleScanner from './BleScanner';
import CharacteristicPanel from './CharacteristicPanel';
import AtConfigPanel from './AtConfigPanel';
import type { BleCharacteristic, BleService } from '../../types';
import type { TextAreaRef } from 'antd/es/input/TextArea';
import { useConfigStore } from '../../stores/configStore';

const { Text } = Typography;
const { TextArea } = Input;

const normalizeUuid = (uuid: string) => uuid.trim().toLowerCase();

const formatTimestamp = (timestamp: number, timezone?: string): string => {
  const date = new Date(timestamp);
  
  if (timezone) {
    try {
      const options: Intl.DateTimeFormatOptions = {
        timeZone: timezone,
        hour12: false,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      };
      const timeStr = date.toLocaleTimeString('zh-CN', options);
      const ms = date.getMilliseconds().toString().padStart(3, '0');
      return `${timeStr}.${ms}`;
    } catch {
      // 如果时区无效，使用本地时区
    }
  }
  
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  const seconds = date.getSeconds().toString().padStart(2, '0');
  const ms = date.getMilliseconds().toString().padStart(3, '0');
  return `${hours}:${minutes}:${seconds}.${ms}`;
};

const BlePage: React.FC = () => {
  const { t } = useTranslation('ble');
  const timezone = useConfigStore((state) => state.settings.timezone);
  const hasHydrated = useConfigStore((state) => state._hasHydrated);
  const effectiveTimezone = hasHydrated ? timezone : 'Asia/Shanghai';
  
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
    setCurrentDevice,
  } = useBle();

  const { ports, setPorts } = useSerialStore();
  const { 
    preferences, 
    updatePreferences, 
    loadPreferences,
    deviceTabs,
    setDeviceTabs,
    updateDeviceTab,
    removeDeviceTab,
    addDeviceLog,
    clearDeviceLogs,
  } = useBleStore();
  const [preferencesLoaded, setPreferencesLoaded] = React.useState(false);
  const processedNotificationIds = useRef<Set<string>>(new Set());
  const logContainerRefs = useRef<Record<string, TextAreaRef>>({});
  const lastLogCountRef = useRef<Record<string, number>>({});
  const discoveringDevicesRef = useRef<Set<string>>(new Set());
  const autoSubscribedRef = useRef<Record<string, Set<string>>>({});

  useEffect(() => {
    loadPreferences()
      .then(() => setPreferencesLoaded(true))
      .catch((err) => {
        console.error('[BlePage]', t('message.loadPrefsFailed'), err);
        setPreferencesLoaded(true);
      });
  }, [loadPreferences]);

  useEffect(() => {
    serialApi.listPorts().then(setPorts).catch(console.error);
  }, [setPorts]);

  const addLogToDeviceCallback = useCallback((deviceId: string, entry: Omit<DeviceLogEntry, 'id'>) => {
    addDeviceLog(deviceId, { ...entry, id: `${Date.now()}-${Math.random()}` });
  }, [addDeviceLog]);

  const autoSubscribeNotifyCharacteristics = useCallback(
    async (deviceId: string, characteristics: BleCharacteristic[]) => {
      const notifyCharacteristics = characteristics.filter(
        (char) => char.properties.notify || char.properties.indicate
      );
      if (notifyCharacteristics.length === 0) return;

      const subscribed = autoSubscribedRef.current[deviceId] ?? new Set<string>();
      autoSubscribedRef.current[deviceId] = subscribed;

      const successUuids: string[] = [];
      for (const char of notifyCharacteristics) {
        if (char.subscribed || subscribed.has(char.uuid)) {
          successUuids.push(char.uuid);
          continue;
        }

        try {
          await bleApi.subscribeBleNotify(deviceId, char.uuid);
          subscribed.add(char.uuid);
          successUuids.push(char.uuid);
          addLogToDeviceCallback(deviceId, {
            timestamp: Date.now(),
            direction: 'SUBSCRIBE',
            text: '自动订阅 notify',
            characteristicUuid: char.uuid,
          });
        } catch (err) {
          console.warn('[BlePage] auto subscribe notify failed:', char.uuid, err);
          addLogToDeviceCallback(deviceId, {
            timestamp: Date.now(),
            direction: 'SUBSCRIBE',
            text: '自动订阅 notify 失败',
            characteristicUuid: char.uuid,
          });
        }
      }

      if (successUuids.length === 0) return;

      const successSet = new Set(successUuids);
      updateDeviceTab(deviceId, {
        characteristics: characteristics.map((char) =>
          successSet.has(char.uuid) ? { ...char, subscribed: true } : char
        ),
        subscribedUuids: [
          ...new Set([
            ...(useBleStore.getState().deviceTabs[deviceId]?.subscribedUuids ?? []),
            ...successUuids,
          ]),
        ],
      });
    },
    [addLogToDeviceCallback, updateDeviceTab]
  );

  useEffect(() => {
    if (!preferencesLoaded) return;

    const restoreConnectedDevices = async () => {
      try {
        const connectionList = await restoreConnections();
        if (!connectionList || connectionList.length === 0) return;

        const existingTabs = useBleStore.getState().deviceTabs;
        const tabs: Record<string, DeviceTabData> = {};
        for (const conn of connectionList) {
          if (!conn.isConnected) continue;

          const allCharacteristics: BleCharacteristic[] = [];
          for (const svc of conn.services || []) {
            if (svc.characteristics) {
              allCharacteristics.push(...svc.characteristics);
            }
          }

          const subscribedUuids = allCharacteristics.filter(c => c.subscribed).map(c => c.uuid);
          
          const existingTab = existingTabs[conn.address];

          tabs[conn.address] = {
            deviceId: conn.address,
            name: conn.name || formatMacAddress(conn.address),
            address: conn.address,
            services: conn.services || [],
            characteristics: allCharacteristics,
            logs: existingTab?.logs || [],
            selectedCharacteristic: existingTab?.selectedCharacteristic || null,
            discoveringServices: false,
            subscribedUuids,
          };
        }

        if (Object.keys(tabs).length > 0) {
          setDeviceTabs(tabs);
          const firstDeviceId = Object.keys(tabs)[0];
          if (!useBleStore.getState().currentDevice) {
            setCurrentDevice(firstDeviceId);
          }
        }
      } catch (err) {
        console.error('[BlePage]', t('message.restoreFailed'), err);
      }
    };

    restoreConnectedDevices();
  }, [preferencesLoaded, restoreConnections, setCurrentDevice, setDeviceTabs]);

  useEffect(() => {
    if (!currentDevice) return;
    const conn = connections.find((c) => c.address === currentDevice && c.isConnected);
    if (!conn) return;

    if (discoveringDevicesRef.current.has(currentDevice)) return;

    const existing = deviceTabs[currentDevice];
    if (existing && existing.services.length > 0) return;

    discoveringDevicesRef.current.add(currentDevice);

    if (existing) {
      updateDeviceTab(currentDevice, { discoveringServices: true });
    } else {
      setDeviceTabs({
        ...deviceTabs,
        [currentDevice]: {
          deviceId: currentDevice,
          name: conn.name || formatMacAddress(conn.address),
          address: conn.address,
          services: [],
          characteristics: [],
          logs: [],
          selectedCharacteristic: null,
          discoveringServices: true,
          subscribedUuids: [],
        },
      });
    }

    discoverServices(currentDevice)
      .then((svcList) => {
        discoveringDevicesRef.current.delete(currentDevice);
        if (!svcList) return;
        
        const allCharacteristics: BleCharacteristic[] = [];
        for (const svc of svcList) {
          if (svc.characteristics) {
            allCharacteristics.push(...svc.characteristics);
          }
        }
        
        const subscribedUuids = allCharacteristics.filter(c => c.subscribed).map(c => c.uuid);
        
        updateDeviceTab(currentDevice, {
          services: svcList,
          characteristics: allCharacteristics,
          discoveringServices: false,
          subscribedUuids,
        });

        void autoSubscribeNotifyCharacteristics(currentDevice, allCharacteristics);
      })
      .catch(() => {
        discoveringDevicesRef.current.delete(currentDevice);
        updateDeviceTab(currentDevice, { discoveringServices: false });
      });
  }, [currentDevice, connections, discoverServices, deviceTabs, setDeviceTabs, updateDeviceTab, autoSubscribeNotifyCharacteristics]);

  useEffect(() => {
    if (!currentDevice || services.length === 0) return;
    const tab = deviceTabs[currentDevice];
    if (!tab) return;
    if (JSON.stringify(tab.services) !== JSON.stringify(services)) {
      updateDeviceTab(currentDevice, { services });
    }
  }, [currentDevice, services, deviceTabs, updateDeviceTab]);

  const handleModeChange = async (newMode: 'native' | 'at') => {
    await configure(newMode, serialPort || undefined);
  };

  const handleSerialPortChange = async (port: string) => {
    await configure(mode, port);
  };

  const handleSendAtCommand = (_command: string) => {
  };

  useEffect(() => {
    if (!notifications.length || !currentDevice) return;
    
    for (const notif of notifications) {
      if (processedNotificationIds.current.has(notif.id)) continue;
      processedNotificationIds.current.add(notif.id);
      
      const targetAddress = connections.find(c => c.address === currentDevice)?.address;
      if (notif.deviceId === currentDevice || notif.deviceId === targetAddress) {
        addLogToDeviceCallback(currentDevice, {
          timestamp: notif.timestamp,
          direction: 'NOTIFY',
          data: notif.data,
          characteristicUuid: notif.characteristicUuid,
        });
      }
    }
  }, [notifications, currentDevice, connections, addLogToDeviceCallback]);

  const currentTab = currentDevice ? deviceTabs[currentDevice] : null;
  
  useEffect(() => {
    if (!currentTab || !preferences.autoScroll) return;
    
    const currentCount = currentTab.logs.length;
    const lastCount = lastLogCountRef.current[currentDevice!] || 0;
    
    if (currentCount !== lastCount) {
      lastLogCountRef.current[currentDevice!] = currentCount;
      requestAnimationFrame(() => {
        const textArea = logContainerRefs.current[currentDevice!]?.resizableTextArea?.textArea;
        if (textArea) {
          textArea.scrollTop = textArea.scrollHeight;
        }
      });
    }
  }, [currentTab, currentDevice, preferences.autoScroll]);

  const handleConnect = async (address: string) => {
    const existingConn = connections.find((c) => c.address === address);
    if (existingConn) {
      await handleDisconnect(address);
      return;
    }

    try {
      await connectDevice(address);
    } catch (err) {
      console.error('[BlePage] connect failed:', err);
    }
  };

  const handleDisconnect = async (deviceId: string) => {
    try {
      await disconnectDevice(deviceId);
    } catch {
      // error already handled by hook
    }
    removeDeviceTab(deviceId);
    delete autoSubscribedRef.current[deviceId];
    if (currentDevice === deviceId) {
      setCurrentDevice(null);
    }
  };

  const handleServiceSelectForDevice = useCallback(
    async (serviceUuid: string, deviceId: string) => {
      try {
        const charList = await discoverCharacteristics(serviceUuid, deviceId);
        updateDeviceTab(deviceId, { characteristics: charList || [] });
        void autoSubscribeNotifyCharacteristics(deviceId, charList || []);
      } catch (err) {
        console.error(t('message.discoverFailed'), err);
      }
    },
    [discoverCharacteristics, updateDeviceTab, autoSubscribeNotifyCharacteristics, t]
  );

  const handleCharacteristicSelectForDevice = useCallback(
    async (characteristic: BleCharacteristic, deviceId: string) => {
      updateDeviceTab(deviceId, { selectedCharacteristic: characteristic });

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
          const tab = deviceTabs[deviceId];
          if (tab) {
            const existingIds = new Set(tab.logs.map(l => l.id));
            const newLogs = cacheLogs.filter(l => !existingIds.has(l.id));
            updateDeviceTab(deviceId, { logs: [...tab.logs, ...newLogs].slice(-500) });
          }
        }
      } catch (err) {
        console.debug('[BlePage] get cache failed or no cache:', err);
      }
    },
    [deviceTabs, updateDeviceTab]
  );

  const handleReadForDevice = useCallback(
    async (uuid: string, deviceId: string) => {
      try {
        const data = await readCharacteristic(uuid, deviceId);
        addLogToDeviceCallback(deviceId, {
          timestamp: Date.now(),
          direction: 'READ',
          data,
          characteristicUuid: uuid,
        });
      } catch {
        // handled by hook
      }
    },
    [readCharacteristic, addLogToDeviceCallback]
  );

  const handleWriteForDevice = useCallback(
    async (uuid: string, dataStr: string, format: 'hex' | 'text', withoutResponse: boolean, deviceId: string) => {
      try {
        await writeCharacteristic(uuid, dataStr, format, withoutResponse, deviceId);
        addLogToDeviceCallback(deviceId, {
          timestamp: Date.now(),
          direction: 'WRITE',
          text: dataStr,
          characteristicUuid: uuid,
        });
      } catch {
        // handled by hook
      }
    },
    [writeCharacteristic, addLogToDeviceCallback]
  );

  const handleSubscribeForDevice = useCallback(
    async (uuid: string, deviceId: string) => {
      try {
        await subscribeNotify(uuid, deviceId);
        const tab = deviceTabs[deviceId];
        if (tab) {
          const newUuids = [...new Set([...tab.subscribedUuids, uuid])];
          updateDeviceTab(deviceId, { subscribedUuids: newUuids });
        }
        addLogToDeviceCallback(deviceId, {
          timestamp: Date.now(),
          direction: 'SUBSCRIBE',
          characteristicUuid: uuid,
        });
      } catch {
        // handled by hook
      }
    },
    [subscribeNotify, addLogToDeviceCallback, deviceTabs, updateDeviceTab]
  );

  const handleUnsubscribeForDevice = useCallback(
    async (uuid: string, deviceId: string) => {
      try {
        await unsubscribeNotify(uuid, deviceId);
        const tab = deviceTabs[deviceId];
        if (tab) {
          const newUuids = tab.subscribedUuids.filter(u => u !== uuid);
          updateDeviceTab(deviceId, { subscribedUuids: newUuids });
        }
        addLogToDeviceCallback(deviceId, {
          timestamp: Date.now(),
          direction: 'UNSUBSCRIBE',
          characteristicUuid: uuid,
        });
      } catch {
        // handled by hook
      }
    },
    [unsubscribeNotify, addLogToDeviceCallback, deviceTabs, updateDeviceTab]
  );

  const handleClearLogs = useCallback((deviceId: string) => {
    clearDeviceLogs(deviceId);
  }, [clearDeviceLogs]);

  const handleExportLog = useCallback(async (tabData: DeviceTabData) => {
    if (tabData.logs.length === 0) return;
    
    const logText = tabData.logs
      .map((entry) => {
        const ts = formatTimestamp(entry.timestamp, effectiveTimezone);
        const dir = entry.direction;
        let content = '';
        if (entry.data) {
          content = `[${entry.data.length} byte] ${formatBleData(entry.data, 'hex')}`;
        } else if (entry.text) {
          content = entry.text;
        }
        const charInfo = entry.characteristicUuid ? ` [${getShortUuid(entry.characteristicUuid)}]` : '';
        return `[${ts}][${dir}]${charInfo} ${content}`;
      })
      .join('\n');

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
      console.error('[BlePage]', t('message.saveLogFailed'), err);
    }
  }, [t, effectiveTimezone]);

  const buildTreeData = (svcList: BleService[]) => {
    return svcList.map((svc) => ({
      key: svc.uuid,
      title: (
        <div style={{ lineHeight: 1.3 }}>
          <div style={{ fontSize: 13, fontWeight: 500 }}>{getServiceName(svc.uuid)}</div>
          <div style={{ fontSize: 9, color: '#666' }}>{svc.uuid}</div>
        </div>
      ),
      children: (svc.characteristics || []).map((char) => ({
        key: `${svc.uuid}-${char.uuid}`,
        title: (
          <div style={{ lineHeight: 1.3 }}>
            <div style={{ fontSize: 13, fontWeight: 500 }}>{getCharacteristicName(char.uuid)}</div>
            <div style={{ fontSize: 9, color: '#666' }}>{char.uuid}</div>
            <div style={{ marginTop: 2 }}>
              {char.properties.read && <Tag color="green" style={{ fontSize: 9, padding: '0 4px', margin: '0 2px 0 0' }}>R</Tag>}
              {char.properties.write && <Tag color="blue" style={{ fontSize: 9, padding: '0 4px', margin: '0 2px 0 0' }}>W</Tag>}
              {char.properties.writeWithoutResponse && <Tag color="cyan" style={{ fontSize: 9, padding: '0 4px', margin: '0 2px 0 0' }}>WNR</Tag>}
              {char.properties.notify && <Tag color="orange" style={{ fontSize: 9, padding: '0 4px', margin: '0 2px 0 0' }}>N</Tag>}
              {char.properties.indicate && <Tag color="purple" style={{ fontSize: 9, padding: '0 4px', margin: '0 2px 0 0' }}>I</Tag>}
            </div>
          </div>
        ),
        isLeaf: true,
      })),
    }));
  };

  const renderScanContent = () => (
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

      {!preferences.configCollapsed && (
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
            <Text strong>{t('title.configPanel')}</Text>
            <Button
              type="text"
              size="small"
              icon={<MenuFoldOutlined />}
              onClick={() => updatePreferences({ configCollapsed: true })}
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

      {preferences.configCollapsed && (
        <Button
          type="text"
          icon={<MenuUnfoldOutlined />}
          onClick={() => updatePreferences({ configCollapsed: false })}
          style={{ flexShrink: 0, alignSelf: 'flex-start' }}
        />
      )}
    </div>
  );

  const renderDeviceContent = (tabData: DeviceTabData) => {
    const treeData = buildTreeData(tabData.services);
    const isSubscribed = tabData.selectedCharacteristic 
      ? tabData.subscribedUuids.some(
          (uuid) => normalizeUuid(uuid) === normalizeUuid(tabData.selectedCharacteristic!.uuid)
        )
      : false;

    return (
      <div style={{ display: 'flex', height: '100%', gap: 8, overflow: 'hidden' }}>
        {!preferences.gattCollapsed && (
          <div style={{ flex: '0 0 280px', minWidth: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden', background: 'var(--bg-secondary)', borderRadius: 8, padding: 8 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8, flexShrink: 0 }}>
              <Text strong style={{ fontSize: 13 }}>{t('title.gattTree')}</Text>
              <Button
                type="text"
                size="small"
                icon={<MenuFoldOutlined />}
                onClick={() => updatePreferences({ gattCollapsed: true })}
                title={t('tooltip.collapseTree')}
              />
            </div>
            <div style={{ flex: '1 1 0', overflow: 'auto', padding: '0 4px' }}>
              {tabData.discoveringServices ? (
                <div style={{ textAlign: 'center', padding: 40 }}>
                  <Spin description={t('status.discovering')} />
                </div>
              ) : tabData.services.length === 0 ? (
                <Empty description={t('placeholder.noServices')} />
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

        {preferences.gattCollapsed && (
          <Button
            type="text"
            icon={<MenuUnfoldOutlined />}
            onClick={() => updatePreferences({ gattCollapsed: false })}
            style={{ flexShrink: 0, alignSelf: 'flex-start' }}
            title={t('tooltip.expandTree')}
          />
        )}

        <div style={{ flex: '1 1 0', minWidth: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <div style={{ flexShrink: 0, marginBottom: 8 }}>
            <CharacteristicPanel
              characteristic={tabData.selectedCharacteristic}
              isSubscribed={isSubscribed}
              collapsed={preferences.panelCollapsed}
              onToggleCollapse={() => updatePreferences({ panelCollapsed: !preferences.panelCollapsed })}
              inputFormat={preferences.inputFormat}
              withoutResponse={preferences.withoutResponse}
              onInputFormatChange={(value) => updatePreferences({ inputFormat: value })}
              onWithoutResponseChange={(value) => updatePreferences({ withoutResponse: value })}
              onRead={(uuid) => handleReadForDevice(uuid, tabData.deviceId)}
              onWrite={(uuid, data, fmt, wnr) => handleWriteForDevice(uuid, data, fmt, wnr, tabData.deviceId)}
              onSubscribe={(uuid) => handleSubscribeForDevice(uuid, tabData.deviceId)}
              onUnsubscribe={(uuid) => handleUnsubscribeForDevice(uuid, tabData.deviceId)}
            />
          </div>

          <Card
            size="small"
            style={{ flex: '1 1 0', display: 'flex', flexDirection: 'column', minHeight: 100 }}
            styles={{ body: { flex: 1, display: 'flex', flexDirection: 'column', padding: 8, overflow: 'hidden', minHeight: 0 } }}
            title={<span>{t('title.dataView')}</span>}
            extra={
              <Space>
                <Tooltip title={preferences.autoScroll ? t('tooltip.autoScrollOn') : t('tooltip.autoScrollOff')}>
                  <Button
                    type={preferences.autoScroll ? 'primary' : 'default'}
                    icon={<VerticalAlignBottomOutlined />}
                    onClick={() => updatePreferences({ autoScroll: !preferences.autoScroll })}
                    size="small"
                  />
                </Tooltip>
                <Segmented
                  value={preferences.displayFormat}
                  onChange={(value) => updatePreferences({ displayFormat: value as 'hex' | 'text' })}
                  size="small"
                  options={[
                    { value: 'text', label: 'TEXT' },
                    { value: 'hex', label: 'HEX' },
                  ]}
                />
                <Button
                  size="small"
                  icon={<DownloadOutlined />}
                  onClick={() => handleExportLog(tabData)}
                  disabled={tabData.logs.length === 0}
                >
                  {t('button.export')}
                </Button>
                <Button
                  size="small"
                  icon={<ClearOutlined />}
                  onClick={() => handleClearLogs(tabData.deviceId)}
                  disabled={tabData.logs.length === 0}
                >
                  {t('button.clear')}
                </Button>
              </Space>
            }
          >
            <TextArea
              ref={(el) => { if (el) logContainerRefs.current[tabData.deviceId] = el; }}
              value={tabData.logs
                .map((entry) => {
                  const timestamp = formatTimestamp(entry.timestamp, effectiveTimezone);
                  const dir = entry.direction;
                  let content = '';
                  if (entry.data) {
                    content = `[${entry.data.length} byte] ${formatBleData(entry.data, preferences.displayFormat)}`;
                  } else if (entry.text) {
                    content = entry.text;
                  }
                  const charInfo = entry.characteristicUuid ? ` [${getShortUuid(entry.characteristicUuid)}]` : '';
                  return `[${timestamp}][${dir}]${charInfo} ${content}`;
                })
                .join('\n')}
              style={{
                flex: '1 1 0',
                overflow: 'auto',
                background: 'var(--bg-primary)',
                padding: 8,
                borderRadius: 4,
                fontFamily: 'Consolas, Monaco, monospace',
                fontSize: 13,
                lineHeight: 1.4,
                minHeight: 0,
                resize: 'none',
              }}
              readOnly
              placeholder={tabData.logs.length === 0 ? t('placeholder.noLogs') : ''}
            />
          </Card>
        </div>
      </div>
    );
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden', padding: 8 }}>
      {error && (
        <Alert
          message={t('common:common.error')}
          description={error}
          type="error"
          closable
          onClose={() => {}}
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      {isConnecting && (
        <Alert
          message={t('status.connecting')}
          type="info"
          showIcon
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      <div style={{ flex: '1 1 0', minHeight: 0, overflow: 'hidden' }}>
        {currentDevice && currentTab
          ? renderDeviceContent(currentTab)
          : renderScanContent()
        }
      </div>
    </div>
  );
};

export default BlePage;
