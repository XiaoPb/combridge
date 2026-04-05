import React from 'react';
import { CodeOutlined, FolderOpenOutlined } from '@ant-design/icons';
import { usePageTabsStore } from '../../stores/pageTabsStore';

const ProtocolTitleTabs: React.FC = () => {
  const { protocolActiveTab, setProtocolActiveTab } = usePageTabsStore();

  const tabs = [
    { key: 'editor', label: '脚本编辑', icon: <CodeOutlined /> },
    { key: 'bind', label: '绑定配置', icon: <FolderOpenOutlined /> },
  ] as const;

  return (
    <div className="title-tabs-container">
      {tabs.map((tab) => {
        const isActive = tab.key === protocolActiveTab;

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => setProtocolActiveTab(tab.key)}
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
