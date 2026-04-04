import { useEffect, useState } from 'react';
import { getState, subscribeToStateChanges } from '../api/stateApi';
import type { AppState, DeviceChannel } from '../types/state';

export function useAppState(): AppState | null {
  const [state, setState] = useState<AppState | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let mounted = true;

    const loadState = async () => {
      try {
        const initialState = await getState();
        if (mounted) {
          setState(initialState);
          setLoading(false);
        }
      } catch (error) {
        console.error('Failed to load state:', error);
        if (mounted) {
          setLoading(false);
        }
      }
    };

    loadState();

    const unsubscribe = subscribeToStateChanges((newState) => {
      if (mounted) {
        setState(newState);
      }
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, []);

  return loading ? null : state;
}

export function useActiveChannel(): DeviceChannel | null {
  const state = useAppState();
  
  if (!state || !state.activeChannelId) {
    return null;
  }
  
  return state.channels.find(c => c.id === state.activeChannelId) || null;
}

export function useChannel(channelId: string | null): DeviceChannel | null {
  const state = useAppState();
  
  if (!state || !channelId) {
    return null;
  }
  
  return state.channels.find(c => c.id === channelId) || null;
}

export function useConnectedChannels(): DeviceChannel[] {
  const state = useAppState();
  
  if (!state) {
    return [];
  }
  
  return state.channels.filter(c => c.connected);
}

export function useChannelsByType(type: 'serial' | 'ble'): DeviceChannel[] {
  const state = useAppState();
  
  if (!state) {
    return [];
  }
  
  return state.channels.filter(c => c.type === type);
}

export function useWindowState() {
  const state = useAppState();
  
  return state?.windowState || null;
}
