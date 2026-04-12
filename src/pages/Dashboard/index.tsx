import React, { useEffect } from 'react';
import { Layout } from 'antd';
import { useDashboardStore } from '../../stores/dashboardStore';
import { dashboardApi } from '../../api/dashboard';
import DashboardToolbar from './DashboardToolbar';
import DashboardCanvas from './DashboardCanvas';
import DashboardPanel from './DashboardPanel';

const { Content, Sider } = Layout;

const DashboardPage: React.FC = () => {
  const {
    currentDashboard,
    createNewDashboard,
    setParserScripts,
  } = useDashboardStore();

  useEffect(() => {
    const init = async () => {
      try {
        await dashboardApi.initDefaultParserScripts();
        const scripts = await dashboardApi.getParserScripts();
        setParserScripts(scripts);
      } catch (error) {
        console.error('Failed to initialize parser scripts:', error);
      }
    };

    init();

    if (!currentDashboard) {
      createNewDashboard();
    }
  }, []);

  return (
    <Layout style={{ height: '100%', background: 'transparent' }}>
      <DashboardToolbar />
      <Layout>
        <Content style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
          <DashboardCanvas />
        </Content>
        <Sider
          width={320}
          theme="light"
          style={{ borderLeft: '1px solid #f0f0f0' }}
        >
          <DashboardPanel />
        </Sider>
      </Layout>
    </Layout>
  );
};

export default DashboardPage;
