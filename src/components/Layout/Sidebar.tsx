import React, { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, Switch, Tooltip } from 'antd';
import {
  UsbOutlined,
  ApiOutlined,
  CodeOutlined,
  SettingOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  MoonOutlined,
  SunOutlined,
} from '@ant-design/icons';
import configService from '../../services/configService';

const { Sider } = Layout;

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
            padding: '8px 0',
          }}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: collapsed ? 'center' : 'flex-start',
              padding: collapsed ? '8px 16px' : '8px 16px 8px 24px',
              gap: '12px',
            }}
          >
            <Tooltip title={isDarkMode ? '切换到浅色模式' : '切换到深色模式'} placement="right">
              <Switch
                checked={isDarkMode}
                onChange={toggleTheme}
                size="small"
                checkedChildren={<MoonOutlined />}
                unCheckedChildren={<SunOutlined />}
              />
            </Tooltip>
            <Tooltip title={language === 'zh-CN' ? 'Switch to English' : '切换到中文'} placement="right">
              <div
                onClick={toggleLanguage}
                style={{
                  cursor: 'pointer',
                  fontSize: '14px',
                  fontWeight: 500,
                  color: 'var(--text-secondary)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  width: '44px',
                  height: '22px',
                  borderRadius: 'var(--border-radius)',
                  transition: 'background-color 0.2s',
                  border: '1px solid var(--border-color)',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.backgroundColor = 'var(--hover-bg)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.backgroundColor = 'transparent';
                }}
              >
                {language === 'zh-CN' ? '文' : 'A'}
              </div>
            </Tooltip>
          </div>
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
