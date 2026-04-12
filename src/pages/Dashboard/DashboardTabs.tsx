import React from 'react';
import { Space, Checkbox, Tag } from 'antd';
import {
  DashboardOutlined,
  CodeOutlined,
  SettingOutlined,
  CodeSquareOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import type { TabType } from '../../types/dashboard';

const tabIcons: Record<TabType, React.ReactNode> = {
  dashboard: <DashboardOutlined />,
  console: <CodeOutlined />,
  settings: <SettingOutlined />,
  jsonEditor: <CodeSquareOutlined />,
};

const tabColors: Record<TabType, string> = {
  dashboard: '#1890ff',
  console: '#52c41a',
  settings: '#722ed1',
  jsonEditor: '#fa8c16',
};

const DashboardTabs: React.FC = () => {
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
    <div
      style={{
        padding: '8px 16px',
        borderBottom: '1px solid #f0f0f0',
        background: '#fff',
        display: 'flex',
        alignItems: 'center',
      }}
    >
      <Space size={8}>
        {tabs.map((tab) => {
          const isActive = activeTabs.includes(tab.key);
          const isJsonEditor = tab.key === 'jsonEditor';

          if (isJsonEditor) {
            return (
              <Tag
                key={tab.key}
                icon={tabIcons[tab.key]}
                color={isActive ? tabColors[tab.key] : 'default'}
                style={{
                  cursor: 'pointer',
                  padding: '4px 12px',
                  fontSize: 13,
                  borderRadius: 4,
                }}
                onClick={() => toggleTab(tab.key)}
              >
                {tab.label}
              </Tag>
            );
          }

          return (
            <Checkbox
              key={tab.key}
              checked={isActive && !isJsonEditorActive}
              disabled={isJsonEditorActive}
              onChange={() => toggleTab(tab.key)}
            >
              <Space size={4}>
                <span style={{ color: isActive ? tabColors[tab.key] : undefined }}>
                  {tabIcons[tab.key]}
                </span>
                <span>{tab.label}</span>
              </Space>
            </Checkbox>
          );
        })}
      </Space>

      {isJsonEditorActive && (
        <Tag color="warning" style={{ marginLeft: 16 }}>
          JSON编辑模式
        </Tag>
      )}
    </div>
  );
};

export default DashboardTabs;
