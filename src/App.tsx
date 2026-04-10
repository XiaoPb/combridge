import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { ConfigProvider, theme, Spin, Result, Button } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import enUS from 'antd/locale/en_US';
import { lazy, Suspense, useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { MainLayout, ErrorBoundary } from './components';
import { useTheme } from './hooks';
import configService from './services/configService';
import { initAllEventListeners, cleanupAllEventListeners } from './services/eventListeners';
import './styles/global.css';

const { defaultAlgorithm, darkAlgorithm } = theme;

const HomePage = lazy(() => import('./pages/Home'));
const SerialPage = lazy(() => import('./pages/Serial'));
const BlePage = lazy(() => import('./pages/Ble'));
const ProtocolPage = lazy(() => import('./pages/Protocol'));
const SystemPage = lazy(() => import('./pages/System'));
const WaveformPage = lazy(() => import('./pages/Waveform'));

const LOADING_TIMEOUT_MS = 30000;

interface PageLoaderProps {
  onLoadTimeout?: () => void;
}

function PageLoader({ onLoadTimeout }: PageLoaderProps) {
  const { t } = useTranslation();
  const [isTimeout, setIsTimeout] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => {
      setIsTimeout(true);
      onLoadTimeout?.();
    }, LOADING_TIMEOUT_MS);

    return () => clearTimeout(timer);
  }, [onLoadTimeout]);

  if (isTimeout) {
    return (
      <div style={{ 
        display: 'flex', 
        alignItems: 'center', 
        justifyContent: 'center', 
        height: '100%',
        width: '100%' 
      }}>
        <Result
          status="warning"
          title={t('common:error.loadingTimeout')}
          subTitle={t('common:error.loadingTimeoutDesc')}
          extra={[
            <Button key="reload" type="primary" onClick={() => window.location.reload()}>
              {t('common:error.refreshPage')}
            </Button>,
          ]}
        />
      </div>
    );
  }

  return (
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
}

function AppContent() {
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

    invoke('show_main_window').catch((err) => {
      console.error('Failed to show main window:', err);
    });

    return unsubscribe;
  }, [i18n]);

  useEffect(() => {
    initAllEventListeners().catch((err) => {
      console.error('Failed to initialize event listeners:', err);
    });
    return () => {
      cleanupAllEventListeners().catch((err) => {
        console.error('Failed to cleanup event listeners:', err);
      });
    };
  }, []);

  useEffect(() => {
    const hideSplashScreen = () => {
      const splash = document.getElementById('splash-screen');
      if (splash) {
        splash.classList.add('hidden');
        setTimeout(() => {
          splash.remove();
        }, 300);
      }
    };

    requestAnimationFrame(() => {
      setTimeout(hideSplashScreen, 100);
    });
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'F12' || (event.ctrlKey && event.shiftKey && event.key === 'I')) {
        event.preventDefault();
        invoke('open_devtools').catch((err) => {
          console.error('Failed to open devtools:', err);
        });
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  const handleLoadTimeout = useCallback(() => {
    console.error('Page loading timeout after 30 seconds');
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
          <ErrorBoundary>
            <Suspense fallback={<PageLoader onLoadTimeout={handleLoadTimeout} />}>
              <Routes>
                <Route path="/" element={<HomePage />} />
                <Route path="/serial" element={<SerialPage />} />
                <Route path="/ble" element={<BlePage />} />
                <Route path="/protocol" element={<ProtocolPage />} />
                <Route path="/waveform" element={<WaveformPage />} />
                <Route path="/system" element={<SystemPage />} />
              </Routes>
            </Suspense>
          </ErrorBoundary>
        </MainLayout>
      </BrowserRouter>
    </ConfigProvider>
  );
}

function App() {
  return (
    <ErrorBoundary>
      <AppContent />
    </ErrorBoundary>
  );
}

export default App;
