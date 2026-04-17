import React, { useEffect } from 'react';
import { Alert, Spin } from 'antd';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { usePageTabsStore } from '../../stores/pageTabsStore';
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

  const { gh3036ActiveTab } = usePageTabsStore();

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

  const renderContent = () => {
    switch (gh3036ActiveTab) {
      case 'config':
        return <ConfigTab />;
      case 'monitor':
        return <MonitorTab />;
      case 'version':
        return <VersionTab />;
      default:
        return <MonitorTab />;
    }
  };

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
      
      <div style={{ flex: 1, overflow: 'auto' }}>
        {renderContent()}
      </div>
    </div>
  );
};

export default Gh3036Page;
