import { useCallback, useRef } from 'react';
import { message } from 'antd';
import { bleApi } from '../api/tauri';
import { useBleStore, parseBleData, type BleMode } from '../stores/bleStore';
import { useLogStore } from '../stores/logStore';
import type { BleScanOptions, BleConnection } from '../types';
import i18n from '../i18n';
import { formatErrorMessage } from '../utils/errorMessage';

const handleBleError = (operation: string, params: Record<string, unknown>, error: unknown): string => {
  const errorMsg = formatErrorMessage(error, operation);
  if (import.meta.env.DEV) {
    console.error(`[useBle] ${operation} 失败:`, { params, error: errorMsg });
  }
  return errorMsg;
};

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
    clearDisconnectedDevice,
    setCurrentDevice,
    setServices,
    setCharacteristics,
    updateCharacteristic,
    clearNotifications,
    setIsScanning,
    setIsConnecting,
    setIsConfigured,
    setError,
  } = useBleStore();

  const addLog = useLogStore((state) => state.addLog);
  const scanTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const scanRequestIdRef = useRef(0);

  const configure = useCallback(async (newMode: BleMode, port?: string) => {
    setError(null);
    try {
      await bleApi.configureBle(newMode, port);
      setMode(newMode);
      setSerialPort(port || null);
      addLog('info', 'BleManager', `BLE模式已配置为 ${newMode}`);
      message.success(`BLE模式已配置为 ${newMode}`);
    } catch (err) {
      const errorMsg = handleBleError(i18n.t('ble:message.configureFailed'), { newMode, port }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `配置BLE模式失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [setError, setMode, setSerialPort, addLog]);

  const scanDevices = useCallback(async (options?: BleScanOptions) => {
    const requestId = scanRequestIdRef.current + 1;
    scanRequestIdRef.current = requestId;
    setIsScanning(true);
    setError(null);
    clearDevices();

    addLog('info', 'BleManager', '开始扫描BLE设备');

    try {
      if (!isConfigured) {
        await bleApi.configureBle(mode, serialPort || undefined);
        setIsConfigured(true);
      }

      if (options?.timeout) {
        scanTimeoutRef.current = setTimeout(() => {
          scanTimeoutRef.current = null;
          bleApi.stopBleScan()
            .then((deviceList) => {
              if (scanRequestIdRef.current === requestId) {
                setDevices(deviceList);
              }
            })
            .catch(() => {})
            .finally(() => {
              if (scanRequestIdRef.current === requestId) {
                setIsScanning(false);
              }
            });
        }, options.timeout);
      }

      const deviceList = await bleApi.scanBleDevices(options);
      if (scanRequestIdRef.current !== requestId) {
        return deviceList;
      }
      setDevices(deviceList);

      if (scanTimeoutRef.current) {
        clearTimeout(scanTimeoutRef.current);
        scanTimeoutRef.current = null;
      }

      setIsScanning(false);

      if (deviceList.length === 0) {
        message.info('未扫描到BLE设备');
      } else {
        addLog('info', 'BleManager', `扫描完成，发现 ${deviceList.length} 个设备`);
        message.success(`扫描到 ${deviceList.length} 个设备`);
      }
    } catch (err) {
      if (scanRequestIdRef.current !== requestId) {
        return [];
      }
      const errorMsg = handleBleError(i18n.t('ble:message.scanFailed'), { options }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `扫描BLE设备失败: ${errorMsg}`);
      message.error(errorMsg);
      setIsScanning(false);
    } finally {
      if (scanTimeoutRef.current) {
        clearTimeout(scanTimeoutRef.current);
        scanTimeoutRef.current = null;
      }
      return [];
    }
  }, [setIsScanning, setError, clearDevices, setDevices, setIsConfigured, isConfigured, mode, serialPort, addLog]);

  const stopScan = useCallback(async () => {
    scanRequestIdRef.current += 1;
    if (scanTimeoutRef.current) {
      clearTimeout(scanTimeoutRef.current);
      scanTimeoutRef.current = null;
    }
    try {
      const deviceList = await bleApi.stopBleScan();
      setDevices(deviceList);
    } catch {
      // best-effort stop
    } finally {
      setIsScanning(false);
    }
    addLog('info', 'BleManager', '扫描已停止');
    message.info('扫描已停止');
  }, [setIsScanning, setDevices, addLog]);

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
      const errorMsg = handleBleError(i18n.t('ble:message.connectFailed'), { address }, err);
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
      clearDisconnectedDevice(deviceId);
      addLog('info', 'BleManager', `设备 ${deviceId} 已断开`);
      message.success('设备已断开');
    } catch (err) {
      const errorMsg = handleBleError(i18n.t('ble:message.disconnectFailed'), { deviceId }, err);
      setError(errorMsg);
      addLog('error', 'BleManager', `断开设备 ${deviceId} 失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [setError, clearDisconnectedDevice, addLog]);

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
      const errorMsg = handleBleError(i18n.t('ble:message.discoverServicesFailed'), { deviceId: targetDevice }, err);
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
      const errorMsg = handleBleError(i18n.t('ble:message.discoverCharacteristicsFailed'), { deviceId: targetDevice, serviceUuid }, err);
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
      const errorMsg = handleBleError(i18n.t('ble:message.readFailed'), { deviceId: targetDevice, characteristicUuid }, err);
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
      const errorMsg = handleBleError(i18n.t('ble:message.writeFailed'), { deviceId: targetDevice, characteristicUuid, data, format, withoutResponse }, err);
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
      const errorMsg = handleBleError(i18n.t('ble:message.subscribeFailed'), { deviceId: targetDevice, characteristicUuid }, err);
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
      const errorMsg = handleBleError(i18n.t('ble:message.unsubscribeFailed'), { deviceId: targetDevice, characteristicUuid }, err);
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
      const errorMsg = handleBleError(i18n.t('ble:message.restoreConnectionsFailed'), {}, err);
      if (import.meta.env.DEV) {
        console.error('[useBle] 恢复连接状态失败:', errorMsg);
      }
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
        if (import.meta.env.DEV) {
          console.debug('[useBle] 恢复订阅成功:', { deviceId, charUuid });
        }
      } catch (err) {
        if (import.meta.env.DEV) {
          console.error('[useBle] 恢复订阅失败:', { deviceId, charUuid, error: err });
        }
      }
    }

    if (restoredUuids.length > 0 && import.meta.env.DEV) {
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
