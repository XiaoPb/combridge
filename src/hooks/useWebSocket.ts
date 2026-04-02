import { useEffect, useCallback, useRef, useState } from 'react';
import { message } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  useConnectionStore,
  generateConnectionId,
  type ConnectionInfo,
  type WebSocketConnection,
  type ConnectionStatus,
} from '../stores/connectionStore';

export interface WebSocketConfig {
  url: string;
  autoReconnect?: boolean;
  maxReconnectAttempts?: number;
  reconnectInterval?: number;
  headers?: Record<string, string>;
}

export interface WebSocketMessage {
  id: string;
  timestamp: number;
  direction: 'send' | 'receive';
  data: number[];
  type: 'binary' | 'text';
}

interface WebSocketState {
  messages: WebSocketMessage[];
  isConnected: boolean;
  connectionId: string | null;
}

export const useWebSocket = () => {
  const {
    connections,
    activeConnectionId,
    addConnection,
    removeConnection,
    updateConnection,
    setActiveConnection,
    setError,
  } = useConnectionStore();

  const [state, setState] = useState<WebSocketState>({
    messages: [],
    isConnected: false,
    connectionId: null,
  });

  const listenersRef = useRef<UnlistenFn[]>([]);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const setupListeners = async () => {
      const unlistenMessage = await listen<{ id: string; data: number[]; timestamp: number }>(
        'websocket-message',
        (event) => {
          const { id, data, timestamp } = event.payload;
          setState((prev) => ({
            ...prev,
            messages: [
              ...prev.messages.slice(-999),
              {
                id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
                timestamp,
                direction: 'receive',
                data,
                type: 'binary',
              },
            ],
          }));
          updateConnection(id, {
            lastActivity: timestamp,
            bytesReceived: (connections.find((c) => c.id === id)?.bytesReceived || 0) + data.length,
          });
        }
      );

      const unlistenStatus = await listen<{ id: string; status: string; error?: string }>(
        'websocket-status',
        (event) => {
          const { id, status, error } = event.payload;
          const wsStatus = status as ConnectionStatus;
          updateConnection(id, {
            status: wsStatus,
            error,
          });
          if (status === 'connected') {
            setState((prev) => ({ ...prev, isConnected: true, connectionId: id }));
            message.success('WebSocket 已连接');
          } else if (status === 'disconnected') {
            setState((prev) => ({ ...prev, isConnected: false, connectionId: null }));
            message.info('WebSocket 已断开');
          } else if (status === 'error') {
            message.error(`WebSocket 错误: ${error || '未知错误'}`);
          }
        }
      );

      listenersRef.current = [unlistenMessage, unlistenStatus];
    };

    setupListeners();

    return () => {
      listenersRef.current.forEach((unlisten) => unlisten());
      listenersRef.current = [];
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current);
      }
    };
  }, [connections, updateConnection]);

  const connect = useCallback(async (config: WebSocketConfig) => {
    const connectionId = generateConnectionId('websocket');
    const connection: WebSocketConnection = {
      id: connectionId,
      type: 'websocket',
      name: config.url,
      status: 'connecting',
      url: config.url,
      bytesReceived: 0,
      bytesSent: 0,
      reconnectAttempts: 0,
      maxReconnectAttempts: config.maxReconnectAttempts || 5,
    };

    addConnection(connection);
    setActiveConnection(connectionId);
    setError(null);

    try {
      await invoke('connect_websocket', {
        config: {
          url: config.url,
          auto_reconnect: config.autoReconnect ?? true,
          max_reconnect_attempts: config.maxReconnectAttempts ?? 5,
          reconnect_interval: config.reconnectInterval ?? 3000,
          headers: config.headers || {},
        },
      });

      updateConnection(connectionId, {
        status: 'connected',
        connectedAt: Date.now(),
        lastActivity: Date.now(),
      });

      setState((prev) => ({
        ...prev,
        isConnected: true,
        connectionId,
        messages: [],
      }));

      return connectionId;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '连接失败';
      updateConnection(connectionId, {
        status: 'error',
        error: errorMsg,
      });
      setError(errorMsg);
      message.error(`WebSocket 连接失败: ${errorMsg}`);
      throw err;
    }
  }, [addConnection, setActiveConnection, setError, updateConnection]);

  const disconnect = useCallback(async (connectionId?: string) => {
    const id = connectionId || state.connectionId;
    if (!id) {
      message.warning('没有活动的 WebSocket 连接');
      return;
    }

    try {
      await invoke('disconnect_websocket', { connectionId: id });
      updateConnection(id, { status: 'disconnected' });
      removeConnection(id);
      setState((prev) => ({
        ...prev,
        isConnected: false,
        connectionId: null,
      }));
      message.success('WebSocket 已断开');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '断开连接失败';
      setError(errorMsg);
      message.error(`断开连接失败: ${errorMsg}`);
      throw err;
    }
  }, [state.connectionId, updateConnection, removeConnection, setError]);

  const send = useCallback(async (data: number[] | string, connectionId?: string) => {
    const id = connectionId || state.connectionId;
    if (!id) {
      message.warning('没有活动的 WebSocket 连接');
      return;
    }

    const bytes = typeof data === 'string' ? Array.from(new TextEncoder().encode(data)) : data;
    if (bytes.length === 0) {
      message.warning('发送数据不能为空');
      return;
    }

    try {
      await invoke('send_websocket_message', {
        connectionId: id,
        message: bytes,
      });

      setState((prev) => ({
        ...prev,
        messages: [
          ...prev.messages.slice(-999),
          {
            id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
            timestamp: Date.now(),
            direction: 'send',
            data: bytes,
            type: typeof data === 'string' ? 'text' : 'binary',
          },
        ],
      }));

      updateConnection(id, {
        lastActivity: Date.now(),
        bytesSent: (connections.find((c) => c.id === id)?.bytesSent || 0) + bytes.length,
      });
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : '发送失败';
      message.error(`发送失败: ${errorMsg}`);
      throw err;
    }
  }, [state.connectionId, connections, updateConnection]);

  const clearMessages = useCallback(() => {
    setState((prev) => ({ ...prev, messages: [] }));
  }, []);

  const getStatus = useCallback(async (connectionId?: string) => {
    const id = connectionId || state.connectionId;
    if (!id) return null;
    return invoke<ConnectionStatus>('get_websocket_status', { connectionId: id });
  }, [state.connectionId]);

  const getAllStatus = useCallback(async () => {
    return invoke<Record<string, ConnectionStatus>>('get_all_websocket_status');
  }, []);

  const getActiveConnection = useCallback((): ConnectionInfo | undefined => {
    if (!activeConnectionId) return undefined;
    return connections.find((c) => c.id === activeConnectionId);
  }, [activeConnectionId, connections]);

  return {
    connections,
    activeConnectionId,
    messages: state.messages,
    isConnected: state.isConnected,
    connectionId: state.connectionId,
    connect,
    disconnect,
    send,
    clearMessages,
    getStatus,
    getAllStatus,
    setActiveConnection,
    getActiveConnection,
  };
};
