import React from 'react';
import { Tag } from 'antd';
import { CloseOutlined } from '@ant-design/icons';
import { useSerial } from '../../hooks/useSerial';
import { useTranslation } from 'react-i18next';

const SerialTitleTabs: React.FC = () => {
  const { tabs, activeTabKey, setActiveTab, removeTab, closePort } = useSerial();
  const { t } = useTranslation('serial');

  const handleClose = async (e: React.MouseEvent, tabKey: string, isConnected: boolean) => {
    e.stopPropagation();
    if (isConnected) {
      try {
        await closePort(tabKey);
      } catch {
        // ignore close errors
      }
    }
    removeTab(tabKey);
  };

  return (
    <div className="title-tabs-container">
      {tabs.map((tab) => {
        const isActive = tab.key === activeTabKey;
        const label = tab.tabType === 'launcher' ? t('tab.launcher') : tab.portName;
        const isClosable = tab.tabType === 'port';

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
            {isClosable && (
              <span
                className="tab-close-btn"
                onClick={(e) => handleClose(e, tab.key, tab.isConnected)}
              >
                <CloseOutlined style={{ fontSize: 10 }} />
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default SerialTitleTabs;
