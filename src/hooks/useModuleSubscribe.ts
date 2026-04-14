import { useEffect, useCallback, useRef } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface EventBusEvent {
  topic: string;
  payload: string;
  timestamp: number;
}

export interface ModuleSubscribeOptions<T> {
  topic: string;
  onEvent: (payload: T) => void;
  enabled?: boolean;
}

export function useModuleSubscribe<T>(options: ModuleSubscribeOptions<T>) {
  const { topic, onEvent, enabled = true } = options;
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const onEventRef = useRef(onEvent);

  onEventRef.current = onEvent;

  const subscribe = useCallback(async () => {
    if (unlistenRef.current) {
      return;
    }

    try {
      unlistenRef.current = await listen<EventBusEvent>('event-bus', (event) => {
        if (event.payload.topic === topic) {
          try {
            const parsedPayload = JSON.parse(event.payload.payload) as T;
            onEventRef.current(parsedPayload);
          } catch (err) {
            console.error(`[useModuleSubscribe] Failed to parse payload for topic "${topic}":`, err);
          }
        }
      });
    } catch (err) {
      console.error(`[useModuleSubscribe] Failed to subscribe to topic "${topic}":`, err);
    }
  }, [topic]);

  const unsubscribe = useCallback(() => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (enabled) {
      subscribe();
    }

    return () => {
      unsubscribe();
    };
  }, [enabled, subscribe, unsubscribe]);

  return {
    subscribe,
    unsubscribe,
    isSubscribed: () => unlistenRef.current !== null,
  };
}

export interface MultiModuleSubscribeOptions {
  topics: string[];
  onEvent: (topic: string, payload: unknown) => void;
  enabled?: boolean;
}

export function useMultiModuleSubscribe(options: MultiModuleSubscribeOptions) {
  const { topics, onEvent, enabled = true } = options;
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const onEventRef = useRef(onEvent);

  onEventRef.current = onEvent;

  const subscribe = useCallback(async () => {
    if (unlistenRef.current) {
      return;
    }

    try {
      unlistenRef.current = await listen<EventBusEvent>('event-bus', (event) => {
        const { topic, payload } = event.payload;
        if (topics.includes(topic)) {
          try {
            const parsedPayload = JSON.parse(payload);
            onEventRef.current(topic, parsedPayload);
          } catch (err) {
            console.error(`[useMultiModuleSubscribe] Failed to parse payload for topic "${topic}":`, err);
          }
        }
      });
    } catch (err) {
      console.error('[useMultiModuleSubscribe] Failed to subscribe:', err);
    }
  }, [topics]);

  const unsubscribe = useCallback(() => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (enabled) {
      subscribe();
    }

    return () => {
      unsubscribe();
    };
  }, [enabled, subscribe, unsubscribe]);

  return {
    subscribe,
    unsubscribe,
    isSubscribed: () => unlistenRef.current !== null,
  };
}
