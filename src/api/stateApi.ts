import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Action, ActionResult, AppState, Device } from '../types/state';

const STATE_CHANGE_EVENT = 'state-change';

export async function dispatchAction(action: Action): Promise<ActionResult> {
  const actionJson = transformAction(action);
  return invoke<ActionResult>('dispatch_action', { action: actionJson });
}

export async function getState(): Promise<AppState> {
  return invoke<AppState>('get_state');
}

export async function getChannelData(
  deviceId: string,
  channelId: string,
  limit?: number
): Promise<Record<string, unknown>> {
  return invoke('get_channel_data', { deviceId, channelId, limit });
}

export async function restoreState(): Promise<void> {
  return invoke('restore_state');
}

export async function saveState(): Promise<void> {
  return invoke('save_state');
}

export async function getConnectedDevices(): Promise<Device[]> {
  return invoke('get_connected_devices');
}

export async function getWindowState(): Promise<AppState['windowState']> {
  return invoke('get_window_state');
}

export function subscribeToStateChanges(
  callback: (state: AppState) => void
): () => void {
  const listenPromise = listen<AppState>(STATE_CHANGE_EVENT, (event) => {
    callback(event.payload);
  });

  return () => {
    listenPromise.then((unlisten) => unlisten());
  };
}

function transformAction(action: Action): Record<string, unknown> {
  const { type, ...payload } = action as { type: string; [key: string]: unknown };
  
  const actionType = type;
  
  switch (actionType) {
    case 'DEVICE_ADD_SERIAL':
      return {
        type: 'DEVICE_ADD_SERIAL',
        id: payload.id,
        name: payload.name,
        baudRate: payload.baudRate,
      };
    case 'DEVICE_ADD_BLE':
      return {
        type: 'DEVICE_ADD_BLE',
        id: payload.id,
        name: payload.name,
        mac: payload.mac,
      };
    case 'DEVICE_REMOVE':
      return {
        type: 'DEVICE_REMOVE',
        deviceId: payload.deviceId,
      };
    case 'DEVICE_CONNECT':
      return {
        type: 'DEVICE_CONNECT',
        deviceId: payload.deviceId,
      };
    case 'DEVICE_DISCONNECT':
      return {
        type: 'DEVICE_DISCONNECT',
        deviceId: payload.deviceId,
      };
    case 'DEVICE_UPDATE_CONFIG':
      return {
        type: 'DEVICE_UPDATE_CONFIG',
        deviceId: payload.deviceId,
        config: payload.config,
      };
    case 'CHANNEL_ADD':
      return {
        type: 'CHANNEL_ADD',
        deviceId: payload.deviceId,
        channelId: payload.channelId,
        direction: payload.direction,
      };
    case 'CHANNEL_SUBSCRIBE':
      return {
        type: 'CHANNEL_SUBSCRIBE',
        deviceId: payload.deviceId,
        channelId: payload.channelId,
        subscribe: payload.subscribe,
      };
    case 'DATA_SEND':
      return {
        type: 'DATA_SEND',
        deviceId: payload.deviceId,
        channelId: payload.channelId,
        data: payload.data,
      };
    case 'DATA_RECEIVE':
      return {
        type: 'DATA_RECEIVE',
        deviceId: payload.deviceId,
        channelId: payload.channelId,
        data: payload.data,
      };
    case 'BUFFER_CLEAR':
      return {
        type: 'BUFFER_CLEAR',
        deviceId: payload.deviceId,
        channelId: payload.channelId,
      };
    case 'DEVICE_SWITCH':
      return {
        type: 'DEVICE_SWITCH',
        deviceId: payload.deviceId,
      };
    case 'TAB_ADD':
      return {
        type: 'TAB_ADD',
        deviceId: payload.deviceId,
        channelId: payload.channelId,
        label: payload.label,
      };
    case 'TAB_REMOVE':
      return {
        type: 'TAB_REMOVE',
        tabKey: payload.tabKey,
      };
    case 'TAB_SWITCH':
      return {
        type: 'TAB_SWITCH',
        tabKey: payload.tabKey,
      };
    case 'SETTINGS_UPDATE':
      return {
        type: 'SETTINGS_UPDATE',
        settings: payload.settings,
      };
    case 'STATE_RESTORE':
      return {
        type: 'STATE_RESTORE',
        windowState: payload.windowState,
      };
    default:
      return { type: actionType, ...payload };
  }
}
