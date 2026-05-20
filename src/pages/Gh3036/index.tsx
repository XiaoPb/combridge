import React, { useEffect, useRef } from 'react';
import { Alert, Spin } from 'antd';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import ConfigTab from './ConfigTab';
import FactoryTestTab from './FactoryTestTab';
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
  const subscribedRef = useRef(false);

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

    if (!subscribedRef.current) {
      subscribedRef.current = true;
      subscribeEvents();
    }

    return () => {
      if (subscribedRef.current) {
        subscribedRef.current = false;
        unsubscribeEvents();
      }
    };
  }, []);

  if (isLoading && !isInitialized) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}>
        <Spin><div>{t('initializing')}</div></Spin>
      </div>
    );
  }

  const renderContent = () => {
    switch (gh3036ActiveTab) {
      case 'config':
        return <ConfigTab />;
      case 'factory':
        return <FactoryTestTab />;
      case 'monitor':
        return <MonitorTab />;
      case 'version':
        return <VersionTab />;
      default:
        return <FactoryTestTab />;
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
