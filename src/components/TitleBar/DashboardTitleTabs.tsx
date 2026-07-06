import React, { useEffect } from 'react';
import { Tag } from 'antd';
import {
  DashboardOutlined,
  CodeOutlined,
  SettingOutlined,
  FileTextOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import type { TabType } from '../../types/dashboard';
import { useMenuVisibilityStore } from '../../stores/menuVisibilityStore';

const tabIcons: Record<TabType, React.ReactNode> = {
  dashboard: <DashboardOutlined />,
  console: <CodeOutlined />,
  settings: <SettingOutlined />,
  jsonEditor: <FileTextOutlined />,
};

const tabColors: Record<TabType, string> = {
  dashboard: '#1890ff',
  console: '#52c41a',
  settings: '#722ed1',
  jsonEditor: '#fa8c16',
};

const DashboardTitleTabs: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { activeTabs, setActiveTabs, toggleTab } = useDashboardStore();
  const { menuVisibility } = useMenuVisibilityStore();

  const tabs: { key: TabType; label: string }[] = [
    { key: 'dashboard', label: t('tabs.dashboard') || '仪表盘' },
    { key: 'console', label: t('tabs.console') || '控制台' },
    { key: 'settings', label: t('tabs.settings') || '设置' },
    { key: 'jsonEditor', label: t('tabs.jsonEditor') || 'JSON编辑器' },
  ];

  const visibleTabs = tabs.filter((tab) => menuVisibility.home.dashboard.tabs[tab.key]);

  useEffect(() => {
    const visibleKeys = new Set(visibleTabs.map((tab) => tab.key));
    const nextActiveTabs = activeTabs.filter((tab) => visibleKeys.has(tab));

    if (nextActiveTabs.length !== activeTabs.length) {
      setActiveTabs(nextActiveTabs.length > 0 ? nextActiveTabs : visibleTabs.slice(0, 1).map((tab) => tab.key));
    }
  }, [activeTabs, setActiveTabs, visibleTabs]);

  const isJsonEditorActive = activeTabs.includes('jsonEditor');

  return (
    <div className="title-tabs-container" style={{ gap: 4 }}>
      {visibleTabs.map((tab) => {
        const isActive = activeTabs.includes(tab.key);

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => toggleTab(tab.key)}
            style={{
              opacity: isJsonEditorActive && tab.key !== 'jsonEditor' ? 0.5 : 1,
              cursor: isJsonEditorActive && tab.key !== 'jsonEditor' ? 'not-allowed' : 'pointer',
            }}
          >
            <span style={{ color: isActive ? tabColors[tab.key] : undefined }}>
              {tabIcons[tab.key]}
            </span>
            <span>{tab.label}</span>
            {isActive && tab.key !== 'jsonEditor' && (
              <Tag
                color={tabColors[tab.key]}
                style={{ marginLeft: 4, fontSize: 10, padding: '0 4px', lineHeight: '16px' }}
              >
                ✓
              </Tag>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default DashboardTitleTabs;
