import React, { useState, useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useLocation } from 'react-router-dom';
import {
  MinusOutlined,
  BorderOutlined,
  CloseOutlined,
  BlockOutlined,
  MoonOutlined,
  SunOutlined,
} from '@ant-design/icons';
import { useConfigStore } from '../../stores/configStore';
import SerialTitleTabs from './SerialTitleTabs';
import BleTitleTabs from './BleTitleTabs';
import ProtocolTitleTabs from './ProtocolTitleTabs';
import SystemTitleTabs from './SystemTitleTabs';
import WaveformTitleTabs from './WaveformTitleTabs';
import { useTranslation } from 'react-i18next';
import { changeLanguage } from '../../i18n';

interface ThemeSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
}

const ThemeSwitch: React.FC<ThemeSwitchProps> = ({ checked, onChange }) => (
  <div
    onClick={() => onChange(!checked)}
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
);

const TitleBar: React.FC = () => {
  const location = useLocation();
  const win = getCurrentWindow();
  const [isDarkMode, setIsDarkMode] = useState(false);
  const [language, setLanguage] = useState<'zh-CN' | 'en-US'>('zh-CN');
  const [isMaximized, setIsMaximized] = useState(false);
  useTranslation();

  useEffect(() => {
    const config = useConfigStore.getState().getConfig();
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const isDark = config.theme === 'dark' || (config.theme === 'system' && prefersDark);
    setIsDarkMode(isDark);
    setLanguage(config.language);

    const unsubscribe = useConfigStore.subscribe((state) => {
      const newConfig = state.settings;
      const newPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      const newIsDark = newConfig.theme === 'dark' || (newConfig.theme === 'system' && newPrefersDark);
      setIsDarkMode(newIsDark);
      setLanguage(newConfig.language);
    });

    win.isMaximized().then(setIsMaximized);

    return unsubscribe;
  }, [win]);

  const handleMinimize = async () => await win.minimize();
  const handleMaximize = async () => {
    await win.toggleMaximize();
    setIsMaximized(await win.isMaximized());
  };
  const handleClose = async () => await win.close();

  const toggleTheme = (checked: boolean) => {
    const newTheme = checked ? 'dark' : 'light';
    useConfigStore.getState().updateConfig({ theme: newTheme });
    setIsDarkMode(checked);
    document.documentElement.setAttribute('data-theme', newTheme);
  };

  const toggleLanguage = () => {
    const newLanguage = language === 'zh-CN' ? 'en-US' : 'zh-CN';
    changeLanguage(newLanguage);
    setLanguage(newLanguage);
  };

  const renderSubTabs = () => {
    switch (location.pathname) {
      case '/serial':
        return <SerialTitleTabs />;
      case '/ble':
        return <BleTitleTabs />;
      case '/protocol':
        return <ProtocolTitleTabs />;
      case '/system':
        return <SystemTitleTabs />;
      case '/waveform':
        return <WaveformTitleTabs />;
      default:
        return null;
    }
  };

  return (
    <div className="title-bar" data-tauri-drag-region>
      <div className="title-bar-left" data-tauri-drag-region>
        <span className="app-logo" data-tauri-drag-region>
          <img src="/icon.png" alt="ComBridge" />
        </span>
        <span className="app-name" data-tauri-drag-region>ComBridge</span>
      </div>

      <div className="title-bar-tabs" data-tauri-drag-region>
        {renderSubTabs()}
      </div>

      <div className="title-bar-right">
        <ThemeSwitch checked={isDarkMode} onChange={toggleTheme} />
        <div className="lang-btn" onClick={toggleLanguage}>
          <img src="/languages.svg" alt="Language" />
        </div>
        <div className="window-controls">
          <button onClick={handleMinimize}><MinusOutlined /></button>
          <button onClick={handleMaximize}>
            {isMaximized ? <BlockOutlined /> : <BorderOutlined />}
          </button>
          <button className="close-btn" onClick={handleClose}><CloseOutlined /></button>
        </div>
      </div>
    </div>
  );
};

export default TitleBar;
