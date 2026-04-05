import { invoke } from './tauri';

export interface LibraryStatus {
  isLinked: boolean;
  isInitialized: boolean;
}

export const gh3036Api = {
  async executeRpc(commandKey: string, params: string[]): Promise<void> {
    await invoke<void>('gh3036_execute_rpc', { commandKey, params });
  },

  async subscribeEvents(): Promise<void> {
    await invoke<void>('gh3036_subscribe_events');
  },

  async getLibraryStatus(): Promise<LibraryStatus> {
    return invoke<LibraryStatus>('gh3036_get_library_status');
  },

  async onRxData(deviceId: string, data: number[]): Promise<void> {
    await invoke<void>('gh3036_on_rx_data', { deviceId, data });
  },
};
