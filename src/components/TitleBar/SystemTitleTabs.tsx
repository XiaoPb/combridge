import React from 'react';
import { InfoCircleOutlined, FileTextOutlined, SettingOutlined } from '@ant-design/icons';
import { usePageTabsStore } from '../../stores/pageTabsStore';

const SystemTitleTabs: React.FC = () => {
  const { systemActiveTab, setSystemActiveTab } = usePageTabsStore();

  const tabs = [
    { key: 'info', label: '系统信息', icon: <InfoCircleOutlined /> },
    { key: 'logs', label: '日志查看', icon: <FileTextOutlined /> },
    { key: 'settings', label: '系统设置', icon: <SettingOutlined /> },
  ] as const;

  return (
    <div className="title-tabs-container">
      {tabs.map((tab) => {
        const isActive = tab.key === systemActiveTab;

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => setSystemActiveTab(tab.key)}
          >
            {tab.icon}
            <span>{tab.label}</span>
          </div>
        );
      })}
    </div>
  );
};

export default SystemTitleTabs;
