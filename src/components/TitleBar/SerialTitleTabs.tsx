import React from 'react';
import { Tag } from 'antd';
import { useSerialStore } from '../../stores/serialStore';

const SerialTitleTabs: React.FC = () => {
  const { tabs, activeTabKey, setActiveTab } = useSerialStore();

  return (
    <div className="title-tabs-container">
      {tabs.map((tab) => {
        const isActive = tab.key === activeTabKey;
        const label = tab.tabType === 'launcher' ? '启动台' : tab.portName;

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => setActiveTab(tab.key)}
          >
            <span>{label}</span>
            {tab.isConnected && tab.tabType === 'port' && (
              <Tag color="success" style={{ marginLeft: 4, fontSize: 10, padding: '0 4px' }}>
                ●
              </Tag>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default SerialTitleTabs;
