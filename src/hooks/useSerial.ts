import { useEffect, useCallback, useRef } from 'react';
import { message } from 'antd';
import { serialApi } from '../api/tauri';
import { onSerialData, onSerialError, onSerialConnected, onSerialDisconnected } from '../api/events';
import { useSerialStore, generateId, parseData } from '../stores/serialStore';
import type { UnlistenFn } from '@tauri-apps/api/event';

let globalSerialListeners: {
  data?: UnlistenFn;
  error?: UnlistenFn;
  connected?: UnlistenFn;
  disconnected?: UnlistenFn;
} = {};

let listenerCount = 0;

async function setupGlobalListeners(
  addReceivedData: (entry: any) => void,
  setError: (error: string | null) => void,
  removeOpenPort: (portName: string) => void
) {
  if (listenerCount > 0) {
    listenerCount++;
    return;
  }

  globalSerialListeners.data = await onSerialData((event) => {
    addReceivedData({
      id: generateId(),
      timestamp: event.timestamp ?? Date.now(),
      data: event.data,
      direction: 'receive',
      format: 'hex',
    });
  });

  globalSerialListeners.error = await onSerialError((event) => {
    setError(event.error);
    message.error(`串口错误: ${event.error}`);
  });

  globalSerialListeners.connected = await onSerialConnected((portName) => {
    message.success(`串口 ${portName} 已连接`);
  });

  globalSerialListeners.disconnected = await onSerialDisconnected((portName) => {
    removeOpenPort(portName);
    message.info(`串口 ${portName} 已断开`);
  });

  listenerCount++;
}

async function cleanupGlobalListeners() {
  listenerCount--;
  if (listenerCount <= 0) {
    listenerCount = 0;
    if (globalSerialListeners.data) {
      globalSerialListeners.data();
      globalSerialListeners.data = undefined;
    }
    if (globalSerialListeners.error) {
      globalSerialListeners.error();
      globalSerialListeners.error = undefined;
    }
    if (globalSerialListeners.connected) {
      globalSerialListeners.connected();
      globalSerialListeners.connected = undefined;
    }
    if (globalSerialListeners.disconnected) {
      globalSerialListeners.disconnected();
      globalSerialListeners.disconnected = undefined;
    }
  }
}

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

  const isMountedRef = useRef(false);

  useEffect(() => {
    if (isMountedRef.current) return;
    isMountedRef.current = true;

    setupGlobalListeners(addReceivedData, setError, removeOpenPort);

    return () => {
      isMountedRef.current = false;
      cleanupGlobalListeners();
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
