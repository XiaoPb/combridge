import { useEffect, useCallback, useRef } from 'react';
import { message } from 'antd';
import { bleApi } from '../api/tauri';
import { onBleData, onBleConnected, onBleDisconnected, onBleError, onBleScanResult, onBleModeChanged } from '../api/events';
import { useBleStore, generateBleId, parseBleData, type BleMode } from '../stores/bleStore';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { BleScanOptions, BleConnection } from '../types';

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
    error,
    setMode,
    setSerialPort,
    setDevices,
    addDevice,
    clearDevices,
    addConnection,
    removeConnection,
    setCurrentDevice,
    setServices,
    clearServices,
    setCharacteristics,
    updateCharacteristic,
    clearCharacteristics,
    addNotification,
    clearNotifications,
    setIsScanning,
    setIsConnecting,
    setError,
  } = useBleStore();

  const listenersRef = useRef<UnlistenFn[]>([]);
  const scanTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const setupListeners = async () => {
      const unlistenData = await onBleData((event) => {
        addNotification({
          id: generateBleId(),
          deviceId: event.deviceId,
          characteristicUuid: event.characteristicUuid,
          data: event.data,
          timestamp: event.timestamp,
        });
      });

      const unlistenConnected = await onBleConnected((event) => {
        addConnection({
          deviceId: event.deviceId,
          address: event.address,
          name: event.name,
          isConnected: true,
          services: [],
          connectedAt: Date.now(),
        });
        setIsConnecting(false);
        message.success(`设备 ${event.name || event.address} 已连接`);
      });

      const unlistenDisconnected = await onBleDisconnected((event) => {
        removeConnection(event.deviceId);
        if (currentDevice === event.deviceId) {
          setCurrentDevice(null);
          clearServices();
          clearCharacteristics();
        }
        message.info(`设备 ${event.address} 已断开`);
      });

      const unlistenError = await onBleError((event) => {
        const errorMsg = event.error;
        setError(errorMsg);
        setIsConnecting(false);
        setIsScanning(false);
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
        addDevice({
          address: deviceInfo.address,
          name: deviceInfo.name,
          rssi: deviceInfo.rssi,
          isConnectable: deviceInfo.isConnectable,
          services: deviceInfo.services,
          discoveredAt: Date.now(),
        });
      });

      const unlistenModeChanged = await onBleModeChanged((event) => {
        setMode(event.mode);
        setSerialPort(event.serialPort || null);
        message.info(`BLE模式已切换为 ${event.mode}`);
      });

      listenersRef.current = [
        unlistenData,
        unlistenConnected,
        unlistenDisconnected,
        unlistenError,
        unlistenScanResult,
        unlistenModeChanged,
      ];
    };

    setupListeners();

    return () => {
      listenersRef.current.forEach((unlisten) => unlisten());
      listenersRef.current = [];
      if (scanTimeoutRef.current) {
        clearTimeout(scanTimeoutRef.current);
      }
    };
  }, [addNotification, addConnection, removeConnection, currentDevice, setError, setIsConnecting, setIsScanning, addDevice, setMode, setSerialPort, setCurrentDevice, clearServices, clearCharacteristics]);

  const configure = useCallback(async (newMode: BleMode, port?: string) => {
    setError(null);
    try {
      await bleApi.configureBle(newMode, port);
      setMode(newMode);
      setSerialPort(port || null);
      message.success(`BLE模式已配置为 ${newMode}`);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '配置BLE失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [setError, setMode, setSerialPort]);

  const scanDevices = useCallback(async (options?: BleScanOptions) => {
    setIsScanning(true);
    setError(null);
    clearDevices();

    try {
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
        message.success(`扫描到 ${deviceList.length} 个设备`);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '扫描设备失败';
      setError(errorMsg);
      message.error(errorMsg);
    } finally {
      if (!options?.timeout) {
        setIsScanning(false);
      }
    }
  }, [setIsScanning, setError, clearDevices, setDevices]);

  const stopScan = useCallback(async () => {
    setIsScanning(false);
    if (scanTimeoutRef.current) {
      clearTimeout(scanTimeoutRef.current);
      scanTimeoutRef.current = null;
    }
    message.info('扫描已停止');
  }, [setIsScanning]);

  const connectDevice = useCallback(async (address: string) => {
    setIsConnecting(true);
    setError(null);

    try {
      const connection = await bleApi.connectBle(address);
      addConnection(connection);
      setCurrentDevice(connection.deviceId);
      message.success(`已连接到 ${connection.name || address}`);
      return connection;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '连接设备失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    } finally {
      setIsConnecting(false);
    }
  }, [setIsConnecting, setError, addConnection, setCurrentDevice]);

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
      message.success('设备已断开');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '断开连接失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [setError, removeConnection, currentDevice, setCurrentDevice, clearServices, clearCharacteristics]);

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
      message.success(`发现 ${serviceList.length} 个服务`);
      return serviceList;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '发现服务失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError, setServices]);

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
      message.success(`发现 ${charList.length} 个特征`);
      return charList;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '发现特征失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError, setCharacteristics]);

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
      const errorMsg = err instanceof Error ? err.message : '读取特征失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError, updateCharacteristic]);

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
      const errorMsg = err instanceof Error ? err.message : '写入特征失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError]);

  const subscribeNotify = useCallback(async (characteristicUuid: string, deviceId?: string) => {
    const targetDevice = deviceId || currentDevice;
    if (!targetDevice) {
      message.warning('请先选择已连接的设备');
      return;
    }

    setError(null);
    try {
      await bleApi.subscribeBleNotify(targetDevice, characteristicUuid);
      message.success('已订阅通知');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '订阅通知失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError]);

  const unsubscribeNotify = useCallback(async (characteristicUuid: string, deviceId?: string) => {
    const targetDevice = deviceId || currentDevice;
    if (!targetDevice) {
      message.warning('请先选择已连接的设备');
      return;
    }

    setError(null);
    try {
      await bleApi.unsubscribeBleNotify(targetDevice, characteristicUuid);
      message.success('已取消订阅');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '取消订阅失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [currentDevice, setError]);

  const isConnected = useCallback((deviceId: string) => {
    return connections.some((c) => c.deviceId === deviceId && c.isConnected);
  }, [connections]);

  const getCurrentConnection = useCallback((): BleConnection | null => {
    if (!currentDevice) return null;
    return connections.find((c) => c.deviceId === currentDevice) || null;
  }, [currentDevice, connections]);

  const getDeviceByAddress = useCallback((address: string) => {
    return devices.find((d) => d.address === address);
  }, [devices]);

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
  };
};
