import React from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu } from 'antd';
import {
  HomeOutlined,
  UsbOutlined,
  ApiOutlined,
  CodeOutlined,
  SettingOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  LineChartOutlined,
  DashboardOutlined,
  HeartOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useMenuVisibilityStore, type SidebarMenuKey } from '../../stores/menuVisibilityStore';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { useDashboardStore } from '../../stores/dashboardStore';

const { Sider } = Layout;

interface SidebarMenuItem {
  key: string;
  visibilityKey: SidebarMenuKey;
  icon: React.ReactNode;
  label: string;
}

interface SidebarProps {
  collapsed: boolean;
  onCollapse?: (collapsed: boolean) => void;
}

const Sidebar: React.FC<SidebarProps> = ({ collapsed, onCollapse }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation('sidebar');
  const { menuVisibility } = useMenuVisibilityStore();
  const { setWaveformActiveTab, setProtocolActiveTab, setSystemActiveTab, setGh3036ActiveTab } = usePageTabsStore();
  const { setActiveTabs } = useDashboardStore();

  const allMenuItems: SidebarMenuItem[] = [
    {
      key: '/',
      visibilityKey: 'home',
      icon: <HomeOutlined />,
      label: t('menu.home'),
    },
    {
      key: '/serial',
      visibilityKey: 'serial',
      icon: <UsbOutlined />,
      label: t('menu.serial'),
    },
    {
      key: '/ble',
      visibilityKey: 'ble',
      icon: <ApiOutlined />,
      label: t('menu.ble'),
    },
    {
      key: '/dashboard',
      visibilityKey: 'dashboard',
      icon: <DashboardOutlined />,
      label: t('menu.dashboard'),
    },
    {
      key: '/gh3036',
      visibilityKey: 'gh3036',
      icon: <HeartOutlined />,
      label: t('menu.gh3036'),
    },
    {
      key: '/protocol',
      visibilityKey: 'protocol',
      icon: <CodeOutlined />,
      label: t('menu.protocol'),
    },
    {
      key: '/waveform',
      visibilityKey: 'waveform',
      icon: <LineChartOutlined />,
      label: t('menu.waveform'),
    },
    {
      key: '/system',
      visibilityKey: 'system',
      icon: <SettingOutlined />,
      label: t('menu.system'),
    },
  ];

  const menuItems = allMenuItems
    .filter((item) => menuVisibility.sidebar[item.visibilityKey])
    .map(({ visibilityKey: _visibilityKey, ...item }) => item);

  const handleMenuClick = ({ key }: { key: string }) => {
    const firstVisibleTab = (moduleKey: keyof typeof menuVisibility.home) =>
      Object.entries(menuVisibility.home[moduleKey].tabs).find(([, visible]) => visible)?.[0];

    switch (key) {
      case '/dashboard': {
        const tab = firstVisibleTab('dashboard');
        if (tab) {
          setActiveTabs([tab as 'dashboard' | 'console' | 'settings' | 'jsonEditor']);
        }
        break;
      }
      case '/gh3036': {
        const tab = firstVisibleTab('gh3036');
        if (tab) {
          setGh3036ActiveTab(tab as 'config' | 'monitor' | 'version' | 'factory' | 'threshold');
        }
        break;
      }
      case '/protocol': {
        const tab = firstVisibleTab('protocol');
        if (tab) {
          setProtocolActiveTab(tab as 'editor' | 'bind');
        }
        break;
      }
      case '/waveform': {
        const tab = firstVisibleTab('waveform');
        if (tab) {
          setWaveformActiveTab(tab as 'realtime' | 'csvLoader');
        }
        break;
      }
      case '/system': {
        const tab = firstVisibleTab('system');
        if (tab) {
          setSystemActiveTab(tab as 'info' | 'logs' | 'settings');
        }
        break;
      }
    }
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
