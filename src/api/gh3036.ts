import { invoke } from './tauri';
import type { 
  Gh3036ChannelConfig, 
  Gh3036CsvConfig, 
  Gh3036FrameData, 
  Gh3036RpcCommand 
} from './types';

export interface LibraryStatus {
  isLinked: boolean;
  isInitialized: boolean;
}

export const gh3036Api = {
  async init(): Promise<void> {
    await invoke<void>('gh3036_init');
  },

  async isInitialized(): Promise<boolean> {
    return invoke<boolean>('gh3036_is_initialized');
  },

  async configureTxChannel(
    channelType: 'serial' | 'ble',
    deviceId: string,
    characteristicUuid?: string
  ): Promise<void> {
    await invoke<void>('gh3036_configure_tx_channel', {
      channelType,
      deviceId,
      characteristicUuid,
    });
  },

  async configureRxChannel(
    channelType: 'serial' | 'ble',
    deviceId: string,
    characteristicUuid?: string
  ): Promise<void> {
    await invoke<void>('gh3036_configure_rx_channel', {
      channelType,
      deviceId,
      characteristicUuid,
    });
  },

  async getChannels(): Promise<{
    tx: Gh3036ChannelConfig | null;
    rx: Gh3036ChannelConfig | null;
  }> {
    const [tx, rx] = await invoke<[Gh3036ChannelConfig | null, Gh3036ChannelConfig | null]>(
      'gh3036_get_channels'
    );
    return { tx, rx };
  },

  async sendData(data: number[]): Promise<void> {
    await invoke<void>('gh3036_send_data', { data });
  },

  async setCsvConfig(enabled: boolean, outputDir: string): Promise<void> {
    await invoke<void>('gh3036_set_csv_config', { enabled, outputDir });
  },

  async getCsvConfig(): Promise<Gh3036CsvConfig> {
    return invoke<Gh3036CsvConfig>('gh3036_get_csv_config');
  },

  async getRpcCommands(): Promise<Gh3036RpcCommand[]> {
    return invoke<Gh3036RpcCommand[]>('gh3036_get_rpc_commands');
  },

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

export type { Gh3036FrameData };
