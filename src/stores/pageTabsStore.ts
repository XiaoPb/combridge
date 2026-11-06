import { create } from 'zustand';

/** GH3220 页签与 GH3036 对齐：配置/监控/版本 + 单独一个调试页 */
export type Gh3220TabKey = 'config' | 'monitor' | 'version' | 'debug';

interface PageTabsState {
  systemActiveTab: 'info' | 'logs' | 'settings' | 'integration';
  protocolActiveTab: 'editor' | 'bind';
  waveformActiveTab: 'realtime' | 'csvLoader';
  gh3036ActiveTab: 'config' | 'monitor' | 'version' | 'factory' | 'threshold';
  gh3220ActiveTab: Gh3220TabKey;
  otaActiveTab: 'upgrade' | 'history';
  setSystemActiveTab: (tab: 'info' | 'logs' | 'settings' | 'integration') => void;
  setProtocolActiveTab: (tab: 'editor' | 'bind') => void;
  setWaveformActiveTab: (tab: 'realtime' | 'csvLoader') => void;
  setGh3036ActiveTab: (tab: 'config' | 'monitor' | 'version' | 'factory' | 'threshold') => void;
  setGh3220ActiveTab: (tab: Gh3220TabKey) => void;
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
