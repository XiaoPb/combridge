import React from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu } from 'antd';
import {
  ExperimentOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';

const { Sider } = Layout;

interface SidebarProps {
  collapsed: boolean;
  onCollapse?: (collapsed: boolean) => void;
}

const Sidebar: React.FC<SidebarProps> = ({ collapsed, onCollapse }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation('sidebar');

  const menuItems = [
    {
      key: '/gh3036',
      icon: <ExperimentOutlined />,
      label: t('menu.gh3036'),
    },
  ];

  const handleMenuClick = ({ key }: { key: string }) => {
    navigate(key);
  };

  const toggleCollapsed = () => {
    onCollapse?.(!collapsed);
  };

  return (
    <Sider
      collapsible
      collapsed={collapsed}
      trigger={null}
      width={260}
      style={{
        background: 'var(--bg-secondary)',
        borderRight: '1px solid var(--border-color)',
      }}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          height: '100%',
        }}
      >
        <Menu
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={handleMenuClick}
          style={{
            borderRight: 0,
            background: 'transparent',
            flex: 1,
            overflow: 'auto',
          }}
        />
        <div
          onClick={toggleCollapsed}
          style={{
            cursor: 'pointer',
            fontSize: '14px',
            color: 'var(--text-secondary)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '40px',
            transition: 'background-color 0.2s',
            gap: '8px',
            borderTop: '1px solid var(--border-color)',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = 'var(--hover-bg)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = 'transparent';
          }}
        >
          {collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
          {!collapsed && <span>{t('collapse')}</span>}
        </div>
      </div>
    </Sider>
  );
};

export default Sidebar;
