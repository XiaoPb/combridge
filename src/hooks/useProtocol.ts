import { useCallback } from 'react';
import { message } from 'antd';
import { protocolApi } from '../api/tauri';
import { useProtocolStore } from '../stores/protocolStore';
import type { PluginInfo } from '../api/types';

export const useProtocol = () => {
  const {
    protocols,
    bindings,
    currentProtocol,
    isLoading,
    error,
    setProtocols,
    addProtocol,
    updateProtocol,
    removeProtocol,
    addBinding,
    removeBinding,
    setCurrentProtocol,
    setIsLoading,
    setError,
  } = useProtocolStore();

  const loadProtocols = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const list = await protocolApi.listProtocols();
      setProtocols(list);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '获取协议列表失败';
      setError(errorMsg);
      message.error(errorMsg);
    } finally {
      setIsLoading(false);
    }
  }, [setIsLoading, setError, setProtocols]);

  const loadProtocol = useCallback(async (pluginId: string, path: string): Promise<PluginInfo | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const info = await protocolApi.loadProtocol(pluginId, path);
      addProtocol(info);
      message.success(`协议 ${info.name} 加载成功`);
      return info;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '加载协议失败';
      setError(errorMsg);
      message.error(errorMsg);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [setIsLoading, setError, addProtocol]);

  const unloadProtocol = useCallback(async (pluginId: string): Promise<boolean> => {
    setError(null);
    try {
      await protocolApi.unloadProtocol(pluginId);
      removeProtocol(pluginId);
      message.success('协议已卸载');
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '卸载协议失败';
      setError(errorMsg);
      message.error(errorMsg);
      return false;
    }
  }, [setError, removeProtocol]);

  const enableProtocol = useCallback(async (pluginId: string): Promise<boolean> => {
    setError(null);
    try {
      await protocolApi.enableProtocol(pluginId);
      updateProtocol(pluginId, { state: 'Enabled' });
      message.success('协议已启用');
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '启用协议失败';
      setError(errorMsg);
      message.error(errorMsg);
      return false;
    }
  }, [setError, updateProtocol]);

  const disableProtocol = useCallback(async (pluginId: string): Promise<boolean> => {
    setError(null);
    try {
      await protocolApi.disableProtocol(pluginId);
      updateProtocol(pluginId, { state: 'Disabled' });
      message.success('协议已禁用');
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '禁用协议失败';
      setError(errorMsg);
      message.error(errorMsg);
      return false;
    }
  }, [setError, updateProtocol]);

  const bindProtocol = useCallback(async (pluginId: string, deviceId: string): Promise<boolean> => {
    setError(null);
    try {
      await protocolApi.bindProtocol(pluginId, deviceId);
      addBinding({ pluginId, deviceId, boundAt: Date.now() });
      const protocol = protocols.find((p) => p.id === pluginId);
      if (protocol && !protocol.bound_devices.includes(deviceId)) {
        updateProtocol(pluginId, {
          bound_devices: [...protocol.bound_devices, deviceId],
        });
      }
      message.success('协议绑定成功');
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '绑定协议失败';
      setError(errorMsg);
      message.error(errorMsg);
      return false;
    }
  }, [setError, addBinding, protocols, updateProtocol]);

  const unbindProtocol = useCallback(async (pluginId: string, deviceId: string): Promise<boolean> => {
    setError(null);
    try {
      await protocolApi.unbindProtocol(pluginId, deviceId);
      removeBinding(pluginId, deviceId);
      const protocol = protocols.find((p) => p.id === pluginId);
      if (protocol) {
        updateProtocol(pluginId, {
          bound_devices: protocol.bound_devices.filter((id) => id !== deviceId),
        });
      }
      message.success('协议解绑成功');
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '解绑协议失败';
      setError(errorMsg);
      message.error(errorMsg);
      return false;
    }
  }, [setError, removeBinding, protocols, updateProtocol]);

  const getProtocol = useCallback(async (pluginId: string): Promise<PluginInfo | null> => {
    try {
      return await protocolApi.getProtocol(pluginId);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '获取协议信息失败';
      setError(errorMsg);
      return null;
    }
  }, [setError]);

  const getBoundProtocols = useCallback(async (deviceId: string): Promise<PluginInfo[]> => {
    try {
      return await protocolApi.getBoundProtocols(deviceId);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '获取绑定协议失败';
      setError(errorMsg);
      return [];
    }
  }, [setError]);

  return {
    protocols,
    bindings,
    currentProtocol,
    isLoading,
    error,
    loadProtocols,
    loadProtocol,
    unloadProtocol,
    enableProtocol,
    disableProtocol,
    bindProtocol,
    unbindProtocol,
    getProtocol,
    getBoundProtocols,
    setCurrentProtocol,
  };
};
