import { create } from 'zustand';

interface PageTabsState {
  systemActiveTab: 'info' | 'logs' | 'settings';
  protocolActiveTab: 'editor' | 'bind';
  setSystemActiveTab: (tab: 'info' | 'logs' | 'settings') => void;
  setProtocolActiveTab: (tab: 'editor' | 'bind') => void;
}

export const usePageTabsStore = create<PageTabsState>((set) => ({
  systemActiveTab: 'info',
  protocolActiveTab: 'editor',
  setSystemActiveTab: (tab) => set({ systemActiveTab: tab }),
  setProtocolActiveTab: (tab) => set({ protocolActiveTab: tab }),
}));
