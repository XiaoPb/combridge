import React from 'react';
import { SettingOutlined, LineChartOutlined, InfoCircleOutlined } from '@ant-design/icons';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { useTranslation } from 'react-i18next';

const Gh3036TitleTabs: React.FC = () => {
  const { gh3036ActiveTab, setGh3036ActiveTab } = usePageTabsStore();
  const { t } = useTranslation('gh3036');

  const tabs = [
    { key: 'config', label: t('tabs.config'), icon: <SettingOutlined /> },
    { key: 'monitor', label: t('tabs.monitor'), icon: <LineChartOutlined /> },
    { key: 'version', label: t('tabs.version'), icon: <InfoCircleOutlined /> },
  ] as const;

  return (
    <div className="title-tabs-container">
      {tabs.map((tab) => {
        const isActive = tab.key === gh3036ActiveTab;

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => setGh3036ActiveTab(tab.key)}
          >
            {tab.icon}
            <span>{tab.label}</span>
          </div>
        );
      })}
    </div>
  );
};

export default Gh3036TitleTabs;
