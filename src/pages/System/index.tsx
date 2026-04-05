import React, { useState } from 'react';
import { Tabs } from 'antd';
import { SettingOutlined, InfoCircleOutlined, FileTextOutlined } from '@ant-design/icons';
import SystemInfo from './SystemInfo';
import LogViewer from './LogViewer';
import SystemSettings from './SystemSettings';

const SystemPage: React.FC = () => {
  const [activeTab, setActiveTab] = useState('info');

  const tabItems = [
    {
      key: 'info',
      label: (
        <span>
          <InfoCircleOutlined />
          系统信息
        </span>
      ),
    },
    {
      key: 'logs',
      label: (
        <span>
          <FileTextOutlined />
          日志查看
        </span>
      ),
    },
    {
      key: 'settings',
      label: (
        <span>
          <SettingOutlined />
          系统设置
        </span>
      ),
    },
  ];

  const renderTabContent = () => {
    switch (activeTab) {
      case 'info':
        return <SystemInfo />;
      case 'logs':
        return <LogViewer />;
      case 'settings':
        return <SystemSettings />;
      default:
        return null;
    }
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <div style={{ flex: '0 0 auto' }}>
        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={tabItems}
          size="small"
          style={{ marginBottom: 0 }}
        />
      </div>
      <div style={{ flex: '1 1 0', minHeight: 0, overflow: 'auto', padding: 8 }}>
        {renderTabContent()}
      </div>
    </div>
  );
};

export default SystemPage;
