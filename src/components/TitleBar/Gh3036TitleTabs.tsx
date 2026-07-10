import React, { useEffect } from 'react';
import { SettingOutlined, LineChartOutlined, InfoCircleOutlined, ExperimentOutlined, FileProtectOutlined } from '@ant-design/icons';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { useTranslation } from 'react-i18next';
import { useMenuVisibilityStore } from '../../stores/menuVisibilityStore';

const Gh3036TitleTabs: React.FC = () => {
  const { gh3036ActiveTab, setGh3036ActiveTab } = usePageTabsStore();
  const { menuVisibility } = useMenuVisibilityStore();
  const { t } = useTranslation('gh3036');

  const tabs = [
    { key: 'config', label: t('tabs.config'), icon: <SettingOutlined /> },
    { key: 'monitor', label: t('tabs.monitor'), icon: <LineChartOutlined /> },
    { key: 'version', label: t('tabs.version'), icon: <InfoCircleOutlined /> },
    { key: 'factory', label: t('tabs.factory'), icon: <ExperimentOutlined /> },
    { key: 'threshold', label: t('tabs.threshold'), icon: <FileProtectOutlined /> },
  ] as const;

  const visibleTabs = tabs.filter((tab) => menuVisibility.home.gh3036.tabs[tab.key]);

  useEffect(() => {
    if (visibleTabs.length > 0 && !visibleTabs.some((tab) => tab.key === gh3036ActiveTab)) {
      setGh3036ActiveTab(visibleTabs[0].key);
    }
  }, [gh3036ActiveTab, setGh3036ActiveTab, visibleTabs]);

  return (
    <div className="title-tabs-container">
      {visibleTabs.map((tab) => {
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
