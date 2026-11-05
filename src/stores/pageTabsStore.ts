import { create } from 'zustand';

interface PageTabsState {
  systemActiveTab: 'info' | 'logs' | 'settings' | 'integration';
  protocolActiveTab: 'editor' | 'bind';
  waveformActiveTab: 'realtime' | 'csvLoader';
  gh3036ActiveTab: 'config' | 'monitor' | 'version' | 'factory' | 'threshold';
  gh3220ActiveTab:
    | 'config'
    | 'monitor'
    | 'version'
    | 'capture'
    | 'control'
    | 'events'
    | 'debug';
  otaActiveTab: 'upgrade' | 'history';
  setSystemActiveTab: (tab: 'info' | 'logs' | 'settings' | 'integration') => void;
  setProtocolActiveTab: (tab: 'editor' | 'bind') => void;
  setWaveformActiveTab: (tab: 'realtime' | 'csvLoader') => void;
  setGh3036ActiveTab: (tab: 'config' | 'monitor' | 'version' | 'factory' | 'threshold') => void;
  setGh3220ActiveTab: (
    tab:
      | 'config'
      | 'monitor'
      | 'version'
      | 'capture'
      | 'control'
      | 'events'
      | 'debug'
  ) => void;
  setOtaActiveTab: (tab: 'upgrade' | 'history') => void;
}

export const usePageTabsStore = create<PageTabsState>((set) => ({
  systemActiveTab: 'info',
  protocolActiveTab: 'editor',
  waveformActiveTab: 'realtime',
  gh3036ActiveTab: 'monitor',
  gh3220ActiveTab: 'monitor',
  otaActiveTab: 'upgrade',
  setSystemActiveTab: (tab) => set({ systemActiveTab: tab }),
  setProtocolActiveTab: (tab) => set({ protocolActiveTab: tab }),
  setWaveformActiveTab: (tab) => set({ waveformActiveTab: tab }),
  setGh3036ActiveTab: (tab) => set({ gh3036ActiveTab: tab }),
  setGh3220ActiveTab: (tab) => set({ gh3220ActiveTab: tab }),
  setOtaActiveTab: (tab) => set({ otaActiveTab: tab }),
}));
