import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { 
  Gh3036ChannelConfig, 
  Gh3036CsvConfig, 
  Gh3036RpcCommand,
  FactoryTestStep, 
  FactoryTestStatus, 
  FactoryTestStepResult,
  FactoryTestResult,
  ConfigValidationResult,
  FactoryTestProgressEvent 
} from '../api/types';
import { gh3036Api, factoryTestApi } from '../api/gh3036';
import { preferencesApi, type Gh3036ChannelPreferences } from '../api/tauri';
import { decodePayload, type EventBusEvent } from '../utils/msgpack';
import type { Gh3036FramePayload, Gh3036FramesPayload } from '../api/events';

export type Gh3036EventData = {
  event_type: string;
  timestamp: number;
  data: Record<string, unknown>;
};

export type Gh3036FrameEventData = Gh3036FramePayload;

export type Gh3036ChannelConfigState = {
  connectionType: string;
  serialPort: string;
  bleDevice: string;
  txChar: string;
  rxChar: string;
};

interface Gh3036State {
  isInitialized: boolean;
  isLoading: boolean;
  error: string | null;
  
  channelConfig: Gh3036ChannelConfigState;
  
  txChannel: Gh3036ChannelConfig | null;
  rxChannel: Gh3036ChannelConfig | null;
  
  csvConfig: Gh3036CsvConfig;
  
  rpcCommands: Gh3036RpcCommand[];
  expandedCommand: string | null;
  
  frameData: Gh3036FramePayload[];
  maxFrameCount: number;
  
  eventData: Gh3036EventData[];
  maxEventCount: number;
  
  framesData: Map<number, Gh3036FramesPayload>;
  maxFramesCount: number;
  
  vitalSigns: {
    hr: number | null;
    spo2: number | null;
    adt: string | null;
    gnadt: string | null;
  };
  
  gsensorData: {
    acc_x: number[];
    acc_y: number[];
    acc_z: number[];
    gyro_x: number[];
    gyro_y: number[];
    gyro_z: number[];
  };
  maxGsensorCount: number;
  
  chartGroups: ChartGroupConfig[];
  selectedFunctionId: number | null;
  
  isLinked: boolean;
  
  eventListeners: {
    event?: UnlistenFn;
    frame?: UnlistenFn;
    deviceDisconnected?: UnlistenFn;
    factoryTest?: UnlistenFn;
  };
  
  rpcConfig: {
    workMode: string;
    command: string;
    writeRegAddr: string;
    writeRegValue: string;
    readRegAddr: string;
    readRegValue: string;
    configPath: string;
    selectedFunctions: string[];
    isRunning: boolean;
    factoryMode: string;
    factoryResult: string;
    version: string;
  };
  
  factoryTest: {
    status: FactoryTestStatus;
    currentStep: FactoryTestStep;
    progress: number;
    message: string;
    configDir: string;
    configValidation: ConfigValidationResult | null;
    stepResults: FactoryTestStepResult[];
    result: FactoryTestResult | null;
    isRunning: boolean;
  };
  
  setIsInitialized: (value: boolean) => void;
  setIsLoading: (value: boolean) => void;
  setError: (error: string | null) => void;
  
  setTxChannel: (config: Gh3036ChannelConfig | null) => void;
  setRxChannel: (config: Gh3036ChannelConfig | null) => void;
  
  setCsvConfig: (config: Gh3036CsvConfig) => void;
  
  setRpcCommands: (commands: Gh3036RpcCommand[]) => void;
  setExpandedCommand: (key: string | null) => void;
  
  addFrameData: (frame: Gh3036FramePayload) => void;
  clearFrameData: () => void;
  
  addEventData: (event: Gh3036EventData) => void;
  clearEventData: () => void;
  
  addFramesData: (frames: Gh3036FramesPayload) => void;
  clearWaveformData: () => void;
  
  setChartGroups: (groups: ChartGroupConfig[]) => void;
  setSelectedFunctionId: (id: number | null) => void;
  
  setIsLinked: (value: boolean) => void;
  
  setRpcConfig: (config: Partial<Gh3036State['rpcConfig']>) => void;
  
  initialize: () => Promise<void>;
  loadChannels: () => Promise<void>;
  loadCsvConfig: () => Promise<void>;
  loadRpcCommands: () => Promise<void>;
  
  loadChannelConfig: () => Promise<void>;
  updateChannelConfig: (config: Partial<Gh3036ChannelConfigState>) => Promise<void>;
  
  configureTxChannel: (channelType: 'serial' | 'ble', deviceId: string, characteristicUuid?: string) => Promise<boolean>;
  configureRxChannel: (channelType: 'serial' | 'ble', deviceId: string, characteristicUuid?: string) => Promise<boolean>;
  
  updateCsvConfig: (enabled: boolean, outputDir: string) => Promise<boolean>;
  
  sendData: (data: number[]) => Promise<boolean>;
  
  executeRpc: (commandKey: string, params: string[]) => Promise<number[]>;
  subscribeEvents: () => Promise<void>;
  unsubscribeEvents: () => void;
  loadLibraryStatus: () => Promise<void>;
  
  setFactoryTestStatus: (status: FactoryTestStatus) => void;
  setFactoryTestStep: (step: FactoryTestStep) => void;
  setFactoryTestProgress: (progress: number, message: string) => void;
  setFactoryTestConfigDir: (configDir: string) => void;
  setFactoryTestConfigValidation: (validation: ConfigValidationResult | null) => void;
  addFactoryTestStepResult: (result: FactoryTestStepResult) => void;
  setFactoryTestResult: (result: FactoryTestResult | null) => void;
  resetFactoryTest: () => void;
  startFactoryTest: () => Promise<void>;
  stopFactoryTest: () => Promise<void>;
  continueFactoryTest: () => Promise<void>;
  setFactoryTestConfigDirAsync: (configDir: string) => Promise<void>;
  validateFactoryTestConfig: () => Promise<void>;
  subscribeFactoryTestEvents: () => Promise<void>;
  unsubscribeFactoryTestEvents: () => void;
}

interface ChartGroupConfig {
  name: string;
  columns: string[];
  height?: number;
}

export const useGh3036Store = create<Gh3036State>((set, get) => ({
  isInitialized: false,
  isLoading: false,
  error: null,
  
  channelConfig: {
    connectionType: 'serial',
    serialPort: '',
    bleDevice: '',
    txChar: '00000004-0000-1000-8000-00805f9b34fb',
    rxChar: '00000003-0000-1000-8000-00805f9b34fb',
  },
  
  txChannel: null,
  rxChannel: null,
  
  csvConfig: {
    enabled: false,
    output_dir: '.',
  },
  
  rpcCommands: [],
  expandedCommand: null,
  
  frameData: [],
  maxFrameCount: 1000,
  
  eventData: [],
  maxEventCount: 500,
  
  framesData: new Map(),
  maxFramesCount: 100,
  
  vitalSigns: {
    hr: null,
    spo2: null,
    adt: null,
    gnadt: null,
  },
  
  gsensorData: {
    acc_x: [],
    acc_y: [],
    acc_z: [],
    gyro_x: [],
    gyro_y: [],
    gyro_z: [],
  },
  maxGsensorCount: 500,
  
  chartGroups: [],
  selectedFunctionId: null,
  
  isLinked: false,
  
  eventListeners: {},
  
  rpcConfig: {
    workMode: '0',
    command: 'idle',
    writeRegAddr: '0000',
    writeRegValue: '0000',
    readRegAddr: '0000',
    readRegValue: '0000',
    configPath: '',
    selectedFunctions: ['adt', 'hr', 'hrv', 'hsm', 'fpbp'],
    isRunning: false,
    factoryMode: '',
    factoryResult: '-',
    version: '-',
  },
  
  factoryTest: {
    status: 'idle',
    currentStep: 'idle',
    progress: 0,
    message: '',
    configDir: '',
    configValidation: null,
    stepResults: [],
    result: null,
    isRunning: false,
  },
  
  setIsInitialized: (value) => set({ isInitialized: value }),
  setIsLoading: (value) => set({ isLoading: value }),
  setError: (error) => set({ error }),
  
  setTxChannel: (config) => set({ txChannel: config }),
  setRxChannel: (config) => set({ rxChannel: config }),
  
  setCsvConfig: (config) => set({ csvConfig: config }),
  
  setRpcCommands: (commands) => set({ rpcCommands: commands }),
  setExpandedCommand: (key) => set({ expandedCommand: key }),
  
  addFrameData: (frame) => {
    const { frameData, maxFrameCount } = get();
    const newData = [...frameData, frame];
    if (newData.length > maxFrameCount) {
      newData.splice(0, newData.length - maxFrameCount);
    }
    set({ frameData: newData });
  },
  
  clearFrameData: () => set({ frameData: [] }),
  
  addEventData: (event) => {
    const { eventData, maxEventCount } = get();
    const newData = [...eventData, event];
    if (newData.length > maxEventCount) {
      newData.splice(0, newData.length - maxEventCount);
    }
    set({ eventData: newData });
  },
  
  clearEventData: () => set({ eventData: [] }),
  
  addFramesData: (frames) => {
    const { framesData, maxFramesCount, gsensorData, maxGsensorCount, vitalSigns } = get();
    const newFramesData = new Map(framesData);
    
    const existing = newFramesData.get(frames.function_id);
    if (existing) {
      const combined: Gh3036FramesPayload = {
        function_id: frames.function_id,
        function_name: frames.function_name,
        frame_count: existing.frame_count + frames.frame_count,
        channel_count: frames.channel_count,
        frame_cnts: [...existing.frame_cnts, ...frames.frame_cnts].slice(-maxFramesCount * 10),
        timestamps: [...existing.timestamps, ...frames.timestamps].slice(-maxFramesCount * 10),
        frame_ids: [...existing.frame_ids, ...frames.frame_ids].slice(-maxFramesCount * 10),
        ipd_pa: existing.ipd_pa.map((ch, i) => [...ch, ...(frames.ipd_pa[i] || [])].slice(-maxFramesCount * 10)),
        rawdata: existing.rawdata.map((rd, i) => [...rd, ...(frames.rawdata[i] || [])].slice(-maxFramesCount * 10)),
        flags: existing.flags.map((f, i) => [...f, ...(frames.flags[i] || [])].slice(-maxFramesCount * 10)),
        agc_info: existing.agc_info.map((a, i) => [...a, ...(frames.agc_info[i] || [])].slice(-maxFramesCount * 10)),
        acc_x: [...existing.acc_x, ...frames.acc_x].slice(-maxFramesCount * 10),
        acc_y: [...existing.acc_y, ...frames.acc_y].slice(-maxFramesCount * 10),
        acc_z: [...existing.acc_z, ...frames.acc_z].slice(-maxFramesCount * 10),
        gyro_x: [...existing.gyro_x, ...frames.gyro_x].slice(-maxFramesCount * 10),
        gyro_y: [...existing.gyro_y, ...frames.gyro_y].slice(-maxFramesCount * 10),
        gyro_z: [...existing.gyro_z, ...frames.gyro_z].slice(-maxFramesCount * 10),
        algo_results: [...existing.algo_results, ...frames.algo_results].slice(-maxFramesCount * 10),
        led_drv_fs: [...existing.led_drv_fs, ...frames.led_drv_fs].slice(-maxFramesCount * 10),
      };
      newFramesData.set(frames.function_id, combined);
    } else {
      newFramesData.set(frames.function_id, frames);
    }
    
    const newGsensorData = {
      acc_x: [...gsensorData.acc_x, ...frames.acc_x].slice(-maxGsensorCount),
      acc_y: [...gsensorData.acc_y, ...frames.acc_y].slice(-maxGsensorCount),
      acc_z: [...gsensorData.acc_z, ...frames.acc_z].slice(-maxGsensorCount),
      gyro_x: [...gsensorData.gyro_x, ...frames.gyro_x].slice(-maxGsensorCount),
      gyro_y: [...gsensorData.gyro_y, ...frames.gyro_y].slice(-maxGsensorCount),
      gyro_z: [...gsensorData.gyro_z, ...frames.gyro_z].slice(-maxGsensorCount),
    };
    
    let newVitalSigns = { ...vitalSigns };
    if (frames.algo_results.length > 0 && frames.algo_results[0].length > 0) {
      const algoValue = frames.algo_results[frames.algo_results.length - 1][0];
      switch (frames.function_id) {
        case 1:
          newVitalSigns = { ...newVitalSigns, hr: algoValue };
          break;
        case 2:
          newVitalSigns = { ...newVitalSigns, spo2: algoValue };
          break;
        case 0:
          newVitalSigns = { ...newVitalSigns, adt: algoValue === 1 ? '佩戴' : '未佩戴' };
          break;
        case 4:
          newVitalSigns = { ...newVitalSigns, gnadt: algoValue === 1 ? '活体' : '非活体' };
          break;
      }
    }
    
    set({ 
      framesData: newFramesData,
      gsensorData: newGsensorData,
      vitalSigns: newVitalSigns,
      selectedFunctionId: get().selectedFunctionId ?? frames.function_id 
    });
  },
  
  clearWaveformData: () => set({ 
    framesData: new Map(), 
    chartGroups: [], 
    selectedFunctionId: null,
    gsensorData: {
      acc_x: [],
      acc_y: [],
      acc_z: [],
      gyro_x: [],
      gyro_y: [],
      gyro_z: [],
    },
    vitalSigns: {
      hr: null,
      spo2: null,
      adt: null,
      gnadt: null,
    },
  }),
  
  setChartGroups: (groups) => set({ chartGroups: groups }),
  setSelectedFunctionId: (id) => set({ selectedFunctionId: id }),
  
  setIsLinked: (value) => set({ isLinked: value }),
  
  setRpcConfig: (config) => set((state) => ({
    rpcConfig: { ...state.rpcConfig, ...config },
  })),
  
  initialize: async () => {
    const { isInitialized, isLoading } = get();
    if (isInitialized || isLoading) {
      console.log('[Gh3036Store] 已初始化或正在初始化，跳过');
      return;
    }
    
    set({ isLoading: true, error: null });
    try {
      await gh3036Api.init();
      set({ isInitialized: true });
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '初始化失败';
      set({ error: errorMsg });
    } finally {
      set({ isLoading: false });
    }
  },
  
  loadChannels: async () => {
    try {
      const { tx, rx } = await gh3036Api.getChannels();
      set({ txChannel: tx, rxChannel: rx });
    } catch (err) {
      console.error('加载通道配置失败:', err);
    }
  },
  
  loadCsvConfig: async () => {
    try {
      const config = await gh3036Api.getCsvConfig();
      set({ csvConfig: config });
    } catch (err) {
      console.error('加载CSV配置失败:', err);
    }
  },
  
  loadRpcCommands: async () => {
    try {
      const commands = await gh3036Api.getRpcCommands();
      set({ rpcCommands: commands });
    } catch (err) {
      console.error('加载RPC命令失败:', err);
    }
  },
  
  loadChannelConfig: async () => {
    try {
      const prefs = await preferencesApi.get();
      const gh3036Channel = prefs.gh3036_channel;
      if (gh3036Channel) {
        set({
          channelConfig: {
            connectionType: gh3036Channel.connection_type || 'serial',
            serialPort: gh3036Channel.serial_port || '',
            bleDevice: gh3036Channel.ble_device || '',
            txChar: gh3036Channel.tx_char || '00000004-0000-1000-8000-00805f9b34fb',
            rxChar: gh3036Channel.rx_char || '00000003-0000-1000-8000-00805f9b34fb',
          },
        });
      }
    } catch (err) {
      console.error('加载通道配置失败:', err);
    }
  },
  
  updateChannelConfig: async (config) => {
    try {
      const currentConfig = get().channelConfig;
      const newConfig = { ...currentConfig, ...config };
      
      const prefs: Gh3036ChannelPreferences = {
        connection_type: newConfig.connectionType,
        serial_port: newConfig.serialPort,
        ble_device: newConfig.bleDevice,
        tx_char: newConfig.txChar,
        rx_char: newConfig.rxChar,
      };
      
      await preferencesApi.updateGh3036Channel(prefs);
      set({ channelConfig: newConfig });
    } catch (err) {
      console.error('保存通道配置失败:', err);
    }
  },
  
  configureTxChannel: async (channelType, deviceId, characteristicUuid) => {
    set({ isLoading: true, error: null });
    try {
      await gh3036Api.configureTxChannel(channelType, deviceId, characteristicUuid);
      set({ 
        txChannel: {
          channel_type: channelType === 'serial' ? 'Serial' : 'Ble',
          device_id: deviceId,
          characteristic_uuid: characteristicUuid || null,
        }
      });
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '配置发送通道失败';
      set({ error: errorMsg });
      return false;
    } finally {
      set({ isLoading: false });
    }
  },
  
  configureRxChannel: async (channelType, deviceId, characteristicUuid) => {
    set({ isLoading: true, error: null });
    try {
      await gh3036Api.configureRxChannel(channelType, deviceId, characteristicUuid);
      set({ 
        rxChannel: {
          channel_type: channelType === 'serial' ? 'Serial' : 'Ble',
          device_id: deviceId,
          characteristic_uuid: characteristicUuid || null,
        }
      });
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '配置接收通道失败';
      set({ error: errorMsg });
      return false;
    } finally {
      set({ isLoading: false });
    }
  },
  
  updateCsvConfig: async (enabled, outputDir) => {
    set({ isLoading: true, error: null });
    try {
      await gh3036Api.setCsvConfig(enabled, outputDir);
      set({ 
        csvConfig: { enabled, output_dir: outputDir }
      });
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '更新CSV配置失败';
      set({ error: errorMsg });
      return false;
    } finally {
      set({ isLoading: false });
    }
  },
  
  sendData: async (data) => {
    set({ error: null });
    try {
      await gh3036Api.sendData(data);
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '发送数据失败';
      set({ error: errorMsg });
      return false;
    }
  },
  
  executeRpc: async (commandKey, params) => {
    set({ isLoading: true, error: null });
    try {
      const result = await gh3036Api.executeRpc(commandKey, params);
      return result;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '执行RPC指令失败';
      set({ error: errorMsg });
      return [];
    } finally {
      set({ isLoading: false });
    }
  },
  
  subscribeEvents: async () => {
    const { eventListeners } = get();
    
    if (eventListeners.event || eventListeners.frame || eventListeners.deviceDisconnected) {
      console.debug('[Gh3036Store] 事件已订阅，跳过重复订阅');
      return;
    }
    
    try {
      const eventUnlisten = await listen<EventBusEvent>('event-bus', (event) => {
        if (event.payload.topic === 'gh3036:event') {
          try {
            const parsed = decodePayload<Gh3036EventData>(event.payload);
            get().addEventData(parsed);
          } catch (err) {
            console.error('[Gh3036Store] Failed to decode gh3036:event payload:', err);
          }
        }
      });
      
      const frameUnlisten = await listen<EventBusEvent>('event-bus', (event) => {
        if (event.payload.topic === 'gh3036:frame') {
          try {
            const parsed = decodePayload<Gh3036FrameEventData>(event.payload);
            get().addFrameData(parsed);
          } catch (err) {
            console.error('[Gh3036Store] Failed to decode gh3036:frame payload:', err);
          }
        }
      });
      
      const deviceDisconnectedUnlisten = await listen<EventBusEvent>('event-bus', (event) => {
        const { topic } = event.payload;
        
        if (topic === 'serial:disconnected' || topic === 'ble:disconnected') {
          try {
            type DeviceDisconnectedPayload = { port_name?: string; address?: string };
            const parsed = decodePayload<DeviceDisconnectedPayload>(event.payload);
            const deviceId = parsed.port_name || parsed.address;
            
            if (deviceId) {
              const { txChannel, rxChannel } = get();
              
              if (txChannel?.device_id === deviceId) {
                set({ txChannel: null });
                console.log('[Gh3036Store] TX 通道已清理: 设备', deviceId, '已断开');
              }
              
              if (rxChannel?.device_id === deviceId) {
                set({ rxChannel: null });
                console.log('[Gh3036Store] RX 通道已清理: 设备', deviceId, '已断开');
              }
            }
          } catch (err) {
            console.error('[Gh3036Store] Failed to decode device disconnected payload:', err);
          }
        }
      });
      
      set({ 
        eventListeners: { 
          event: eventUnlisten, 
          frame: frameUnlisten,
          deviceDisconnected: deviceDisconnectedUnlisten,
        } 
      });
    } catch (err) {
      console.error('订阅事件失败:', err);
    }
  },
  
  unsubscribeEvents: () => {
    const { eventListeners } = get();
    
    if (eventListeners.event) {
      eventListeners.event();
    }
    if (eventListeners.frame) {
      eventListeners.frame();
    }
    if (eventListeners.deviceDisconnected) {
      eventListeners.deviceDisconnected();
    }
    if (eventListeners.factoryTest) {
      eventListeners.factoryTest();
    }
    
    set({ eventListeners: {} });
  },
  
  loadLibraryStatus: async () => {
    try {
      const status = await gh3036Api.getLibraryStatus();
      set({ 
        isLinked: status.isLinked, 
        isInitialized: status.isInitialized 
      });
    } catch (err) {
      console.error('加载库状态失败:', err);
    }
  },
  
  setFactoryTestStatus: (status) => set((state) => ({
    factoryTest: { ...state.factoryTest, status },
  })),
  
  setFactoryTestStep: (step) => set((state) => ({
    factoryTest: { ...state.factoryTest, currentStep: step },
  })),
  
  setFactoryTestProgress: (progress, message) => set((state) => ({
    factoryTest: { ...state.factoryTest, progress, message },
  })),
  
  setFactoryTestConfigDir: (configDir) => set((state) => ({
    factoryTest: { ...state.factoryTest, configDir },
  })),
  
  setFactoryTestConfigValidation: (validation) => set((state) => ({
    factoryTest: { ...state.factoryTest, configValidation: validation },
  })),
  
  addFactoryTestStepResult: (result) => set((state) => ({
    factoryTest: {
      ...state.factoryTest,
      stepResults: [...state.factoryTest.stepResults, result],
    },
  })),
  
  setFactoryTestResult: (result) => set((state) => ({
    factoryTest: { ...state.factoryTest, result },
  })),
  
  resetFactoryTest: () => set((state) => ({
    factoryTest: {
      ...state.factoryTest,
      status: 'idle',
      currentStep: 'idle',
      progress: 0,
      message: '',
      stepResults: [],
      result: null,
      isRunning: false,
    },
  })),
  
  startFactoryTest: async () => {
    set((state) => ({
      factoryTest: {
        ...state.factoryTest,
        status: 'running',
        isRunning: true,
        stepResults: [],
        result: null,
        message: '正在启动产测...',
      },
    }));
    
    try {
      await factoryTestApi.start();
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '启动产测失败';
      set((state) => ({
        factoryTest: {
          ...state.factoryTest,
          status: 'failed',
          isRunning: false,
          message: errorMsg,
        },
      }));
    }
  },
  
  stopFactoryTest: async () => {
    try {
      await factoryTestApi.stop();
      set((state) => ({
        factoryTest: {
          ...state.factoryTest,
          status: 'stopped',
          isRunning: false,
          message: '产测已停止',
        },
      }));
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '停止产测失败';
      set((state) => ({
        factoryTest: {
          ...state.factoryTest,
          message: errorMsg,
        },
      }));
    }
  },
  
  continueFactoryTest: async () => {
    try {
      await factoryTestApi.continue();
      set((state) => ({
        factoryTest: {
          ...state.factoryTest,
          status: 'running',
          message: '继续产测...',
        },
      }));
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '继续产测失败';
      set((state) => ({
        factoryTest: {
          ...state.factoryTest,
          status: 'failed',
          message: errorMsg,
        },
      }));
    }
  },
  
  setFactoryTestConfigDirAsync: async (configDir) => {
    try {
      await factoryTestApi.setConfigDir(configDir);
      set((state) => ({
        factoryTest: { ...state.factoryTest, configDir },
      }));
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '设置配置目录失败';
      set((state) => ({
        factoryTest: {
          ...state.factoryTest,
          message: errorMsg,
        },
      }));
    }
  },
  
  validateFactoryTestConfig: async () => {
    try {
      const validation = await factoryTestApi.validateConfig();
      set((state) => ({
        factoryTest: { ...state.factoryTest, configValidation: validation },
      }));
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '验证配置失败';
      set((state) => ({
        factoryTest: {
          ...state.factoryTest,
          configValidation: {
            base_noise_config: null,
            ppg_noise_config: null,
            lpctr_config: null,
            lplctr_config: null,
            errors: [errorMsg],
            is_valid: false,
          },
        },
      }));
    }
  },
  
  subscribeFactoryTestEvents: async () => {
    const ts = () => new Date().toISOString().substr(11, 12);
    console.log(`[${ts()}] [Gh3036Store] subscribeFactoryTestEvents called`);
    const { eventListeners } = get();
    
    if (eventListeners.factoryTest) {
      console.log(`[${ts()}] [Gh3036Store] 产测事件已订阅，跳过重复订阅`);
      return;
    }
    
    try {
      console.log(`[${ts()}] [Gh3036Store] 开始订阅产测事件...`);
      const factoryTestUnlisten = await listen<EventBusEvent>('event-bus', (event) => {
        const recvTs = new Date().toISOString().substr(11, 12);
        const topic = event.payload.topic;
        const encoding = event.payload.encoding;
        
        if (topic === 'gh3036:factory_test_progress') {
          console.log(
            `[${recvTs}] [Gh3036Store] 收到产测进度事件, encoding=${encoding}, payload_len=${event.payload.payload?.length ?? 'N/A'}`
          );
          try {
            const progressEvent = decodePayload<FactoryTestProgressEvent>(event.payload);
            console.log(
              `[${recvTs}] [Gh3036Store] 进度事件解码成功: step=${progressEvent.current_step}, status=${progressEvent.status}, progress=${progressEvent.progress.toFixed(3)}, msg=${progressEvent.message}`
            );
            
            set((state) => ({
              factoryTest: {
                ...state.factoryTest,
                currentStep: progressEvent.current_step,
                status: progressEvent.status,
                progress: progressEvent.progress,
                message: progressEvent.message,
              },
            }));
            
            if (progressEvent.step_result) {
              get().addFactoryTestStepResult(progressEvent.step_result);
            }
            
            if (progressEvent.status === 'completed' || progressEvent.status === 'failed') {
              set((state) => ({
                factoryTest: {
                  ...state.factoryTest,
                  isRunning: false,
                },
              }));
              
              if (progressEvent.status === 'completed') {
                factoryTestApi.getResult().then((result) => {
                  if (result) {
                    console.log(`[${ts()}] [Gh3036Store] 获取产测结果成功: overall=${result.overall_result}`);
                    set((state) => ({
                      factoryTest: { ...state.factoryTest, result },
                    }));
                  }
                }).catch((err) => {
                  console.error(`[${ts()}] [Gh3036Store] 获取产测结果失败:`, err);
                });
              }
            }
          } catch (err) {
            console.error(`[${recvTs}] [Gh3036Store] 产测进度事件解码失败:`, err, 'raw payload:', event.payload);
          }
        } else if (topic.startsWith('gh3036:')) {
          console.log(`[${recvTs}] [Gh3036Store] 收到 gh3036 事件: topic=${topic}`);
        }
      });
      
      console.log(`[${ts()}] [Gh3036Store] 产测事件订阅成功, unlisten=${typeof factoryTestUnlisten}`);
      set((state) => ({
        eventListeners: {
          ...state.eventListeners,
          factoryTest: factoryTestUnlisten,
        },
      }));
    } catch (err) {
      console.error(`[${ts()}] [Gh3036Store] 订阅产测事件失败:`, err);
    }
  },
  
  unsubscribeFactoryTestEvents: () => {
    const ts = () => new Date().toISOString().substr(11, 12);
    const { eventListeners } = get();
    
    if (eventListeners.factoryTest) {
      console.log(`[${ts()}] [Gh3036Store] 取消产测事件订阅`);
      eventListeners.factoryTest();
      set((state) => ({
        eventListeners: {
          ...state.eventListeners,
          factoryTest: undefined,
        },
      }));
    }
  },
}));
