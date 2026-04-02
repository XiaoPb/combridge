import { useEffect, useCallback, useRef } from 'react';
import { message } from 'antd';
import { serialApi } from '../api/tauri';
import { onSerialData, onSerialError, onSerialConnected, onSerialDisconnected } from '../api/events';
import { useSerialStore, generateId, parseData } from '../stores/serialStore';
import type { UnlistenFn } from '@tauri-apps/api/event';

export const useSerial = () => {
  const {
    ports,
    openPorts,
    currentPort,
    config,
    receivedData,
    sentData,
    isScanning,
    error,
    setPorts,
    addOpenPort,
    removeOpenPort,
    setCurrentPort,
    setConfig,
    addReceivedData,
    addSentData,
    clearAllData,
    setIsScanning,
    setError,
  } = useSerialStore();

  const listenersRef = useRef<UnlistenFn[]>([]);

  useEffect(() => {
    const setupListeners = async () => {
      const unlistenData = await onSerialData((event) => {
        addReceivedData({
          id: generateId(),
          timestamp: event.timestamp,
          data: event.data,
          direction: 'receive',
          format: 'hex',
        });
      });

      const unlistenError = await onSerialError((event) => {
        setError(event.error);
        message.error(`串口错误: ${event.error}`);
      });

      const unlistenConnected = await onSerialConnected((portName) => {
        message.success(`串口 ${portName} 已连接`);
      });

      const unlistenDisconnected = await onSerialDisconnected((portName) => {
        removeOpenPort(portName);
        message.info(`串口 ${portName} 已断开`);
      });

      listenersRef.current = [unlistenData, unlistenError, unlistenConnected, unlistenDisconnected];
    };

    setupListeners();

    return () => {
      listenersRef.current.forEach((unlisten) => unlisten());
      listenersRef.current = [];
    };
  }, [addReceivedData, setError, removeOpenPort]);

  const scanPorts = useCallback(async () => {
    setIsScanning(true);
    setError(null);
    try {
      const portList = await serialApi.scanPorts();
      setPorts(portList);
      if (portList.length === 0) {
        message.warning('未找到可用串口');
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '扫描串口失败';
      setError(errorMsg);
      message.error(errorMsg);
    } finally {
      setIsScanning(false);
    }
  }, [setIsScanning, setError, setPorts]);

  const openPort = useCallback(async (portName: string, portConfig = config) => {
    setError(null);
    try {
      await serialApi.openPort(portName, portConfig);
      addOpenPort({
        portName,
        config: portConfig,
        isConnected: true,
        openedAt: Date.now(),
      });
      setCurrentPort(portName);
      message.success(`串口 ${portName} 已打开`);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '打开串口失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [config, setError, addOpenPort, setCurrentPort]);

  const closePort = useCallback(async (portName: string) => {
    setError(null);
    try {
      await serialApi.closePort(portName);
      removeOpenPort(portName);
      message.success(`串口 ${portName} 已关闭`);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '关闭串口失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [setError, removeOpenPort]);

  const sendData = useCallback(async (data: string, format: 'hex' | 'text' = 'text') => {
    if (!currentPort) {
      message.warning('请先选择并打开串口');
      return;
    }

    const bytes = parseData(data, format);
    if (bytes.length === 0) {
      message.warning('发送数据不能为空');
      return;
    }

    setError(null);
    try {
      await serialApi.sendData(currentPort, bytes);
      addSentData({
        id: generateId(),
        timestamp: Date.now(),
        data: bytes,
        direction: 'send',
        format,
      });
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '发送数据失败';
      setError(errorMsg);
      message.error(errorMsg);
      throw err;
    }
  }, [currentPort, setError, addSentData]);

  const updatePortConfig = useCallback((newConfig: Partial<typeof config>) => {
    setConfig({ ...config, ...newConfig });
  }, [config, setConfig]);

  const isConnected = useCallback((portName: string) => {
    return openPorts.some((p) => p.portName === portName && p.isConnected);
  }, [openPorts]);

  const getCurrentConnection = useCallback(() => {
    if (!currentPort) return null;
    return openPorts.find((p) => p.portName === currentPort) || null;
  }, [currentPort, openPorts]);

  return {
    ports,
    openPorts,
    currentPort,
    config,
    receivedData,
    sentData,
    isScanning,
    error,
    scanPorts,
    openPort,
    closePort,
    sendData,
    clearAllData,
    updatePortConfig,
    setCurrentPort,
    isConnected,
    getCurrentConnection,
  };
};
