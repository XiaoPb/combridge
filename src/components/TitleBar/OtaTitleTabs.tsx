import React, { useEffect, useMemo } from 'react';
import { CloudUploadOutlined, HistoryOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { useMenuVisibilityStore } from '../../stores/menuVisibilityStore';

const OtaTitleTabs: React.FC = () => {
  const { otaActiveTab, setOtaActiveTab } = usePageTabsStore();
  const { menuVisibility } = useMenuVisibilityStore();
  const { t } = useTranslation('ota');

  const tabs = useMemo(
    () => [
      { key: 'upgrade' as const, label: t('tabs.upgrade'), icon: <CloudUploadOutlined /> },
      { key: 'history' as const, label: t('tabs.history'), icon: <HistoryOutlined /> },
    ],
    [t],
  );

  const visibleTabs = useMemo(
    () => tabs.filter((tab) => menuVisibility.home.ota.tabs[tab.key]),
    [tabs, menuVisibility.home.ota.tabs],
  );

  useEffect(() => {
    if (visibleTabs.length > 0 && !visibleTabs.some((tab) => tab.key === otaActiveTab)) {
      setOtaActiveTab(visibleTabs[0].key);
    }
  }, [otaActiveTab, setOtaActiveTab, visibleTabs]);

  return (
    <div className="title-tabs-container">
      {visibleTabs.map((tab) => {
        const isActive = tab.key === otaActiveTab;
        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => setOtaActiveTab(tab.key)}
          >
            {tab.icon}
            <span>{tab.label}</span>
          </div>
        );
      })}
    </div>
  );
};

export default OtaTitleTabs;
