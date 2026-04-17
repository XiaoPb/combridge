import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { useConfigStore } from '../stores/configStore';

import zhCNCommon from '../locales/zh-CN/common.json';
import zhCNSidebar from '../locales/zh-CN/sidebar.json';
import zhCNSerial from '../locales/zh-CN/serial.json';
import zhCNBle from '../locales/zh-CN/ble.json';
import zhCNProtocol from '../locales/zh-CN/protocol.json';
import zhCNDashboard from '../locales/zh-CN/dashboard.json';
import zhCNSystem from '../locales/zh-CN/system.json';
import zhCNWaveform from '../locales/zh-CN/waveform.json';
import zhCNHome from '../locales/zh-CN/home.json';
import zhCNGh3036 from '../locales/zh-CN/gh3036.json';

import enUSCommon from '../locales/en-US/common.json';
import enUSSidebar from '../locales/en-US/sidebar.json';
import enUSSerial from '../locales/en-US/serial.json';
import enUSBle from '../locales/en-US/ble.json';
import enUSProtocol from '../locales/en-US/protocol.json';
import enUSDashboard from '../locales/en-US/dashboard.json';
import enUSSystem from '../locales/en-US/system.json';
import enUSWaveform from '../locales/en-US/waveform.json';
import enUSHome from '../locales/en-US/home.json';
import enUSGh3036 from '../locales/en-US/gh3036.json';

const resources = {
  'zh-CN': {
    common: zhCNCommon,
    sidebar: zhCNSidebar,
    serial: zhCNSerial,
    ble: zhCNBle,
    protocol: zhCNProtocol,
    dashboard: zhCNDashboard,
    system: zhCNSystem,
    waveform: zhCNWaveform,
    home: zhCNHome,
    gh3036: zhCNGh3036,
  },
  'en-US': {
    common: enUSCommon,
    sidebar: enUSSidebar,
    serial: enUSSerial,
    ble: enUSBle,
    protocol: enUSProtocol,
    dashboard: enUSDashboard,
    system: enUSSystem,
    waveform: enUSWaveform,
    home: enUSHome,
    gh3036: enUSGh3036,
  },
};

const savedLanguage = useConfigStore.getState().getConfig().language || 'zh-CN';

i18n.use(initReactI18next).init({
  resources,
  lng: savedLanguage,
  fallbackLng: 'zh-CN',
  defaultNS: 'common',
  interpolation: {
    escapeValue: false,
  },
});

export default i18n;

export const changeLanguage = (lng: 'zh-CN' | 'en-US') => {
  i18n.changeLanguage(lng);
  useConfigStore.getState().updateConfig({ language: lng });
};

export const getCurrentLanguage = () => i18n.language as 'zh-CN' | 'en-US';
