import React, { useEffect, useState } from 'react';
import { Tabs, Alert, Spin } from 'antd';
import { SettingOutlined, LineChartOutlined, InfoCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import ConfigTab from './ConfigTab';
import MonitorTab from './MonitorTab';
import VersionTab from './VersionTab';

const Gh3036Page: React.FC = () => {
  const { t } = useTranslation('gh3036');
  const {
    isInitialized,
    isLoading,
    error,
    initialize,
    loadChannels,
    loadCsvConfig,
    loadRpcCommands,
    subscribeEvents,
    unsubscribeEvents,
    loadLibraryStatus,
  } = useGh3036Store();

  const [activeTab, setActiveTab] = useState('monitor');

  useEffect(() => {
    loadLibraryStatus();
  }, [loadLibraryStatus]);

  useEffect(() => {
    if (!isInitialized) {
      initialize();
    }
    loadChannels();
    loadCsvConfig();
    loadRpcCommands();
    subscribeEvents();

    return () => {
      unsubscribeEvents();
    };
  }, [isInitialized, initialize, loadChannels, loadCsvConfig, loadRpcCommands, subscribeEvents, unsubscribeEvents]);

  if (isLoading && !isInitialized) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}>
        <Spin tip={t('initializing')} />
      </div>
    );
  }

  const tabItems = [
    {
      key: 'config',
      label: (
        <span>
          <SettingOutlined />
          <span style={{ marginLeft: 4 }}>{t('tabs.config')}</span>
        </span>
      ),
      children: <ConfigTab />,
    },
    {
      key: 'monitor',
      label: (
        <span>
          <LineChartOutlined />
          <span style={{ marginLeft: 4 }}>{t('tabs.monitor')}</span>
        </span>
      ),
      children: <MonitorTab />,
    },
    {
      key: 'version',
      label: (
        <span>
          <InfoCircleOutlined />
          <span style={{ marginLeft: 4 }}>{t('tabs.version')}</span>
        </span>
      ),
      children: <VersionTab />,
    },
  ];

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {error && (
        <Alert
          message={t('common:error')}
          description={error}
          type="error"
          closable
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}
      
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={tabItems}
        style={{ flex: 1, overflow: 'hidden' }}
        className="gh3036-tabs"
      />
    </div>
  );
};

export default Gh3036Page;
