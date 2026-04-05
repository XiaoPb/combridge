import React, { useState } from 'react';
import { InfoCircleOutlined, FileTextOutlined, SettingOutlined } from '@ant-design/icons';

interface SystemTitleTabsProps {
  onTabChange?: (tab: 'info' | 'logs' | 'settings') => void;
}

const SystemTitleTabs: React.FC<SystemTitleTabsProps> = ({ onTabChange }) => {
  const [activeTab, setActiveTab] = useState<'info' | 'logs' | 'settings'>('info');

  const tabs = [
    { key: 'info', label: '系统信息', icon: <InfoCircleOutlined /> },
    { key: 'logs', label: '日志查看', icon: <FileTextOutlined /> },
    { key: 'settings', label: '系统设置', icon: <SettingOutlined /> },
  ];

  const handleTabClick = (key: 'info' | 'logs' | 'settings') => {
    setActiveTab(key);
    onTabChange?.(key);
  };

  return (
    <div className="title-tabs-container">
      {tabs.map((tab) => {
        const isActive = tab.key === activeTab;

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => handleTabClick(tab.key as 'info' | 'logs' | 'settings')}
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
