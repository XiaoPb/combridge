import { useCallback } from 'react';
import { dispatchAction } from '../api/stateApi';
import type { Action, ActionResult } from '../types/state';
import i18n from '../i18n';
import { formatErrorMessage } from '../utils/errorMessage';

export function useAppDispatch() {
  const dispatch = useCallback(async (action: Action): Promise<ActionResult> => {
    try {
      const result = await dispatchAction(action);
      return result;
    } catch (error) {
      console.error('Dispatch action failed:', error);
      return {
        success: false,
        message: formatErrorMessage(error, i18n.t('common:message.dispatchFailed')),
      };
    }
  }, []);

  return dispatch;
}

export function useDeviceActions() {
  const dispatch = useAppDispatch();

  const addSerialDevice = useCallback(
    async (id: string, name: string, baudRate: number = 115200) => {
      return dispatch({
        type: 'DEVICE_ADD_SERIAL',
        id,
        name,
        baudRate,
      });
    },
    [dispatch]
  );

  const addBleDevice = useCallback(
    async (id: string, name: string, mac: string) => {
      return dispatch({
        type: 'DEVICE_ADD_BLE',
        id,
        name,
        mac,
      });
    },
    [dispatch]
  );

  const removeDevice = useCallback(
    async (deviceId: string) => {
      return dispatch({
        type: 'DEVICE_REMOVE',
        deviceId,
      });
    },
    [dispatch]
  );

  const connectDevice = useCallback(
    async (deviceId: string) => {
      return dispatch({
        type: 'DEVICE_CONNECT',
        deviceId,
      });
    },
    [dispatch]
  );

  const disconnectDevice = useCallback(
    async (deviceId: string) => {
      return dispatch({
        type: 'DEVICE_DISCONNECT',
        deviceId,
      });
    },
    [dispatch]
  );

  const updateDeviceConfig = useCallback(
    async (deviceId: string, config: Record<string, unknown>) => {
      return dispatch({
        type: 'DEVICE_UPDATE_CONFIG',
        deviceId,
        config,
      });
    },
    [dispatch]
  );

  const switchDevice = useCallback(
    async (deviceId: string) => {
      return dispatch({
        type: 'DEVICE_SWITCH',
        deviceId,
      });
    },
    [dispatch]
  );

  return {
    addSerialDevice,
    addBleDevice,
    removeDevice,
    connectDevice,
    disconnectDevice,
    updateDeviceConfig,
    switchDevice,
  };
}

export function useChannelActions() {
  const dispatch = useAppDispatch();

  const addChannel = useCallback(
    async (deviceId: string, channelId: string, direction: 'read' | 'write' | 'notify') => {
      return dispatch({
        type: 'CHANNEL_ADD',
        deviceId,
        channelId,
        direction,
      });
    },
    [dispatch]
  );

  const subscribeChannel = useCallback(
    async (deviceId: string, channelId: string, subscribe: boolean) => {
      return dispatch({
        type: 'CHANNEL_SUBSCRIBE',
        deviceId,
        channelId,
        subscribe,
      });
    },
    [dispatch]
  );

  const sendData = useCallback(
    async (deviceId: string, channelId: string, data: number[]) => {
      return dispatch({
        type: 'DATA_SEND',
        deviceId,
        channelId,
        data,
      });
    },
    [dispatch]
  );

  const clearBuffer = useCallback(
    async (deviceId: string, channelId: string) => {
      return dispatch({
        type: 'BUFFER_CLEAR',
        deviceId,
        channelId,
      });
    },
    [dispatch]
  );

  return {
    addChannel,
    subscribeChannel,
    sendData,
    clearBuffer,
  };
}

export function useTabActions() {
  const dispatch = useAppDispatch();

  const addTab = useCallback(
    async (deviceId: string, channelId: string | undefined, label: string) => {
      return dispatch({
        type: 'TAB_ADD',
        deviceId,
        channelId,
        label,
      });
    },
    [dispatch]
  );

  const removeTab = useCallback(
    async (tabKey: string) => {
      return dispatch({
        type: 'TAB_REMOVE',
        tabKey,
      });
    },
    [dispatch]
  );

  const switchTab = useCallback(
    async (tabKey: string) => {
      return dispatch({
        type: 'TAB_SWITCH',
        tabKey,
      });
    },
    [dispatch]
  );

  return {
    addTab,
    removeTab,
    switchTab,
  };
}
