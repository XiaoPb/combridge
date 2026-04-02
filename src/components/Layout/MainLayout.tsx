import { useState } from 'react';
import { Layout } from 'antd';
import Sidebar from './Sidebar';
import Header from './Header';
import SettingsModal from '../SettingsModal';

const { Content } = Layout;

interface MainLayoutProps {
  children: React.ReactNode;
}

const MainLayout: React.FC<MainLayoutProps> = ({ children }) => {
  const [collapsed, setCollapsed] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  return (
    <Layout style={{ height: '100vh' }}>
      <Sidebar collapsed={collapsed} />
      <Layout>
        <Header 
          collapsed={collapsed} 
          onCollapse={setCollapsed}
          onSettingsClick={() => setSettingsOpen(true)}
        />
        <Content
          style={{
            margin: '16px',
            padding: '24px',
            background: 'var(--bg-primary)',
            borderRadius: 'var(--border-radius)',
            overflow: 'auto',
          }}
        >
          {children}
        </Content>
      </Layout>
      <SettingsModal 
        open={settingsOpen} 
        onClose={() => setSettingsOpen(false)} 
      />
    </Layout>
  );
};

export default MainLayout;
