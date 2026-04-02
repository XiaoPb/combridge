import { create } from 'zustand';

export type ConnectionType = 'serial' | 'ble' | 'websocket';
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'disconnecting' | 'error';

export interface ConnectionInfo {
  id: string;
  type: ConnectionType;
  name: string;
  status: ConnectionStatus;
  connectedAt?: number;
  lastActivity?: number;
  bytesReceived: number;
  bytesSent: number;
  error?: string;
  metadata?: Record<string, unknown>;
}

export interface WebSocketConnection extends ConnectionInfo {
  type: 'websocket';
  url: string;
  reconnectAttempts: number;
  maxReconnectAttempts: number;
}

interface ConnectionState {
  connections: ConnectionInfo[];
  activeConnectionId: string | null;
  isConnecting: boolean;
  error: string | null;

  addConnection: (connection: ConnectionInfo) => void;
  removeConnection: (id: string) => void;
  updateConnection: (id: string, updates: Partial<ConnectionInfo>) => void;
  setActiveConnection: (id: string | null) => void;
  setIsConnecting: (isConnecting: boolean) => void;
  setError: (error: string | null) => void;
  clearAllConnections: () => void;
  getConnection: (id: string) => ConnectionInfo | undefined;
  getActiveConnection: () => ConnectionInfo | undefined;
  getConnectionsByType: (type: ConnectionType) => ConnectionInfo[];
}

const initialState = {
  connections: [],
  activeConnectionId: null,
  isConnecting: false,
  error: null,
};

export const useConnectionStore = create<ConnectionState>((set, get) => ({
  ...initialState,

  addConnection: (connection) =>
    set((state) => {
      const exists = state.connections.find((c) => c.id === connection.id);
      if (exists) {
        return {
          connections: state.connections.map((c) =>
            c.id === connection.id ? connection : c
          ),
        };
      }
      return { connections: [...state.connections, connection] };
    }),

  removeConnection: (id) =>
    set((state) => ({
      connections: state.connections.filter((c) => c.id !== id),
      activeConnectionId: state.activeConnectionId === id ? null : state.activeConnectionId,
    })),

  updateConnection: (id, updates) =>
    set((state) => ({
      connections: state.connections.map((c) =>
        c.id === id ? { ...c, ...updates } : c
      ),
    })),

  setActiveConnection: (activeConnectionId) => set({ activeConnectionId }),

  setIsConnecting: (isConnecting) => set({ isConnecting }),

  setError: (error) => set({ error }),

  clearAllConnections: () => set({ connections: [], activeConnectionId: null }),

  getConnection: (id) => get().connections.find((c) => c.id === id),

  getActiveConnection: () => {
    const state = get();
    if (!state.activeConnectionId) return undefined;
    return state.connections.find((c) => c.id === state.activeConnectionId);
  },

  getConnectionsByType: (type) => get().connections.filter((c) => c.type === type),
}));

export const generateConnectionId = (type: ConnectionType): string => {
  return `${type}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
};

export const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

export const getConnectionStatusColor = (status: ConnectionStatus): string => {
  switch (status) {
    case 'connected':
      return '#52c41a';
    case 'connecting':
    case 'disconnecting':
      return '#faad14';
    case 'error':
      return '#ff4d4f';
    case 'disconnected':
    default:
      return '#8c8c8c';
  }
};

export const getConnectionStatusText = (status: ConnectionStatus): string => {
  switch (status) {
    case 'connected':
      return '已连接';
    case 'connecting':
      return '连接中';
    case 'disconnecting':
      return '断开中';
    case 'error':
      return '错误';
    case 'disconnected':
    default:
      return '已断开';
  }
};
