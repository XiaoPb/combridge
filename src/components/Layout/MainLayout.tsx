import { useState } from 'react';
import { Layout } from 'antd';
import Sidebar from './Sidebar';
import TitleBar from '../TitleBar';

const { Content } = Layout;

interface MainLayoutProps {
  children: React.ReactNode;
}

const MainLayout: React.FC<MainLayoutProps> = ({ children }) => {
  const [collapsed, setCollapsed] = useState(true);

  return (
    <Layout style={{ height: '100vh' }}>
      <TitleBar />
      <Layout style={{ flex: 1 }}>
        <Sidebar collapsed={collapsed} onCollapse={setCollapsed} />
        <Layout style={{ flex: 1 }}>
          <Content
            style={{
              margin: '8px',
              padding: '8px',
              background: 'var(--bg-primary)',
              borderRadius: 'var(--border-radius)',
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column',
            }}
          >
            {children}
          </Content>
        </Layout>
      </Layout>
    </Layout>
  );
};

export default MainLayout;
