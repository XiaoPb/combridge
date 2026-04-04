import { useEffect, useCallback, useRef } from 'react';
import { message } from 'antd';
import { bleApi } from '../api/tauri';
import { onBleData, onBleConnected, onBleDisconnected, onBleError, onBleScanResult, onBleModeChanged } from '../api/events';
import { useBleStore, generateBleId, parseBleData, type BleMode } from '../stores/bleStore';
import { useLogStore } from '../stores/logStore';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { BleScanOptions, BleConnection } from '../types';

const handleBleError = (operation: string, params: Record<string, unknown>, error: unknown): string => {
  const errorMsg = error instanceof Error ? error.message : String(error);
  console.error(`[useBle] ${operation} 失败:`, { params, error: errorMsg });
  return errorMsg;
};

let globalBleListeners: UnlistenFn[] = [];
let bleListenerCount = 0;
let bleListenerSetupPromise: Promise<void> | null = null;

async function setupGlobalBleListeners() {
  if (bleListenerSetupPromise) {
    await bleListenerSetupPromise;
  }

  if (bleListenerCount > 0) {
    bleListenerCount++;
    return;
  }

  bleListenerSetupPromise = (async () => {
    const unlistenData = await onBleData((event) => {
      console.debug('[useBle] 收到BLE数据:', {
        deviceId: event.deviceId,
        characteristicUuid: event.characteristicUuid,
        dataLen: event.data?.length,
        timestamp: event.timestamp,
      });
      const store = useBleStore.getState();
      store.addNotification({
        id: generateBleId(),
        deviceId: event.deviceId,
        characteristicUuid: event.characteristicUuid,
        data: event.data,
        timestamp: event.timestamp,
      });
    });

    const unlistenConnected = await onBleConnected((event) => {
      const store = useBleStore.getState();
      store.addConnection({
        deviceId: event.deviceId,
        address: event.address,
        name: event.name,
        isConnected: true,
        services: [],
        connectedAt: Date.now(),
      });
      store.setIsConnecting(false);
      useLogStore.getState().addLog('info', 'BleManager', `设备 ${event.name || event.address} 已连接`);
      message.success(`设备 ${event.name || event.address} 已连接`);
    });

    const unlistenDisconnected = await onBleDisconnected((event) => {
      const store = useBleStore.getState();
      store.removeConnection(event.deviceId);
      if (store.currentDevice === event.deviceId) {
        store.setCurrentDevice(null);
        store.clearServices();
        store.clearCharacteristics();
      }
      useLogStore.getState().addLog('info', 'BleManager', `设备 ${event.address} 已断开`);
      message.info(`设备 ${event.address} 已断开`);
    });

    const unlistenError = await onBleError((event) => {
      const errorMsg = event.error;
      const store = useBleStore.getState();
      store.setError(errorMsg);
      store.setIsConnecting(false);
      store.setIsScanning(false);
      useLogStore.getState().addLog('error', 'BleManager', `BLE错误: ${errorMsg}`);
      message.error(`BLE错误: ${errorMsg}`);
    });

    const unlistenScanResult = await onBleScanResult((device: unknown) => {
      const deviceInfo = device as {
        address: string;
        name?: string;
        rssi?: number;
        isConnectable: boolean;
        services?: string[];
      };
      const store = useBleStore.getState();
      store.addDevice({
        address: deviceInfo.address,
        name: deviceInfo.name,
        rssi: deviceInfo.rssi,
        isConnectable: deviceInfo.isConnectable,
        services: deviceInfo.services,
        discoveredAt: Date.now(),
      });
    });

    const unlistenModeChanged = await onBleModeChanged((event) => {
      const store = useBleStore.getState();
      store.setMode(event.mode);
      store.setSerialPort(event.serialPort || null);
      useLogStore.getState().addLog('info', 'BleManager', `BLE模式已切换为 ${event.mode}`);
      message.info(`BLE模式已切换为 ${event.mode}`);
    });

    globalBleListeners = [
      unlistenData,
      unlistenConnected,
      unlistenDisconnected,
      unlistenError,
      unlistenScanResult,
      unlistenModeChanged,
    ];

    bleListenerCount++;
    bleListenerSetupPromise = null;
  })();

  await bleListenerSetupPromise;
}

async function cleanupGlobalBleListeners() {
  if (bleListenerSetupPromise) {
    await bleListenerSetupPromise;
  }

  bleListenerCount--;
  if (bleListenerCount <= 0) {
    bleListenerCount = 0;
    globalBleListeners.forEach((unlisten) => unlisten());
    globalBleListeners = [];
  }
}

export const useBle = () => {
  const {
    mode,
    serialPort,
    devices,
    connections,
    currentDevice,
    services,
    characteristics,
    notifications,
    isScanning,
    isConnecting,
    isConfigured,
    error,
    setMode,
    setSerialPort,
    setDevices,
    clearDevices,
    addConnection,
    removeConnection,
    setCurrentDevice,
    setServices,
    clearServices,
    setCharacteristics,
    updateCharacteristic,
    clearCharacteristics,
    clearNotifications,
    setIsScanning,
    setIsConnecting,
    setIsConfigured,
    setError,
  } = useBleStore();

  const addLog = useLogStore((state) => state.addLog);
  const scanTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setupGlobalBleListeners();

    return () => {
      cleanupGlobalBleListeners();
      if (scanTimeoutRef.current) {
        clearTimeout(scanTimeoutRef.current);
      }
    };
  }, []);

  const configure = useCallback(async (newMode: BleMode, port?: string) => {
    setError(null);
    try {
      await bleApi.configureBle(newMode, port);
      setMode(newMode);
      setSerialPort(port || null);
      addLog('info', 'BleManager', `BLE模式已配置为 ${newMode}`);
      message.success(`BLE模式已配置为 ${newMode}`);
    } catch (err) {
      const errorMsg = handleBleError('configure', { newMode, port }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `配置BLE模式失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [setError, setMode, setSerialPort, addLog]);

  const scanDevices = useCallback(async (options?: BleScanOptions) => {
    setIsScanning(true);
    setError(null);
    clearDevices();

    addLog('info', 'BleManager', '开始扫描BLE设备');

    try {
      if (!isConfigured) {
        await bleApi.configureBle(mode, serialPort || undefined);
        setIsConfigured(true);
      }

      const deviceList = await bleApi.scanBleDevices(options);
      setDevices(deviceList);

      if (options?.timeout) {
        scanTimeoutRef.current = setTimeout(() => {
          setIsScanning(false);
        }, options.timeout);
      }

      if (deviceList.length === 0) {
        message.info('未扫描到BLE设备');
      } else {
        addLog('info', 'BleManager', `扫描完成，发现 ${deviceList.length} 个设备`);
        message.success(`扫描到 ${deviceList.length} 个设备`);
      }
    } catch (err) {
      const errorMsg = handleBleError('scanDevices', { options }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `扫描BLE设备失败: ${errorMsg}`);
      message.error(errorMsg);
    } finally {
      if (!options?.timeout) {
        setIsScanning(false);
      }
    }
  }, [setIsScanning, setError, clearDevices, setDevices, setIsConfigured, isConfigured, mode, serialPort, addLog]);

  const stopScan = useCallback(async () => {
    setIsScanning(false);
    if (scanTimeoutRef.current) {
      clearTimeout(scanTimeoutRef.current);
      scanTimeoutRef.current = null;
    }
    addLog('info', 'BleManager', '扫描已停止');
    message.info('扫描已停止');
  }, [setIsScanning, addLog]);

  const connectDevice = useCallback(async (address: string) => {
    setIsConnecting(true);
    setError(null);

    try {
      const connection = await bleApi.connectBle(address);
      addConnection(connection);
      setCurrentDevice(address);
      addLog('info', 'BleManager', `已连接到 ${connection.name || address}`);
      message.success(`已连接到 ${connection.name || address}`);
      return connection;
    } catch (err) {
      const errorMsg = handleBleError('connectDevice', { address }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `连接设备 ${address} 失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    } finally {
      setIsConnecting(false);
    }
  }, [setIsConnecting, setError, addConnection, setCurrentDevice, addLog]);

  const disconnectDevice = useCallback(async (deviceId: string) => {
    setError(null);

    try {
      await bleApi.disconnectBle(deviceId);
      removeConnection(deviceId);
      if (currentDevice === deviceId) {
        setCurrentDevice(null);
        clearServices();
        clearCharacteristics();
      }
      addLog('info', 'BleManager', `设备 ${deviceId} 已断开`);
      message.success('设备已断开');
    } catch (err) {
      const errorMsg = handleBleError('disconnectDevice', { deviceId }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `断开设备 ${deviceId} 失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [setError, removeConnection, currentDevice, setCurrentDevice, clearServices, clearCharacteristics, addLog]);

  const discoverServices = useCallback(async (deviceId?: string) => {
    const targetDevice = deviceId || currentDevice;
    if (!targetDevice) {
      message.warning('请先选择已连接的设备');
      return;
    }

    setError(null);
    try {
      const serviceList = await bleApi.discoverBleServices(targetDevice);
      setServices(serviceList);
      addLog('info', 'BleManager', `发现 ${serviceList.length} 个服务`);
      message.success(`发现 ${serviceList.length} 个服务`);
      return serviceList;
    } catch (err) {
      const errorMsg = handleBleError('discoverServices', { deviceId: targetDevice }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `发现服务失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError, setServices, addLog]);

  const discoverCharacteristics = useCallback(async (serviceUuid: string, deviceId?: string) => {
    const targetDevice = deviceId || currentDevice;
    if (!targetDevice) {
      message.warning('请先选择已连接的设备');
      return;
    }

    setError(null);
    try {
      const charList = await bleApi.discoverBleCharacteristics(targetDevice, serviceUuid);
      setCharacteristics(charList);
      addLog('info', 'BleManager', `发现 ${charList.length} 个特征`);
      message.success(`发现 ${charList.length} 个特征`);
      return charList;
    } catch (err) {
      const errorMsg = handleBleError('discoverCharacteristics', { deviceId: targetDevice, serviceUuid }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `发现特征失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError, setCharacteristics, addLog]);

  const readCharacteristic = useCallback(async (characteristicUuid: string, deviceId?: string) => {
    const targetDevice = deviceId || currentDevice;
    if (!targetDevice) {
      message.warning('请先选择已连接的设备');
      return;
    }

    setError(null);
    try {
      const data = await bleApi.readBleCharacteristic(targetDevice, characteristicUuid);
      updateCharacteristic(characteristicUuid, { value: data });
      message.success('读取成功');
      return data;
    } catch (err) {
      const errorMsg = handleBleError('readCharacteristic', { deviceId: targetDevice, characteristicUuid }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `读取特征失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError, updateCharacteristic, addLog]);

  const writeCharacteristic = useCallback(async (
    characteristicUuid: string,
    data: string,
    format: 'hex' | 'text' = 'text',
    withoutResponse = false,
    deviceId?: string
  ) => {
    const targetDevice = deviceId || currentDevice;
    if (!targetDevice) {
      message.warning('请先选择已连接的设备');
      return;
    }

    const bytes = parseBleData(data, format);
    if (bytes.length === 0) {
      message.warning('写入数据不能为空');
      return;
    }

    setError(null);
    try {
      await bleApi.writeBleCharacteristic(targetDevice, characteristicUuid, bytes, withoutResponse);
      message.success('写入成功');
    } catch (err) {
      const errorMsg = handleBleError('writeCharacteristic', { deviceId: targetDevice, characteristicUuid, data, format, withoutResponse }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `写入特征失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError, addLog]);

  const subscribeNotify = useCallback(async (characteristicUuid: string, deviceId?: string) => {
    const targetDevice = deviceId || currentDevice;
    if (!targetDevice) {
      message.warning('请先选择已连接的设备');
      return;
    }

    setError(null);
    try {
      await bleApi.subscribeBleNotify(targetDevice, characteristicUuid);
      addLog('info', 'BleManager', `已订阅特征 ${characteristicUuid} 的通知`);
      message.success('已订阅通知');
    } catch (err) {
      const errorMsg = handleBleError('subscribeNotify', { deviceId: targetDevice, characteristicUuid }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `订阅通知失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError, addLog]);

  const unsubscribeNotify = useCallback(async (characteristicUuid: string, deviceId?: string) => {
    const targetDevice = deviceId || currentDevice;
    if (!targetDevice) {
      message.warning('请先选择已连接的设备');
      return;
    }

    setError(null);
    try {
      await bleApi.unsubscribeBleNotify(targetDevice, characteristicUuid);
      addLog('info', 'BleManager', `已取消订阅特征 ${characteristicUuid} 的通知`);
      message.success('已取消订阅');
    } catch (err) {
      const errorMsg = handleBleError('unsubscribeNotify', { deviceId: targetDevice, characteristicUuid }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `取消订阅失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError, addLog]);

  const isConnected = useCallback((deviceId: string) => {
    return connections.some((c) => (c.deviceId === deviceId || c.address === deviceId) && c.isConnected);
  }, [connections]);

  const getCurrentConnection = useCallback((): BleConnection | null => {
    if (!currentDevice) return null;
    return connections.find((c) => c.deviceId === currentDevice || c.address === currentDevice) || null;
  }, [currentDevice, connections]);

  const getDeviceByAddress = useCallback((address: string) => {
    return devices.find((d) => d.address === address);
  }, [devices]);

  const restoreConnections = useCallback(async () => {
    try {
      const connectionList = await bleApi.getConnections();
      for (const conn of connectionList) {
        if (conn.isConnected) {
          addConnection(conn);
        }
      }
      return connectionList;
    } catch (err) {
      const errorMsg = handleBleError('restoreConnections', {}, err);
      console.error('[useBle] 恢复连接状态失败:', errorMsg);
      return [];
    }
  }, [addConnection]);

  const restoreSubscriptions = useCallback(async (deviceId: string, charUuids?: string[]) => {
    const uuidsToRestore = charUuids || (await bleApi.getSubscriptions(deviceId).catch(() => []));
    if (!uuidsToRestore || uuidsToRestore.length === 0) {
      return [];
    }

    const restoredUuids: string[] = [];
    for (const charUuid of uuidsToRestore) {
      try {
        await bleApi.subscribeBleNotify(deviceId, charUuid);
        restoredUuids.push(charUuid);
        console.debug('[useBle] 恢复订阅成功:', { deviceId, charUuid });
      } catch (err) {
        console.error('[useBle] 恢复订阅失败:', { deviceId, charUuid, error: err });
      }
    }

    if (restoredUuids.length > 0) {
      console.info('[useBle] 恢复订阅完成:', { deviceId, count: restoredUuids.length });
    }
    return restoredUuids;
  }, []);

  return {
    mode,
    serialPort,
    devices,
    connections,
    currentDevice,
    services,
    characteristics,
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
    clearNotifications,
    setCurrentDevice,
    isConnected,
    getCurrentConnection,
    getDeviceByAddress,
    restoreConnections,
    restoreSubscriptions,
  };
};
