import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { 
  Gh3036ChannelConfig, 
  Gh3036CsvConfig, 
  Gh3036RpcCommand 
} from '../api/types';
import { gh3036Api } from '../api/gh3036';
import { decodePayload, type EventBusEvent } from '../utils/msgpack';
import type { Gh3036FramePayload, Gh3036FramesPayload } from '../api/events';

export type Gh3036EventData = {
  event_type: string;
  timestamp: number;
  data: Record<string, unknown>;
};

export type Gh3036FrameEventData = Gh3036FramePayload;

interface Gh3036State {
  isInitialized: boolean;
  isLoading: boolean;
  error: string | null;
  
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
  
  chartGroups: ChartGroupConfig[];
  selectedFunctionId: number | null;
  
  isLinked: boolean;
  
  eventListeners: {
    event?: UnlistenFn;
    frame?: UnlistenFn;
    deviceDisconnected?: UnlistenFn;
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
  
  initialize: () => Promise<void>;
  loadChannels: () => Promise<void>;
  loadCsvConfig: () => Promise<void>;
  loadRpcCommands: () => Promise<void>;
  
  configureTxChannel: (channelType: 'serial' | 'ble', deviceId: string, characteristicUuid?: string) => Promise<boolean>;
  configureRxChannel: (channelType: 'serial' | 'ble', deviceId: string, characteristicUuid?: string) => Promise<boolean>;
  
  updateCsvConfig: (enabled: boolean, outputDir: string) => Promise<boolean>;
  
  sendData: (data: number[]) => Promise<boolean>;
  
  executeRpc: (commandKey: string, params: string[]) => Promise<boolean>;
  subscribeEvents: () => Promise<void>;
  unsubscribeEvents: () => void;
  loadLibraryStatus: () => Promise<void>;
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
  
  chartGroups: [],
  selectedFunctionId: null,
  
  isLinked: false,
  
  eventListeners: {},
  
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
    const { framesData, maxFramesCount } = get();
    const newFramesData = new Map(framesData);
    
    const existing = newFramesData.get(frames.function_id);
    if (existing) {
      const combined: Gh3036FramesPayload = {
        function_id: frames.function_id,
        function_name: frames.function_name,
        frame_count: existing.frame_count + frames.frame_count,
        channel_count: frames.channel_count,
        timestamps: [...existing.timestamps, ...frames.timestamps].slice(-maxFramesCount * 10),
        frame_ids: [...existing.frame_ids, ...frames.frame_ids].slice(-maxFramesCount * 10),
        channels: existing.channels.map((ch, i) => [...ch, ...(frames.channels[i] || [])].slice(-maxFramesCount * 10)),
        rawdata: existing.rawdata.map((rd, i) => [...rd, ...(frames.rawdata[i] || [])].slice(-maxFramesCount * 10)),
        gs_data: existing.gs_data.map((gs, i) => [...gs, ...(frames.gs_data[i] || [])].slice(-maxFramesCount * 10)),
      };
      newFramesData.set(frames.function_id, combined);
    } else {
      newFramesData.set(frames.function_id, frames);
    }
    
    set({ 
      framesData: newFramesData,
      selectedFunctionId: get().selectedFunctionId ?? frames.function_id 
    });
  },
  
  clearWaveformData: () => set({ framesData: new Map(), chartGroups: [], selectedFunctionId: null }),
  
  setChartGroups: (groups) => set({ chartGroups: groups }),
  setSelectedFunctionId: (id) => set({ selectedFunctionId: id }),
  
  setIsLinked: (value) => set({ isLinked: value }),
  
  initialize: async () => {
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
      await gh3036Api.executeRpc(commandKey, params);
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '执行RPC指令失败';
      set({ error: errorMsg });
      return false;
    } finally {
      set({ isLoading: false });
    }
  },
  
  subscribeEvents: async () => {
    const { eventListeners } = get();
    
    if (eventListeners.event || eventListeners.frame) {
      return;
    }
    
    try {
      await gh3036Api.subscribeEvents();
      
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
}));
