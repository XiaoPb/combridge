import React from 'react';
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
  const { activeTabs, toggleTab } = useDashboardStore();

  const tabs: { key: TabType; label: string }[] = [
    { key: 'dashboard', label: t('tabs.dashboard') || '仪表盘' },
    { key: 'console', label: t('tabs.console') || '控制台' },
    { key: 'settings', label: t('tabs.settings') || '设置' },
    { key: 'jsonEditor', label: t('tabs.jsonEditor') || 'JSON编辑器' },
  ];

  const isJsonEditorActive = activeTabs.includes('jsonEditor');

  return (
    <div className="title-tabs-container" style={{ gap: 4 }}>
      {tabs.map((tab) => {
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
