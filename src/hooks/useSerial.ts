import { useCallback, useRef } from 'react';
import { message } from 'antd';
import { serialApi } from '../api/tauri';
import { useSerialStore, generateId } from '../stores/serialStore';
import { useLogStore } from '../stores/logStore';
import { DEFAULT_SERIAL_CONFIG } from '../types';
import type { SerialConfig } from '../types';
import type { CacheData } from '../api/types';

export const useSerial = () => {
  const {
    ports,
    tabs,
    activeTabKey,
    isScanning,
    error,
    preferences,
    setPorts,
    addPortTab,
    removeTab,
    setActiveTab,
    updateTab,
    addSentData,
    setIsScanning,
    setError,
    hasPortTab,
    getPortTab,
    updatePreferences,
  } = useSerialStore();

  const addLog = useLogStore((state) => state.addLog);

  const autoScanRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const scanPorts = useCallback(async () => {
    setIsScanning(true);
    setError(null);
    try {
      const portList = await serialApi.listPorts();
      setPorts(portList);
      if (portList.length === 0) {
        message.warning('未找到可用串口');
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '扫描串口失败';
      setError(errorMsg);
      addLog('error', 'SerialManager', `扫描串口失败: ${errorMsg}`);
      message.error(errorMsg);
    } finally {
      setIsScanning(false);
    }
  }, [setIsScanning, setError, setPorts, addLog]);

  const openPort = useCallback(async (portName: string, portConfig?: SerialConfig) => {
    const config = portConfig || DEFAULT_SERIAL_CONFIG;
    setError(null);
    
    const existingTab = getPortTab(portName);
    if (existingTab) {
      if (existingTab.isConnected) {
        message.warning(`串口 ${portName} 已在连接中`);
        setActiveTab(existingTab.key);
        return existingTab.key;
      }
      try {
        await serialApi.openPort(portName, config);
        updateTab(existingTab.key, { isConnected: true, openedAt: Date.now(), config });
        setActiveTab(existingTab.key);
        addLog('info', 'SerialManager', `串口 ${portName} 已打开`);
        message.success(`串口 ${portName} 已打开`);
        return existingTab.key;
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : '打开串口失败';
        setError(errorMsg);
        addLog('error', 'SerialManager', `打开串口 ${portName} 失败: ${errorMsg}`);
        message.error(errorMsg);
        throw err;
      }
    }
    
    try {
      await serialApi.openPort(portName, config);
      const key = addPortTab(portName, config);
      updateTab(key, { isConnected: true, openedAt: Date.now() });
      addLog('info', 'SerialManager', `串口 ${portName} 已打开`);
      message.success(`串口 ${portName} 已打开`);

      try {
        const cacheData: CacheData = await serialApi.getCache(portName);
        const store = useSerialStore.getState();
        
        for (const entry of cacheData.tx || []) {
          store.addSentData(portName, {
            id: generateId(),
            timestamp: entry.timestamp,
            data: entry.data,
            direction: 'send',
            format: 'hex',
          });
        }
        
        for (const entry of cacheData.rx || []) {
          store.addReceivedData(portName, {
            id: generateId(),
            timestamp: entry.timestamp,
            data: entry.data,
            direction: 'receive',
            format: 'hex',
          });
        }
      } catch (cacheErr) {
        console.debug('[useSerial] 获取缓存数据失败或无缓存:', cacheErr);
      }

      return key;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '打开串口失败';
      setError(errorMsg);
      addLog('error', 'SerialManager', `打开串口 ${portName} 失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [setError, addPortTab, updateTab, setActiveTab, getPortTab, addLog]);

  const closePort = useCallback(async (tabKey: string) => {
    const tab = tabs.find((t) => t.key === tabKey);
    if (!tab) return;

    try {
      await serialApi.closePort(tab.portName);
      updateTab(tabKey, { isConnected: false });
      addLog('info', 'SerialManager', `串口 ${tab.portName} 已关闭`);
      message.success(`串口 ${tab.portName} 已关闭`);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '关闭串口失败';
      setError(errorMsg);
      addLog('error', 'SerialManager', `关闭串口 ${tab.portName} 失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [tabs, setError, updateTab, addLog]);

  const sendData = useCallback(async (tabKey: string, data: string, format: 'hex' | 'text' = 'text') => {
    const tab = tabs.find((t) => t.key === tabKey);
    if (!tab || !tab.isConnected) {
      message.warning('串口未连接');
      return;
    }

    const bytes: number[] = [];
    if (format === 'hex') {
      const hex = data.replace(/\s+/g, '');
      for (let i = 0; i < hex.length; i += 2) {
        bytes.push(parseInt(hex.substr(i, 2), 16));
      }
    } else {
      bytes.push(...Array.from(new TextEncoder().encode(data)));
    }

    if (bytes.length === 0) {
      message.warning('发送数据不能为空');
      return;
    }

    try {
      await serialApi.sendData(tab.portName, bytes);
      addSentData(tab.portName, {
        id: generateId(),
        timestamp: Date.now(),
        data: bytes,
        direction: 'send',
        format,
      });
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '发送数据失败';
      setError(errorMsg);
      addLog('error', 'SerialManager', `发送数据失败: ${errorMsg}`);
      message.error(errorMsg);
      throw err;
    }
  }, [tabs, setError, addSentData, addLog]);

  const clearTabData = useCallback((tabKey: string) => {
    updateTab(tabKey, { receivedData: [], sentData: [] });
  }, [updateTab]);

  const updateTabConfig = useCallback((tabKey: string, newConfig: Partial<typeof tabs[0]['config']>) => {
    const tab = tabs.find((t) => t.key === tabKey);
    if (tab) {
      updateTab(tabKey, { config: { ...tab.config, ...newConfig } });
    }
  }, [tabs, updateTab]);

  const toggleTabSettings = useCallback((tabKey: string) => {
    const tab = tabs.find((t) => t.key === tabKey);
    if (tab) {
      updateTab(tabKey, { settingsCollapsed: !tab.settingsCollapsed });
    }
  }, [tabs, updateTab]);

  const restoreConnectedPorts = useCallback(async () => {
    try {
      const openPorts: string[] = await serialApi.getOpenPorts();
      for (const portName of openPorts) {
        if (!hasPortTab(portName)) {
          addPortTab(portName);
          updateTab(useSerialStore.getState().tabs.find(t => t.portName === portName && t.tabType === 'port')?.key || '', { isConnected: true, openedAt: Date.now() });
        } else {
          const tab = getPortTab(portName);
          if (tab && !tab.isConnected) {
            updateTab(tab.key, { isConnected: true, openedAt: Date.now() });
          }
        }
      }
      addLog('info', 'SerialManager', `已恢复 ${openPorts.length} 个已连接串口`);
    } catch (err) {
      addLog('error', 'SerialManager', `恢复已连接串口失败: ${err instanceof Error ? err.message : '未知错误'}`);
    }
  }, [addPortTab, updateTab, hasPortTab, getPortTab, addLog]);

  const startAutoScan = useCallback((intervalMs: number = 3000) => {
    if (autoScanRef.current) return;
    autoScanRef.current = setInterval(async () => {
      try {
        const portList = await serialApi.listPorts();
        const currentPorts = useSerialStore.getState().ports;
        const currentNames = new Set(currentPorts.map(p => p.name));
        const newPorts = portList.filter(p => !currentNames.has(p.name));
        if (newPorts.length > 0) {
          setPorts(portList);
          addLog('info', 'SerialManager', `检测到新串口: ${newPorts.map(p => p.name).join(', ')}`);
        } else {
          const removedNames = [...currentNames].filter(name => !portList.some(p => p.name === name));
          if (removedNames.length > 0) {
            setPorts(portList);
          }
        }
        const openPorts: string[] = await serialApi.getOpenPorts();
        const store = useSerialStore.getState();
        for (const portName of openPorts) {
          const existingTab = store.tabs.find(t => t.portName === portName && t.tabType === 'port');
          if (!existingTab) {
            store.addPortTab(portName);
            const newTab = store.tabs.find(t => t.portName === portName && t.tabType === 'port');
            if (newTab) {
              store.updateTab(newTab.key, { isConnected: true, openedAt: Date.now() });
            }
          } else if (!existingTab.isConnected) {
            store.updateTab(existingTab.key, { isConnected: true, openedAt: Date.now() });
          }
        }
      } catch {
      }
    }, intervalMs);
  }, [setPorts, addLog]);

  const stopAutoScan = useCallback(() => {
    if (autoScanRef.current) {
      clearInterval(autoScanRef.current);
      autoScanRef.current = null;
    }
  }, []);

  const activeTab = tabs.find((t) => t.key === activeTabKey);

  return {
    ports,
    tabs,
    activeTab,
    activeTabKey,
    isScanning,
    error,
    scanPorts,
    openPort,
    closePort,
    sendData,
    clearTabData,
    updateTabConfig,
    toggleTabSettings,
    setActiveTab,
    removeTab,
    setError,
    hasPortTab,
    preferences,
    updatePreferences,
    restoreConnectedPorts,
    startAutoScan,
    stopAutoScan,
  };
};
