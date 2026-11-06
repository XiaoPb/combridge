import React, { useEffect, useMemo } from 'react';
import {
  SettingOutlined,
  LineChartOutlined,
  InfoCircleOutlined,
  BugOutlined,
} from '@ant-design/icons';
import { usePageTabsStore, type Gh3220TabKey } from '../../stores/pageTabsStore';
import { useTranslation } from 'react-i18next';
import { useMenuVisibilityStore } from '../../stores/menuVisibilityStore';

export type { Gh3220TabKey };

const Gh3220TitleTabs: React.FC = () => {
  const { gh3220ActiveTab, setGh3220ActiveTab } = usePageTabsStore();
  const { menuVisibility } = useMenuVisibilityStore();
  const { t } = useTranslation('gh3220');

  // 4 个页签：与 GH3036 的处理方式一致，调试类功能统一收纳到 debug
  const tabs: Array<{
    key: Gh3220TabKey;
    label: string;
    icon: React.ReactNode;
  }> = useMemo(
    () => [
      { key: 'config', label: t('tabs.config'), icon: <SettingOutlined /> },
      { key: 'monitor', label: t('tabs.monitor'), icon: <LineChartOutlined /> },
      { key: 'version', label: t('tabs.version'), icon: <InfoCircleOutlined /> },
      { key: 'debug', label: t('tabs.debug'), icon: <BugOutlined /> },
    ],
    [t]
  );

  const visibleTabs = useMemo(
    () => tabs.filter((tab) => menuVisibility.home.gh3220.tabs[tab.key]),
    [tabs, menuVisibility]
  );

  useEffect(() => {
    if (
      visibleTabs.length > 0 &&
      !visibleTabs.some((tab) => tab.key === gh3220ActiveTab)
    ) {
      setGh3220ActiveTab(visibleTabs[0].key);
    }
  }, [gh3220ActiveTab, setGh3220ActiveTab, visibleTabs]);

  return (
    <div className="title-tabs-container">
      {visibleTabs.map((tab) => {
        const isActive = tab.key === gh3220ActiveTab;

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => setGh3220ActiveTab(tab.key)}
          >
            {tab.icon}
            <span>{tab.label}</span>
          </div>
        );
      })}
    </div>
  );
};

export default Gh3220TitleTabs;
