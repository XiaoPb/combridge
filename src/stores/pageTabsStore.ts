import { create } from 'zustand';

interface PageTabsState {
  systemActiveTab: 'info' | 'logs' | 'settings';
  protocolActiveTab: 'editor' | 'bind';
  waveformActiveTab: 'realtime' | 'csvLoader';
  gh3036ActiveTab: 'config' | 'monitor' | 'version' | 'factory';
  setSystemActiveTab: (tab: 'info' | 'logs' | 'settings') => void;
  setProtocolActiveTab: (tab: 'editor' | 'bind') => void;
  setWaveformActiveTab: (tab: 'realtime' | 'csvLoader') => void;
  setGh3036ActiveTab: (tab: 'config' | 'monitor' | 'version' | 'factory') => void;
}

export const usePageTabsStore = create<PageTabsState>((set) => ({
  systemActiveTab: 'info',
  protocolActiveTab: 'editor',
  waveformActiveTab: 'realtime',
  gh3036ActiveTab: 'factory',
  setSystemActiveTab: (tab) => set({ systemActiveTab: tab }),
  setProtocolActiveTab: (tab) => set({ protocolActiveTab: tab }),
  setWaveformActiveTab: (tab) => set({ waveformActiveTab: tab }),
  setGh3036ActiveTab: (tab) => set({ gh3036ActiveTab: tab }),
}));
