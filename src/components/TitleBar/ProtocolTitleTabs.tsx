import React, { useState } from 'react';
import { CodeOutlined, FolderOpenOutlined } from '@ant-design/icons';

interface ProtocolTitleTabsProps {
  onTabChange?: (tab: 'editor' | 'bind') => void;
}

const ProtocolTitleTabs: React.FC<ProtocolTitleTabsProps> = ({ onTabChange }) => {
  const [activeTab, setActiveTab] = useState<'editor' | 'bind'>('editor');

  const tabs = [
    { key: 'editor', label: '脚本编辑', icon: <CodeOutlined /> },
    { key: 'bind', label: '绑定配置', icon: <FolderOpenOutlined /> },
  ];

  const handleTabClick = (key: 'editor' | 'bind') => {
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
            onClick={() => handleTabClick(tab.key as 'editor' | 'bind')}
          >
            {tab.icon}
            <span>{tab.label}</span>
          </div>
        );
      })}
    </div>
  );
};

export default ProtocolTitleTabs;
