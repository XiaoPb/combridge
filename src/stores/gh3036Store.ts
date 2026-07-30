import { create } from 'zustand';
import { persist } from 'zustand/middleware';
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
  FactoryTestProgressEvent,
  FactoryThresholdConfig,
  FactoryEvaluationResult,
  ThresholdConfigValidation
} from '../api/types';
import { gh3036Api, factoryTestApi } from '../api/gh3036';
import { preferencesApi, type Gh3036ChannelPreferences } from '../api/tauri';
import { decodePayload, type EventBusEvent } from '../utils/msgpack';
import type { Gh3036FramePayload, Gh3036FramesPayload, Gh3036RefDataPayload } from '../api/events';
import { getCurrentTimeString } from '../utils/helpers';
import { useConfigStore } from './configStore';
import { mergeGh3036Frames } from './gh3036FrameBuffer';
import { hasFactoryTestResult } from './factoryTestState';
import i18n from '../i18n';
import { formatErrorMessage } from '../utils/errorMessage';

const getTs = (): string => {
  const state = useConfigStore.getState();
  const timezone = state._hasHydrated ? state.settings.timezone : 'Asia/Shanghai';
  return getCurrentTimeString(timezone);
};

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
  txCharManuallyEdited: boolean;
  rxCharManuallyEdited: boolean;
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
  
  refData: {
    hrValues: number[];
    hrCount: number;
    hrValid: boolean;
    hrvValues: number[];
    hrvCount: number;
    hrvValid: boolean;
    spo2Values: number[];
    spo2Count: number;
    spo2Valid: boolean;
    timestamp: number;
  };
  
  vitalSigns: {
    hr: number | null;
    hrConfidence: number | null;
    hrSnr: number | null;
    hrRef: number[];
    spo2: number | null;
    spo2RValue: number | null;
    spo2Confidence: number | null;
    spo2ConfidenceLevel: number | null;
    spo2Ref: number[];
    hrvRri: number[];
    hrvConfidence: number | null;
    hrvRriCount: number | null;
    hrvRef: number[];
    adt: number | null;
    adtConfidence: number | null;
    gnadt: number | null;
    gnadtConfidence: number | null;
  };
  
  gsensorData: Map<number, {
    acc_x: number[];
    acc_y: number[];
    acc_z: number[];
    gyro_x: number[];
    gyro_y: number[];
    gyro_z: number[];
  }>;
  maxGsensorCount: number;
  
  chartGroups: ChartGroupConfig[];
  selectedFunctionId: number | null;
  
  chartLegendSelected: Record<string, boolean>;
  ipdRawDataType: 'ipd' | 'rawdata';
  displayDurationSeconds: number;

  sampleRateConfig: Record<number, number>;
  
  isLinked: boolean;
  
  eventListeners: {
    event?: UnlistenFn;
    frame?: UnlistenFn;
    deviceDisconnected?: UnlistenFn;
    factoryTest?: UnlistenFn;
  };
  
  _factoryTestSubscribing: boolean;
  factoryTestListenerId: number;
  
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
    versionType: number;
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
    thresholdConfig: FactoryThresholdConfig | null;
    thresholdValidation: ThresholdConfigValidation | null;
    evaluationResult: FactoryEvaluationResult | null;
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
  
  updateRefData: (refData: Gh3036RefDataPayload) => void;
  
  setChartGroups: (groups: ChartGroupConfig[]) => void;
  setSelectedFunctionId: (id: number | null) => void;
  setChartLegendSelected: (selected: Record<string, boolean>) => void;
  setIpdRawDataType: (type: 'ipd' | 'rawdata') => void;
  setDisplayDurationSeconds: (seconds: number) => void;
  setMaxFramesCount: (count: number) => void;
  setSampleRateConfig: (config: Record<number, number>) => void;
  
  setIsLinked: (value: boolean) => void;
  
  setRpcConfig: (config: Partial<Gh3036State['rpcConfig']>) => void;
  
  initialize: () => Promise<void>;
  loadChannels: () => Promise<void>;
  loadCsvConfig: () => Promise<void>;
  loadRpcCommands: () => Promise<void>;
  
  loadChannelConfig: () => Promise<void>;
  updateChannelConfig: (config: Partial<Gh3036ChannelConfigState>) => Promise<void>;
  
  setTxCharManuallyEdited: (edited: boolean) => void;
  setRxCharManuallyEdited: (edited: boolean) => void;
  
  configureTxChannel: (channelType: 'serial' | 'ble', deviceId: string, characteristicUuid?: string) => Promise<boolean>;
  configureRxChannel: (channelType: 'serial' | 'ble', deviceId: string, characteristicUuid?: string) => Promise<boolean>;
  
  updateCsvConfig: (enabled: boolean, outputDir: string) => Promise<boolean>;
  
  sendData: (data: number[]) => Promise<boolean>;
  
  executeRpc: (commandKey: string, params: string[]) => Promise<number[] | null>;
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
  subscribeFactoryTestEvents: (listenerId?: number) => Promise<void>;
  unsubscribeFactoryTestEvents: (listenerId?: number) => void;
  setThresholdConfig: (config: FactoryThresholdConfig | null) => void;
  setThresholdValidation: (validation: ThresholdConfigValidation | null) => void;
  setEvaluationResult: (result: FactoryEvaluationResult | null) => void;
  loadThresholdConfig: () => Promise<void>;
  validateThresholdConfig: () => Promise<void>;
  loadEvaluationResult: () => Promise<void>;
}

interface ChartGroupConfig {
  name: string;
  columns: string[];
  height?: number;
}

export const useGh3036Store = create<Gh3036State>()(
  persist(
    (set, get) => ({
      isInitialized: false,
      isLoading: false,
      error: null,
      
      channelConfig: {
        connectionType: 'serial',
        serialPort: '',
        bleDevice: '',
        txChar: '00000004-0000-1000-8000-00805f9b34fb',
        rxChar: '00000003-0000-1000-8000-00805f9b34fb',
        txCharManuallyEdited: false,
        rxCharManuallyEdited: false,
      },
      
      txChannel: null,
      rxChannel: null,
      
      csvConfig: {
        enabled: true,
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
  
  refData: {
    hrValues: [],
    hrCount: 0,
    hrValid: false,
    hrvValues: [],
    hrvCount: 0,
    hrvValid: false,
    spo2Values: [],
    spo2Count: 0,
    spo2Valid: false,
    timestamp: 0,
  },
  
  vitalSigns: {
    hr: null,
    hrConfidence: null,
    hrSnr: null,
    hrRef: [],
    spo2: null,
    spo2RValue: null,
    spo2Confidence: null,
    spo2ConfidenceLevel: null,
    spo2Ref: [],
    hrvRri: [],
    hrvConfidence: null,
    hrvRriCount: null,
    hrvRef: [],
    adt: null,
    adtConfidence: null,
    gnadt: null,
    gnadtConfidence: null,
  },
  
  gsensorData: new Map(),
  maxGsensorCount: 500,
  
  chartGroups: [],
  selectedFunctionId: null,
  chartLegendSelected: {},
  ipdRawDataType: 'ipd',
  displayDurationSeconds: 10,

  sampleRateConfig: {
    0: 5,
    1: 25,
    2: 25,
    3: 25,
    4: 25,
  },
  
  isLinked: false,
  
  eventListeners: {},
  
  _factoryTestSubscribing: false,
  factoryTestListenerId: 0,
  
  rpcConfig: {
    workMode: '0',
    command: 'idle',
    writeRegAddr: '0000',
    writeRegValue: '0000',
    readRegAddr: '0000',
    readRegValue: '0000',
    configPath: '',
    selectedFunctions: [],
    isRunning: false,
    factoryMode: '',
    factoryResult: '-',
    version: '-',
    versionType: 1,
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
    thresholdConfig: null,
    thresholdValidation: null,
    evaluationResult: null,
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
    newFramesData.set(
      frames.function_id,
      mergeGh3036Frames(existing, frames, maxFramesCount)
    );
    
    const newGsensorData = new Map(gsensorData);
    const existingGsensor = newGsensorData.get(frames.function_id) || {
      acc_x: [],
      acc_y: [],
      acc_z: [],
      gyro_x: [],
      gyro_y: [],
      gyro_z: [],
    };
    newGsensorData.set(frames.function_id, {
      acc_x: [...existingGsensor.acc_x, ...frames.acc_x].slice(-maxGsensorCount),
      acc_y: [...existingGsensor.acc_y, ...frames.acc_y].slice(-maxGsensorCount),
      acc_z: [...existingGsensor.acc_z, ...frames.acc_z].slice(-maxGsensorCount),
      gyro_x: [...existingGsensor.gyro_x, ...frames.gyro_x].slice(-maxGsensorCount),
      gyro_y: [...existingGsensor.gyro_y, ...frames.gyro_y].slice(-maxGsensorCount),
      gyro_z: [...existingGsensor.gyro_z, ...frames.gyro_z].slice(-maxGsensorCount),
    });
    
    let newVitalSigns = { ...vitalSigns };
    if (frames.algo_results.length > 0 && frames.algo_results[0].length > 0) {
      const lastAlgoResult = frames.algo_results[frames.algo_results.length - 1];
      switch (frames.function_id) {
        case 1:
          newVitalSigns = { 
            ...newVitalSigns, 
            hr: lastAlgoResult[0] ?? null,
            hrConfidence: lastAlgoResult[1] ?? null,
            hrSnr: lastAlgoResult[2] ?? null,
          };
          break;
        case 2:
          newVitalSigns = { 
            ...newVitalSigns, 
            spo2: lastAlgoResult[0] ?? null,
            spo2RValue: lastAlgoResult[1] ?? null,
            spo2Confidence: lastAlgoResult[2] ?? null,
            spo2ConfidenceLevel: lastAlgoResult[3] ?? null,
          };
          break;
        case 3: {
          const rriCount = lastAlgoResult[5] ?? 0;
          const rriValues = [
            lastAlgoResult[0] ?? 0,
            lastAlgoResult[1] ?? 0,
            lastAlgoResult[2] ?? 0,
            lastAlgoResult[3] ?? 0,
          ];
          const hrvRri = rriValues.map((val, idx) => idx < rriCount ? val : 0);
          newVitalSigns = { 
            ...newVitalSigns, 
            hrvRri,
            hrvConfidence: lastAlgoResult[4] ?? null,
            hrvRriCount: rriCount,
          };
          break;
        }
        case 0:
          newVitalSigns = { 
            ...newVitalSigns, 
            adt: lastAlgoResult[0] ?? null,
            adtConfidence: lastAlgoResult[1] ?? null,
          };
          break;
        case 4:
          newVitalSigns = { 
            ...newVitalSigns, 
            gnadt: lastAlgoResult[0] ?? null,
            gnadtConfidence: lastAlgoResult[1] ?? null,
          };
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
    gsensorData: new Map(),
    vitalSigns: {
      hr: null,
      hrConfidence: null,
      hrSnr: null,
      hrRef: [],
      spo2: null,
      spo2RValue: null,
      spo2Confidence: null,
      spo2ConfidenceLevel: null,
      spo2Ref: [],
      hrvRri: [],
      hrvConfidence: null,
      hrvRriCount: null,
      hrvRef: [],
      adt: null,
      adtConfidence: null,
      gnadt: null,
      gnadtConfidence: null,
    },
    refData: {
      hrValues: [],
      hrCount: 0,
      hrValid: false,
      hrvValues: [],
      hrvCount: 0,
      hrvValid: false,
      spo2Values: [],
      spo2Count: 0,
      spo2Valid: false,
      timestamp: 0,
    },
  }),
  
  updateRefData: (refData) => {
    const { vitalSigns } = get();
    const newVitalSigns = { ...vitalSigns };
    
    if (refData.hr_valid && refData.hr_count > 0) {
      newVitalSigns.hrRef = refData.hr_values.slice(0, refData.hr_count);
    } else {
      newVitalSigns.hrRef = [];
    }
    
    if (refData.hrv_valid && refData.hrv_count > 0) {
      newVitalSigns.hrvRef = refData.hrv_values.slice(0, refData.hrv_count);
    } else {
      newVitalSigns.hrvRef = [];
    }
    
    if (refData.spo2_valid && refData.spo2_count > 0) {
      newVitalSigns.spo2Ref = refData.spo2_values.slice(0, refData.spo2_count);
    } else {
      newVitalSigns.spo2Ref = [];
    }
    
    set({ 
      refData: {
        hrValues: refData.hr_values,
        hrCount: refData.hr_count,
        hrValid: refData.hr_valid,
        hrvValues: refData.hrv_values,
        hrvCount: refData.hrv_count,
        hrvValid: refData.hrv_valid,
        spo2Values: refData.spo2_values,
        spo2Count: refData.spo2_count,
        spo2Valid: refData.spo2_valid,
        timestamp: refData.timestamp,
      },
      vitalSigns: newVitalSigns,
    });
  },
  
  setChartGroups: (groups) => set({ chartGroups: groups }),
  setSelectedFunctionId: (id) => set({ selectedFunctionId: id }),
  setChartLegendSelected: (selected) => set({ chartLegendSelected: selected }),
  setIpdRawDataType: (type) => set({ ipdRawDataType: type }),
  setDisplayDurationSeconds: (seconds) => set({ displayDurationSeconds: seconds }),
  setMaxFramesCount: (count) => set({ maxFramesCount: count }),

  setSampleRateConfig: (config) => set({ sampleRateConfig: config }),
  
  setIsLinked: (value) => set({ isLinked: value }),
  
  setRpcConfig: (config) => set((state) => ({
    rpcConfig: { ...state.rpcConfig, ...config },
  })),
  
  initialize: async () => {
    const { isInitialized, isLoading } = get();
    if (isInitialized || isLoading) {
      return;
    }
    
    set({ isLoading: true, error: null });
    try {
      await gh3036Api.init();
      set({ isInitialized: true });
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.initialize'));
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
      const prefs = await preferencesApi.get();
      const gh3036Csv = prefs.gh3036Csv;
      if (gh3036Csv) {
        set({
          csvConfig: {
            enabled: gh3036Csv.enabled ?? true,
            output_dir: gh3036Csv.outputDir || '.',
          },
        });
      }
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
            txCharManuallyEdited: false,
            rxCharManuallyEdited: false,
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
  
  setTxCharManuallyEdited: (edited) => set((state) => ({
    channelConfig: { ...state.channelConfig, txCharManuallyEdited: edited }
  })),
  
  setRxCharManuallyEdited: (edited) => set((state) => ({
    channelConfig: { ...state.channelConfig, rxCharManuallyEdited: edited }
  })),
  
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
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.configureTx'));
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
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.configureRx'));
      set({ error: errorMsg });
      return false;
    } finally {
      set({ isLoading: false });
    }
  },
  
  updateCsvConfig: async (enabled, outputDir) => {
    set({ isLoading: true, error: null });
    try {
      await preferencesApi.updateGh3036Csv({
        enabled,
        outputDir: outputDir,
      });
      set({ 
        csvConfig: { enabled, output_dir: outputDir }
      });
      return true;
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.updateCsv'));
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
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.sendData'));
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
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.executeRpc'));
      set({ error: errorMsg });
      return null;
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
              }

              if (rxChannel?.device_id === deviceId) {
                set({ rxChannel: null });
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
  
  resetFactoryTest: () => {
    set((state) => ({
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
    }));
  },
  
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
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.startFactoryTest'));
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
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.stopFactoryTest'));
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
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.continueFactoryTest'));
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
        factoryTest: { 
          ...state.factoryTest, 
          configDir,
          thresholdConfig: null,
          thresholdValidation: null,
        },
      }));
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.setConfigDir'));
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
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.validateConfig'));
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
  
  subscribeFactoryTestEvents: async (listenerId?: number) => {
    const { eventListeners, _factoryTestSubscribing } = get();
    const subscribeId = listenerId ?? get().factoryTestListenerId + 1;
    
    if (eventListeners.factoryTest) {
      return;
    }

    if (_factoryTestSubscribing) {
      return;
    }
    
    set({ _factoryTestSubscribing: true, factoryTestListenerId: subscribeId });

    try {
      const factoryTestUnlisten = await listen<EventBusEvent>('event-bus', (event) => {
        const recvTs = getTs();
        const topic = event.payload.topic;

        if (topic === 'gh3036:factory_test_progress') {
          try {
            const progressEvent = decodePayload<FactoryTestProgressEvent>(event.payload);
            
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
            
            if (hasFactoryTestResult(progressEvent.status)) {
              set((state) => ({
                factoryTest: {
                  ...state.factoryTest,
                  isRunning: false,
                },
              }));
              
              Promise.all([
                factoryTestApi.getResult(),
                factoryTestApi.getEvaluationResult(),
              ]).then(([result, evaluationResult]) => {
                set((state) => ({
                  factoryTest: {
                    ...state.factoryTest,
                    result,
                    evaluationResult,
                  },
                }));
              }).catch((err) => {
                console.error(`[${getTs()}] [Gh3036Store] 获取产测终态结果失败:`, err);
              });
            }
          } catch (err) {
            console.error(`[${recvTs}] [Gh3036Store] 产测进度事件解码失败:`, err, 'raw payload:', event.payload);
          }
        }
      });
      
      if (!get()._factoryTestSubscribing || get().factoryTestListenerId !== subscribeId) {
        factoryTestUnlisten();
        return;
      }

      set((state) => ({
        eventListeners: {
          ...state.eventListeners,
          factoryTest: factoryTestUnlisten,
        },
        _factoryTestSubscribing: false,
      }));
    } catch (err) {
      set({ _factoryTestSubscribing: false });
      console.error(`[${getTs()}] [Gh3036Store] 订阅产测事件失败:`, err);
    }
  },
  
  unsubscribeFactoryTestEvents: (listenerId?: number) => {
    const { eventListeners, factoryTestListenerId } = get();
    
    if (listenerId !== undefined && listenerId !== factoryTestListenerId) {
      return;
    }

    set({ _factoryTestSubscribing: false });

    if (eventListeners.factoryTest) {
      eventListeners.factoryTest();
      set((state) => ({
        eventListeners: {
          ...state.eventListeners,
          factoryTest: undefined,
        },
        factoryTestListenerId: 0,
      }));
    }
  },
  
  setThresholdConfig: (config) => set((state) => ({
    factoryTest: { ...state.factoryTest, thresholdConfig: config },
  })),
  
  setThresholdValidation: (validation) => set((state) => ({
    factoryTest: { ...state.factoryTest, thresholdValidation: validation },
  })),
  
  setEvaluationResult: (result) => set((state) => ({
    factoryTest: { ...state.factoryTest, evaluationResult: result },
  })),
  
  loadThresholdConfig: async () => {
    try {
      const config = await factoryTestApi.getThresholdConfig();
      set((state) => ({
        factoryTest: { ...state.factoryTest, thresholdConfig: config },
      }));
    } catch (err) {
      console.error('[Gh3036Store] 加载卡控配置失败:', err);
    }
  },
  
  validateThresholdConfig: async () => {
    try {
      const validation = await factoryTestApi.validateThresholdConfig();
      set((state) => ({
        factoryTest: { ...state.factoryTest, thresholdValidation: validation },
      }));
    } catch (err) {
      const errorMsg = formatErrorMessage(err, i18n.t('gh3036:errors.validateThreshold'));
      set((state) => ({
        factoryTest: {
          ...state.factoryTest,
          thresholdValidation: {
            is_valid: false,
            errors: [errorMsg],
            warnings: [],
            tests_status: {
              base_noise: { enabled: false, has_global_threshold: false, channel_rules_count: 0 },
              ppg_noise: { enabled: false, has_global_threshold: false, channel_rules_count: 0 },
              lpctr: { enabled: false, has_global_threshold: false, channel_rules_count: 0 },
              lplctr: { enabled: false, has_global_threshold: false, channel_rules_count: 0 },
            },
          },
        },
      }));
    }
  },
  
  loadEvaluationResult: async () => {
    try {
      const result = await factoryTestApi.getEvaluationResult();
      set((state) => ({
        factoryTest: { ...state.factoryTest, evaluationResult: result },
      }));
    } catch (err) {
      console.error('[Gh3036Store] 加载判断结果失败:', err);
    }
  },
}),
{
  name: 'gh3036-chart-settings',
  partialize: (state) => ({
    chartLegendSelected: state.chartLegendSelected ? { ...state.chartLegendSelected } : {},
    ipdRawDataType: state.ipdRawDataType || 'ipd',
    displayDurationSeconds: state.displayDurationSeconds || 10,
    sampleRateConfig: state.sampleRateConfig || { 0: 5, 1: 25, 2: 25, 3: 25, 4: 25 },
  }),
  merge: (persisted, current) => ({
    ...current,
    chartLegendSelected: (persisted as { chartLegendSelected?: Record<string, boolean> })?.chartLegendSelected || {},
    ipdRawDataType: (persisted as { ipdRawDataType?: 'ipd' | 'rawdata' })?.ipdRawDataType || 'ipd',
    displayDurationSeconds: (persisted as { displayDurationSeconds?: number })?.displayDurationSeconds || 10,
    sampleRateConfig: (persisted as { sampleRateConfig?: Record<number, number> })?.sampleRateConfig || { 0: 5, 1: 25, 2: 25, 3: 25, 4: 25 },
  }),
}
  )
);
