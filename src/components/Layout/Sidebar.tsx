import React, { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, Tooltip } from 'antd';
import {
  UsbOutlined,
  ApiOutlined,
  CodeOutlined,
  SettingOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  MoonOutlined,
  SunOutlined,
  GlobalOutlined,
} from '@ant-design/icons';
import configService from '../../services/configService';

const { Sider } = Layout;

interface ThemeSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  collapsed: boolean;
}

const ThemeSwitch: React.FC<ThemeSwitchProps> = ({ checked, onChange, collapsed }) => {
  return (
    <div
      onClick={() => onChange(!checked)}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: collapsed ? 'center' : 'flex-start',
        padding: collapsed ? '12px 16px' : '12px 16px 12px 24px',
        cursor: 'pointer',
        transition: 'background-color 0.2s',
        gap: '8px',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.backgroundColor = 'var(--hover-bg)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.backgroundColor = 'transparent';
      }}
    >
      <div
        style={{
          position: 'relative',
          width: '44px',
          height: '22px',
          borderRadius: '11px',
          backgroundColor: checked ? 'var(--primary-color)' : 'var(--border-color)',
          transition: 'background-color 0.3s',
          cursor: 'pointer',
        }}
      >
        <div
          style={{
            position: 'absolute',
            top: '2px',
            left: checked ? '24px' : '2px',
            width: '18px',
            height: '18px',
            borderRadius: '50%',
            backgroundColor: '#fff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'left 0.3s',
            boxShadow: '0 2px 4px rgba(0,0,0,0.2)',
          }}
        >
          {checked ? (
            <MoonOutlined style={{ fontSize: '12px', color: 'var(--primary-color)' }} />
          ) : (
            <SunOutlined style={{ fontSize: '12px', color: '#faad14' }} />
          )}
        </div>
      </div>
      {!collapsed && (
        <span style={{ color: 'var(--text-secondary)', fontSize: '14px' }}>
          {checked ? '深色' : '浅色'}
        </span>
      )}
    </div>
  );
};

interface SidebarProps {
  collapsed: boolean;
  onCollapse?: (collapsed: boolean) => void;
}

const Sidebar: React.FC<SidebarProps> = ({ collapsed, onCollapse }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const [isDarkMode, setIsDarkMode] = useState(false);
  const [language, setLanguage] = useState<'zh-CN' | 'en-US'>('zh-CN');

  useEffect(() => {
    const config = configService.getConfig();
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const isDark = config.theme === 'dark' || (config.theme === 'system' && prefersDark);
    setIsDarkMode(isDark);
    setLanguage(config.language);

    const unsubscribe = configService.subscribe((newConfig) => {
      const newPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      const newIsDark = newConfig.theme === 'dark' || (newConfig.theme === 'system' && newPrefersDark);
      setIsDarkMode(newIsDark);
      setLanguage(newConfig.language);
    });

    return unsubscribe;
  }, []);

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

  const toggleTheme = (checked: boolean) => {
    const newTheme = checked ? 'dark' : 'light';
    configService.updateConfig({ theme: newTheme });
    setIsDarkMode(checked);
    document.documentElement.setAttribute('data-theme', newTheme);
  };

  const toggleLanguage = () => {
    const newLanguage = language === 'zh-CN' ? 'en-US' : 'zh-CN';
    configService.updateConfig({ language: newLanguage });
    setLanguage(newLanguage);
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
        <div
          style={{
            height: 'var(--header-height)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: collapsed ? 'center' : 'flex-start',
            padding: collapsed ? '0 16px' : '0 16px 0 24px',
            borderBottom: '1px solid var(--border-color)',
            flexShrink: 0,
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
        </div>
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
          style={{
            borderTop: '1px solid var(--border-color)',
            flexShrink: 0,
          }}
        >
          <Tooltip title={isDarkMode ? '切换到浅色模式' : '切换到深色模式'} placement="right">
            <ThemeSwitch
              checked={isDarkMode}
              onChange={toggleTheme}
              collapsed={collapsed}
            />
          </Tooltip>
          <Tooltip title={language === 'zh-CN' ? 'Switch to English' : '切换到中文'} placement="right">
            <div
              onClick={toggleLanguage}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: collapsed ? 'center' : 'flex-start',
                padding: collapsed ? '12px 16px' : '12px 16px 12px 24px',
                cursor: 'pointer',
                color: 'var(--text-secondary)',
                fontSize: '14px',
                transition: 'background-color 0.2s',
                gap: '8px',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = 'var(--hover-bg)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = 'transparent';
              }}
            >
              <GlobalOutlined style={{ fontSize: '16px' }} />
              {!collapsed && <span>{language === 'zh-CN' ? '中文' : 'English'}</span>}
            </div>
          </Tooltip>
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
            {!collapsed && <span>收起侧边栏</span>}
          </div>
        </div>
      </div>
    </Sider>
  );
};

export default Sidebar;
