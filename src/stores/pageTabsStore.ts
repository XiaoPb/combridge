import { create } from 'zustand';

interface PageTabsState {
  systemActiveTab: 'info' | 'logs' | 'settings';
  protocolActiveTab: 'editor' | 'bind';
  waveformActiveTab: 'realtime' | 'csvLoader';
  setSystemActiveTab: (tab: 'info' | 'logs' | 'settings') => void;
  setProtocolActiveTab: (tab: 'editor' | 'bind') => void;
  setWaveformActiveTab: (tab: 'realtime' | 'csvLoader') => void;
}

export const usePageTabsStore = create<PageTabsState>((set) => ({
  systemActiveTab: 'info',
  protocolActiveTab: 'editor',
  waveformActiveTab: 'realtime',
  setSystemActiveTab: (tab) => set({ systemActiveTab: tab }),
  setProtocolActiveTab: (tab) => set({ protocolActiveTab: tab }),
  setWaveformActiveTab: (tab) => set({ waveformActiveTab: tab }),
}));
