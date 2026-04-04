import React from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu } from 'antd';
import {
  UsbOutlined,
  ApiOutlined,
  CodeOutlined,
  SettingOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from '@ant-design/icons';

const { Sider } = Layout;

interface SidebarProps {
  collapsed: boolean;
  onCollapse?: (collapsed: boolean) => void;
}

const Sidebar: React.FC<SidebarProps> = ({ collapsed, onCollapse }) => {
  const navigate = useNavigate();
  const location = useLocation();

  const menuItems = [
    {
      key: '/serial',
      icon: <UsbOutlined />,
      label: '串口',
    },
    {
      key: '/ble',
      icon: <ApiOutlined />,
      label: 'BLE',
    },
    {
      key: '/protocol',
      icon: <CodeOutlined />,
      label: '协议',
    },
    {
      key: '/system',
      icon: <SettingOutlined />,
      label: '系统',
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
          height: 'var(--header-height)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: collapsed ? '0 16px' : '0 16px 0 24px',
          borderBottom: '1px solid var(--border-color)',
        }}
      >
        <h1
          style={{
            margin: 0,
            fontSize: collapsed ? '16px' : '18px',
            fontWeight: 600,
            color: 'var(--primary-color)',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
          }}
        >
          {collapsed ? 'CB' : 'ComBridge'}
        </h1>
        <div
          onClick={toggleCollapsed}
          style={{
            cursor: 'pointer',
            fontSize: '16px',
            color: 'var(--text-secondary)',
            display: 'flex',
            alignItems: 'center',
          }}
        >
          {collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
        </div>
      </div>
      <Menu
        mode="inline"
        selectedKeys={[location.pathname]}
        items={menuItems}
        onClick={handleMenuClick}
        style={{
          borderRight: 0,
          background: 'transparent',
        }}
      />
    </Sider>
  );
};

export default Sidebar;
