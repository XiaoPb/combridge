import { create } from 'zustand';
import type { PluginInfo, PluginState } from '../api/types';

export interface ProtocolBinding {
  pluginId: string;
  deviceId: string;
  boundAt: number;
}

interface ProtocolState {
  protocols: PluginInfo[];
  bindings: ProtocolBinding[];
  currentProtocol: string | null;
  isLoading: boolean;
  error: string | null;

  setProtocols: (protocols: PluginInfo[]) => void;
  addProtocol: (protocol: PluginInfo) => void;
  updateProtocol: (pluginId: string, updates: Partial<PluginInfo>) => void;
  removeProtocol: (pluginId: string) => void;
  setBindings: (bindings: ProtocolBinding[]) => void;
  addBinding: (binding: ProtocolBinding) => void;
  removeBinding: (pluginId: string, deviceId: string) => void;
  setCurrentProtocol: (pluginId: string | null) => void;
  setIsLoading: (isLoading: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

const initialState = {
  protocols: [],
  bindings: [],
  currentProtocol: null,
  isLoading: false,
  error: null,
};

export const useProtocolStore = create<ProtocolState>((set) => ({
  ...initialState,

  setProtocols: (protocols) => set({ protocols }),

  addProtocol: (protocol) =>
    set((state) => {
      const exists = state.protocols.find((p) => p.id === protocol.id);
      if (exists) {
        return {
          protocols: state.protocols.map((p) =>
            p.id === protocol.id ? protocol : p
          ),
        };
      }
      return { protocols: [...state.protocols, protocol] };
    }),

  updateProtocol: (pluginId, updates) =>
    set((state) => ({
      protocols: state.protocols.map((p) =>
        p.id === pluginId ? { ...p, ...updates } : p
      ),
    })),

  removeProtocol: (pluginId) =>
    set((state) => ({
      protocols: state.protocols.filter((p) => p.id !== pluginId),
      currentProtocol: state.currentProtocol === pluginId ? null : state.currentProtocol,
    })),

  setBindings: (bindings) => set({ bindings }),

  addBinding: (binding) =>
    set((state) => {
      const exists = state.bindings.find(
        (b) => b.pluginId === binding.pluginId && b.deviceId === binding.deviceId
      );
      if (exists) return state;
      return { bindings: [...state.bindings, binding] };
    }),

  removeBinding: (pluginId, deviceId) =>
    set((state) => ({
      bindings: state.bindings.filter(
        (b) => !(b.pluginId === pluginId && b.deviceId === deviceId)
      ),
    })),

  setCurrentProtocol: (currentProtocol) => set({ currentProtocol }),

  setIsLoading: (isLoading) => set({ isLoading }),

  setError: (error) => set({ error }),

  reset: () => set(initialState),
}));

export const getPluginStateColor = (state: PluginState): string => {
  switch (state) {
    case 'Enabled':
      return 'green';
    case 'Loaded':
      return 'blue';
    case 'Disabled':
      return 'orange';
    case 'Error':
      return 'red';
    case 'Unloaded':
    default:
      return 'default';
  }
};

export const getPluginStateText = (state: PluginState): string => {
  switch (state) {
    case 'Enabled':
      return '已启用';
    case 'Loaded':
      return '已加载';
    case 'Disabled':
      return '已禁用';
    case 'Error':
      return '错误';
    case 'Unloaded':
    default:
      return '未加载';
  }
};
