import { useEffect, useState } from 'react';
import { getState, subscribeToStateChanges } from '../api/stateApi';
import type { AppState, Device } from '../types/state';

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

export function useActiveDevice(): Device | null {
  const state = useAppState();
  
  if (!state || !state.activeDeviceId) {
    return null;
  }
  
  return state.devices[state.activeDeviceId] || null;
}

export function useDevice(deviceId: string | null): Device | null {
  const state = useAppState();
  
  if (!state || !deviceId) {
    return null;
  }
  
  return state.devices[deviceId] || null;
}

export function useConnectedDevices(): Device[] {
  const state = useAppState();
  
  if (!state) {
    return [];
  }
  
  return Object.values(state.devices).filter(d => d.connected);
}

export function useDevicesByType(type: 'serial' | 'ble'): Device[] {
  const state = useAppState();
  
  if (!state) {
    return [];
  }
  
  return Object.values(state.devices).filter(d => d.type === type);
}

export function useWindowState() {
  const state = useAppState();
  
  return state?.windowState || null;
}

export function useAppSettings() {
  const state = useAppState();
  
  return state?.settings || null;
}
