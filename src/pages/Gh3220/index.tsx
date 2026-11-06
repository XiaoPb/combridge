import React, { useEffect, useRef } from 'react';
import { Alert } from 'antd';
import { useTranslation } from 'react-i18next';
import { useGh3220Store } from '../../stores/gh3220Store';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import ConfigTab from './ConfigTab';
import MonitorTab from './MonitorTab';
import VersionTab from './VersionTab';
import DebugTab from './DebugTab';

const Gh3220Page: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const { gh3220ActiveTab } = usePageTabsStore();
  const {
    status,
    error,
    refreshStatus,
    loadPreferences,
    subscribeEvents,
    unsubscribeEvents,
  } = useGh3220Store();
  const subscribedRef = useRef(false);

  useEffect(() => {
    void refreshStatus();
    void loadPreferences();

    if (!subscribedRef.current) {
      subscribedRef.current = true;
      void subscribeEvents();
    }

    return () => {
      if (subscribedRef.current) {
        subscribedRef.current = false;
        unsubscribeEvents();
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const renderContent = () => {
    switch (gh3220ActiveTab) {
      case 'config':
        return <ConfigTab />;
      case 'monitor':
        return <MonitorTab />;
      case 'version':
        return <VersionTab />;
      case 'debug':
        return <DebugTab />;
      default:
        return <MonitorTab />;
    }
  };

  // 未连接时各页签（除配置/调试）给出可感知的提示条，样式与 GH3036 的错误条一致
  const showNotConnected =
    status.state === 'disconnected' &&
    gh3220ActiveTab !== 'config' &&
    gh3220ActiveTab !== 'debug';

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {error && (
        <Alert
          message={t('common.error', '错误')}
          description={error}
          type="error"
          closable
          style={{ marginBottom: 8, flexShrink: 0 }}
          afterClose={() => useGh3220Store.getState().setError(null)}
        />
      )}
      {showNotConnected && (
        <Alert
          message={t('capture.notConnected')}
          type="warning"
          showIcon
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      <div style={{ flex: 1, overflow: 'auto', padding: '8px 8px 0 8px' }}>
        {renderContent()}
      </div>
    </div>
  );
};

export default Gh3220Page;
