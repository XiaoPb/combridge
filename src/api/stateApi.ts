import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Action, ActionResult, AppState } from '../types/state';

const STATE_CHANGE_EVENT = 'state-change';

export async function dispatchAction(action: Action): Promise<ActionResult> {
  const actionJson = transformAction(action);
  return invoke<ActionResult>('dispatch_action', { action: actionJson });
}

export async function getState(): Promise<AppState> {
  return invoke<AppState>('get_state');
}

export async function getChannelData(
  channelId: string,
  direction?: string,
  limit?: number
): Promise<Record<string, unknown>> {
  return invoke('get_channel_data', { channelId, direction, limit });
}

export async function restoreState(): Promise<void> {
  return invoke('restore_state');
}

export async function saveState(): Promise<void> {
  return invoke('save_state');
}

export async function getConnectedChannels(): Promise<AppState['channels']> {
  return invoke('get_connected_channels');
}

export async function getWindowState(): Promise<AppState['windowState']> {
  return invoke('get_window_state');
}

export function subscribeToStateChanges(
  callback: (state: AppState) => void
): () => void {
  let unlisten: (() => void) | null = null;
  
  listen<AppState>(STATE_CHANGE_EVENT, (event) => {
    callback(event.payload);
  }).then((fn) => {
    unlisten = fn;
  });
  
  return () => {
    if (unlisten) {
      unlisten();
    }
  };
}

function transformAction(action: Action): Record<string, unknown> {
  switch (action.type) {
    case 'CHANNEL_ADD':
      return {
        type: 'CHANNEL_ADD',
        name: action.name,
        channelType: action.channelType,
        config: action.config,
      };
    case 'CHANNEL_REMOVE':
      return {
        type: 'CHANNEL_REMOVE',
        id: action.id,
      };
    case 'CHANNEL_CONNECT':
      return {
        type: 'CHANNEL_CONNECT',
        id: action.id,
        config: action.config,
      };
    case 'CHANNEL_DISCONNECT':
      return {
        type: 'CHANNEL_DISCONNECT',
        id: action.id,
      };
    case 'DATA_SEND':
      return {
        type: 'DATA_SEND',
        channelId: action.channelId,
        data: action.data,
      };
    case 'CHANNEL_SWITCH':
      return {
        type: 'CHANNEL_SWITCH',
        channelId: action.channelId,
      };
    case 'BUFFER_CLEAR':
      return {
        type: 'BUFFER_CLEAR',
        channelId: action.channelId,
        direction: action.direction,
      };
    case 'TAB_ADD':
      return {
        type: 'TAB_ADD',
        channelId: action.channelId,
        label: action.label,
      };
    case 'TAB_REMOVE':
      return {
        type: 'TAB_REMOVE',
        tabKey: action.tabKey,
      };
    case 'TAB_SWITCH':
      return {
        type: 'TAB_SWITCH',
        tabKey: action.tabKey,
      };
    case 'SETTINGS_UPDATE':
      return {
        type: 'SETTINGS_UPDATE',
        settings: action.settings,
      };
    case 'STATE_RESTORE':
      return {
        type: 'STATE_RESTORE',
        windowState: action.windowState,
      };
    default:
      return action as Record<string, unknown>;
  }
}
