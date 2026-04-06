import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConfigProvider, theme } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import enUS from 'antd/locale/en_US';
import { MainLayout } from './components';
import { SerialPage, BlePage, ProtocolPage, SystemPage, WaveformPage } from './pages';
import { useTheme } from './hooks';
import { useTranslation } from 'react-i18next';
import { useEffect, useState } from 'react';
import configService from './services/configService';
import './styles/global.css';

const { defaultAlgorithm, darkAlgorithm } = theme;

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

    return unsubscribe;
  }, [i18n]);

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
          <Routes>
            <Route path="/" element={<Navigate to="/serial" replace />} />
            <Route path="/serial" element={<SerialPage />} />
            <Route path="/ble" element={<BlePage />} />
            <Route path="/protocol" element={<ProtocolPage />} />
            <Route path="/waveform" element={<WaveformPage />} />
            <Route path="/system" element={<SystemPage />} />
          </Routes>
        </MainLayout>
      </BrowserRouter>
    </ConfigProvider>
  );
}

export default App;
