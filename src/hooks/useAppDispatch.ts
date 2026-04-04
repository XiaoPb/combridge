import { useCallback } from 'react';
import { dispatchAction } from '../api/stateApi';
import type { Action, ActionResult } from '../types/state';

export function useAppDispatch() {
  const dispatch = useCallback(async (action: Action): Promise<ActionResult> => {
    try {
      const result = await dispatchAction(action);
      return result;
    } catch (error) {
      console.error('Dispatch action failed:', error);
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }, []);

  return dispatch;
}

export function useChannelActions() {
  const dispatch = useAppDispatch();

  const addChannel = useCallback(
    async (name: string, channelType: 'serial' | 'ble', config?: Record<string, unknown>) => {
      return dispatch({
        type: 'CHANNEL_ADD',
        name,
        channelType,
        config,
      });
    },
    [dispatch]
  );

  const removeChannel = useCallback(
    async (id: string) => {
      return dispatch({
        type: 'CHANNEL_REMOVE',
        id,
      });
    },
    [dispatch]
  );

  const connectChannel = useCallback(
    async (id: string, config?: Record<string, unknown>) => {
      return dispatch({
        type: 'CHANNEL_CONNECT',
        id,
        config,
      });
    },
    [dispatch]
  );

  const disconnectChannel = useCallback(
    async (id: string) => {
      return dispatch({
        type: 'CHANNEL_DISCONNECT',
        id,
      });
    },
    [dispatch]
  );

  const switchChannel = useCallback(
    async (channelId: string) => {
      return dispatch({
        type: 'CHANNEL_SWITCH',
        channelId,
      });
    },
    [dispatch]
  );

  const sendData = useCallback(
    async (channelId: string, data: number[]) => {
      return dispatch({
        type: 'DATA_SEND',
        channelId,
        data,
      });
    },
    [dispatch]
  );

  const clearBuffer = useCallback(
    async (channelId: string, direction: 'tx' | 'rx' | 'all') => {
      return dispatch({
        type: 'BUFFER_CLEAR',
        channelId,
        direction,
      });
    },
    [dispatch]
  );

  return {
    addChannel,
    removeChannel,
    connectChannel,
    disconnectChannel,
    switchChannel,
    sendData,
    clearBuffer,
  };
}

export function useTabActions() {
  const dispatch = useAppDispatch();

  const addTab = useCallback(
    async (channelId: string, label: string) => {
      return dispatch({
        type: 'TAB_ADD',
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
