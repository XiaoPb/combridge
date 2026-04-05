import React from 'react';
import { InfoCircleOutlined, FileTextOutlined, SettingOutlined } from '@ant-design/icons';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { useTranslation } from 'react-i18next';

const SystemTitleTabs: React.FC = () => {
  const { systemActiveTab, setSystemActiveTab } = usePageTabsStore();
  const { t } = useTranslation('system');

  const tabs = [
    { key: 'info', label: t('tab.info'), icon: <InfoCircleOutlined /> },
    { key: 'logs', label: t('tab.logs'), icon: <FileTextOutlined /> },
    { key: 'settings', label: t('tab.settings'), icon: <SettingOutlined /> },
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
