import { invoke } from './tauri';
import type { 
  Gh3036ChannelConfig, 
  Gh3036CsvConfig, 
  Gh3036FrameData, 
  Gh3036RpcCommand,
  Gh3036VersionTypeConfig,
  FactoryTestStep,
  FactoryTestStatus,
  FactoryTestResult,
  ConfigValidationResult,
  FactoryThresholdConfig,
  FactoryEvaluationResult,
  ThresholdConfigValidation
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

  async getVersionTypes(): Promise<Gh3036VersionTypeConfig[]> {
    return invoke<Gh3036VersionTypeConfig[]>('gh3036_get_version_types');
  },

  async executeRpc(commandKey: string, params: string[]): Promise<number[]> {
    return invoke<number[]>('gh3036_execute_rpc', { commandKey, params });
  },

  async subscribeEvents(): Promise<void> {
    await invoke<void>('gh3036_subscribe_events');
  },

  async getLibraryStatus(): Promise<LibraryStatus> {
    return invoke<LibraryStatus>('gh3036_get_library_status');
  },

  async loadConfigFile(filePath: string): Promise<string[]> {
    return invoke<string[]>('gh3036_load_config_file', { filePath });
  },
};

export const factoryTestApi = {
  async start(): Promise<void> {
    const ts = () => new Date().toISOString().substr(11, 12);
    console.log(`[${ts()}] [factoryTestApi] 调用 gh3036_factory_test_start`);
    try {
      await invoke<void>('gh3036_factory_test_start');
      console.log(`[${ts()}] [factoryTestApi] gh3036_factory_test_start 成功`);
    } catch (err) {
      console.error(`[${ts()}] [factoryTestApi] gh3036_factory_test_start 失败:`, err);
      throw err;
    }
  },

  async stop(): Promise<void> {
    await invoke<void>('gh3036_factory_test_stop');
  },

  async getStatus(): Promise<{ status: FactoryTestStatus; currentStep: FactoryTestStep }> {
    return invoke<{ status: FactoryTestStatus; currentStep: FactoryTestStep }>('gh3036_factory_test_status');
  },

  async continue(): Promise<void> {
    await invoke<void>('gh3036_factory_test_continue');
  },

  async setConfigDir(configDir: string): Promise<void> {
    await invoke<void>('gh3036_factory_test_set_config_dir', { configDir });
  },

  async validateConfig(): Promise<ConfigValidationResult> {
    return invoke<ConfigValidationResult>('gh3036_factory_test_validate_config');
  },

  async getResult(): Promise<FactoryTestResult | null> {
    return invoke<FactoryTestResult | null>('gh3036_factory_test_get_result');
  },

  async validateThresholdConfig(): Promise<ThresholdConfigValidation> {
    return invoke<ThresholdConfigValidation>('gh3036_validate_threshold_config');
  },

  async getThresholdConfig(): Promise<FactoryThresholdConfig | null> {
    return invoke<FactoryThresholdConfig | null>('gh3036_get_threshold_config');
  },

  async getEvaluationResult(): Promise<FactoryEvaluationResult | null> {
    return invoke<FactoryEvaluationResult | null>('gh3036_get_evaluation_result');
  },
};

export type { Gh3036FrameData };
