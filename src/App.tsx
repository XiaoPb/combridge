import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConfigProvider, theme } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { MainLayout } from './components';
import { SerialPage, BlePage, ProtocolPage, SystemPage } from './pages';
import { useTheme } from './hooks';
import './styles/global.css';

const { defaultAlgorithm, darkAlgorithm } = theme;

function App() {
  const { isDark } = useTheme();

  return (
    <ConfigProvider
      locale={zhCN}
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
            <Route path="/system" element={<SystemPage />} />
          </Routes>
        </MainLayout>
      </BrowserRouter>
    </ConfigProvider>
  );
}

export default App;
