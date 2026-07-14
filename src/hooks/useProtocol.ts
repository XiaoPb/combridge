import { useCallback } from 'react';
import { message } from 'antd';
import { protocolApi } from '../api/tauri';
import { useProtocolStore } from '../stores/protocolStore';
import { useLogStore } from '../stores/logStore';
import type { PluginInfo } from '../api/types';
import i18n from '../i18n';
import { formatErrorMessage } from '../utils/errorMessage';

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

  const addLog = useLogStore((state) => state.addLog);

  const loadProtocols = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const list = await protocolApi.listProtocols();
      setProtocols(list);
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('protocol:message.listFailed'));
      setError(errorMsg);
      addLog('error', 'ProtocolManager', `获取协议列表失败: ${errorMsg}`);
      message.error(errorMsg);
    } finally {
      setIsLoading(false);
    }
  }, [setIsLoading, setError, setProtocols, addLog]);

  const loadProtocol = useCallback(async (pluginId: string, path: string): Promise<PluginInfo | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const info = await protocolApi.loadProtocol(pluginId, path);
      addProtocol(info);
      addLog('info', 'ProtocolManager', `协议 ${info.name} 加载成功`);
      message.success(`协议 ${info.name} 加载成功`);
      return info;
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('protocol:message.loadFailed'));
      setError(errorMsg);
      addLog('error', 'ProtocolManager', `加载协议失败: ${errorMsg}`);
      message.error(errorMsg);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [setIsLoading, setError, addProtocol, addLog]);

  const unloadProtocol = useCallback(async (pluginId: string): Promise<boolean> => {
    setError(null);
    try {
      await protocolApi.unloadProtocol(pluginId);
      removeProtocol(pluginId);
      addLog('info', 'ProtocolManager', `协议 ${pluginId} 已卸载`);
      message.success('协议已卸载');
      return true;
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('protocol:message.unloadFailed'));
      setError(errorMsg);
      addLog('error', 'ProtocolManager', `卸载协议失败: ${errorMsg}`);
      message.error(errorMsg);
      return false;
    }
  }, [setError, removeProtocol, addLog]);

  const enableProtocol = useCallback(async (pluginId: string): Promise<boolean> => {
    setError(null);
    try {
      await protocolApi.enableProtocol(pluginId);
      updateProtocol(pluginId, { state: 'Enabled' });
      addLog('info', 'ProtocolManager', `协议 ${pluginId} 已启用`);
      message.success('协议已启用');
      return true;
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('protocol:message.enableFailed'));
      setError(errorMsg);
      addLog('error', 'ProtocolManager', `启用协议失败: ${errorMsg}`);
      message.error(errorMsg);
      return false;
    }
  }, [setError, updateProtocol, addLog]);

  const disableProtocol = useCallback(async (pluginId: string): Promise<boolean> => {
    setError(null);
    try {
      await protocolApi.disableProtocol(pluginId);
      updateProtocol(pluginId, { state: 'Disabled' });
      addLog('info', 'ProtocolManager', `协议 ${pluginId} 已禁用`);
      message.success('协议已禁用');
      return true;
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('protocol:message.disableFailed'));
      setError(errorMsg);
      addLog('error', 'ProtocolManager', `禁用协议失败: ${errorMsg}`);
      message.error(errorMsg);
      return false;
    }
  }, [setError, updateProtocol, addLog]);

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
      addLog('info', 'ProtocolManager', `协议 ${pluginId} 已绑定到设备 ${deviceId}`);
      message.success('协议绑定成功');
      return true;
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('protocol:message.bindFailed'));
      setError(errorMsg);
      addLog('error', 'ProtocolManager', `绑定协议失败: ${errorMsg}`);
      message.error(errorMsg);
      return false;
    }
  }, [setError, addBinding, protocols, updateProtocol, addLog]);

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
      addLog('info', 'ProtocolManager', `协议 ${pluginId} 已从设备 ${deviceId} 解绑`);
      message.success('协议解绑成功');
      return true;
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('protocol:message.unbindFailed'));
      setError(errorMsg);
      addLog('error', 'ProtocolManager', `解绑协议失败: ${errorMsg}`);
      message.error(errorMsg);
      return false;
    }
  }, [setError, removeBinding, protocols, updateProtocol, addLog]);

  const getProtocol = useCallback(async (pluginId: string): Promise<PluginInfo | null> => {
    try {
      return await protocolApi.getProtocol(pluginId);
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('protocol:message.getInfoFailed'));
      setError(errorMsg);
      return null;
    }
  }, [setError]);

  const getBoundProtocols = useCallback(async (deviceId: string): Promise<PluginInfo[]> => {
    try {
      return await protocolApi.getBoundProtocols(deviceId);
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('protocol:message.getBindingsFailed'));
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
