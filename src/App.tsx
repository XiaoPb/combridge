import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConfigProvider, theme, Spin } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import enUS from 'antd/locale/en_US';
import { lazy, Suspense, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { MainLayout } from './components';
import { useTheme } from './hooks';
import configService from './services/configService';
import { initSerialEventListeners, cleanupSerialEventListeners } from './services/eventListeners';
import './styles/global.css';

const { defaultAlgorithm, darkAlgorithm } = theme;

const SerialPage = lazy(() => import('./pages/Serial'));
const BlePage = lazy(() => import('./pages/Ble'));
const ProtocolPage = lazy(() => import('./pages/Protocol'));
const SystemPage = lazy(() => import('./pages/System'));
const WaveformPage = lazy(() => import('./pages/Waveform'));

const PageLoader = () => (
  <div style={{ 
    display: 'flex', 
    alignItems: 'center', 
    justifyContent: 'center', 
    height: '100%',
    width: '100%' 
  }}>
    <Spin size="large" />
  </div>
);

function App() {
  const { isDark } = useTheme();
  const { i18n } = useTranslation();
  const [antdLocale, setAntdLocale] = useState(zhCN);

  useEffect(() => {
    const config = configService.getConfig();
    const savedLanguage = config.language || 'zh-CN';
    if (i18n.language !== savedLanguage) {
      i18n.changeLanguage(savedLanguage);
    }
    setAntdLocale(savedLanguage === 'zh-CN' ? zhCN : enUS);

    const unsubscribe = configService.subscribe((newConfig) => {
      const newLang = newConfig.language || 'zh-CN';
      if (i18n.language !== newLang) {
        i18n.changeLanguage(newLang);
      }
      setAntdLocale(newLang === 'zh-CN' ? zhCN : enUS);
    });

    invoke('show_main_window').catch(console.error);

    return unsubscribe;
  }, [i18n]);

  useEffect(() => {
    initSerialEventListeners().catch(console.error);
    return () => {
      cleanupSerialEventListeners().catch(console.error);
    };
  }, []);

  return (
    <ConfigProvider
      locale={antdLocale}
      theme={{
        algorithm: isDark ? darkAlgorithm : defaultAlgorithm,
        token: {
          colorPrimary: '#1890ff',
        },
      }}
    >
      <BrowserRouter>
        <MainLayout>
          <Suspense fallback={<PageLoader />}>
            <Routes>
              <Route path="/" element={<Navigate to="/serial" replace />} />
              <Route path="/serial" element={<SerialPage />} />
              <Route path="/ble" element={<BlePage />} />
              <Route path="/protocol" element={<ProtocolPage />} />
              <Route path="/waveform" element={<WaveformPage />} />
              <Route path="/system" element={<SystemPage />} />
            </Routes>
          </Suspense>
        </MainLayout>
      </BrowserRouter>
    </ConfigProvider>
  );
}

export default App;
