import { useEffect, useRef, useCallback } from 'react';
import { useWaveformStore } from '../stores/waveformStore';

export const useWaveform = (bufferId: string | null) => {
  const store = useWaveformStore();
  const intervalRef = useRef<number | null>(null);

  const startAutoRefresh = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
    }

    if (bufferId) {
      intervalRef.current = window.setInterval(() => {
        store.readData(bufferId);
      }, store.refreshInterval);
      store.startRefresh();
    }
  }, [bufferId, store]);

  const stopAutoRefresh = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
    store.stopRefresh();
  }, [store]);

  useEffect(() => {
    if (store.isRunning && bufferId) {
      startAutoRefresh();
    } else {
      stopAutoRefresh();
    }

    return () => {
      stopAutoRefresh();
    };
  }, [store.isRunning, bufferId, startAutoRefresh, stopAutoRefresh]);

  useEffect(() => {
    store.refreshBuffers();
  }, []);

  return {
    ...store,
    startAutoRefresh,
    stopAutoRefresh,
  };
};
