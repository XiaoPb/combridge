import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
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
  const [loadError, setLoadError] = useState<Error | null>(null);

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

  const handleLoadTimeout = useCallback(() => {
    console.error('Page loading timeout after 30 seconds');
  }, []);

  const handleLoadError = useCallback((error: Error) => {
    console.error('Page loading error:', error);
    setLoadError(error);
  }, []);

  if (loadError) {
    return (
      <div style={{ padding: 24 }}>
        <Result
          status="error"
          title="加载失败"
          subTitle="页面加载失败，请刷新页面重试"
          extra={[
            <Button key="reload" type="primary" onClick={() => window.location.reload()}>
              刷新页面
            </Button>,
          ]}
        />
      </div>
    );
  }

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
                <Route path="/" element={<Navigate to="/serial" replace />} />
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
