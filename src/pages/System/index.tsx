import React, { useState } from 'react';
import { Tabs, Card } from 'antd';
import { SettingOutlined, InfoCircleOutlined, FileTextOutlined, LinkOutlined } from '@ant-design/icons';
import SystemInfo from './SystemInfo';
import LogViewer from './LogViewer';
import WebSocketConfig from './WebSocketConfig';

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
      children: <SystemInfo />,
    },
    {
      key: 'logs',
      label: (
        <span>
          <FileTextOutlined />
          日志查看
        </span>
      ),
      children: <LogViewer />,
    },
    {
      key: 'websocket',
      label: (
        <span>
          <LinkOutlined />
          WebSocket
        </span>
      ),
      children: <WebSocketConfig />,
    },
    {
      key: 'settings',
      label: (
        <span>
          <SettingOutlined />
          系统设置
        </span>
      ),
      children: (
        <Card>
          <div style={{ textAlign: 'center', padding: '40px 0' }}>
            <SettingOutlined style={{ fontSize: '48px', color: 'var(--primary-color)' }} />
            <p style={{ marginTop: 16, color: 'var(--text-secondary)' }}>
              系统设置功能开发中...
            </p>
          </div>
        </Card>
      ),
    },
  ];

  return (
    <div style={{ padding: 0 }}>
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={tabItems}
        size="large"
        style={{ marginBottom: 0 }}
      />
    </div>
  );
};

export default SystemPage;
