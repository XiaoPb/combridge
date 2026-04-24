import React, { useEffect, useState, useRef } from 'react';
import { Collapse, Alert, Spin } from 'antd';
import { useTranslation } from 'react-i18next';
import { SettingOutlined, CodeOutlined } from '@ant-design/icons';
import { useGh3036Store } from '../../stores/gh3036Store';
import Gh3036RpcList from './Gh3036RpcList';
import Gh3036ChannelConfig from './Gh3036ChannelConfig';
import Gh3036DataView from './Gh3036DataView';

const Gh3036Panel: React.FC = () => {
  const { t } = useTranslation('protocol');
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
  } = useGh3036Store();

  const [activeKeys, setActiveKeys] = useState<string[]>(['channel', 'commands']);
  const subscribedRef = useRef(false);

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
        <Spin><div>{t('gh3036.initializing')}</div></Spin>
      </div>
    );
  }

  const collapseItems = [
    {
      key: 'channel',
      label: (
        <span>
          <SettingOutlined style={{ marginRight: 8 }} />
          {t('gh3036.channelConfig')}
        </span>
      ),
      children: <Gh3036ChannelConfig />,
    },
    {
      key: 'commands',
      label: (
        <span>
          <CodeOutlined style={{ marginRight: 8 }} />
          {t('gh3036.rpcCommands')}
        </span>
      ),
      children: <Gh3036RpcList />,
    },
  ];

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', gap: 8, padding: 8 }}>
      {error && (
        <Alert
          message={t('common:error')}
          description={error}
          type="error"
          closable
          style={{ flexShrink: 0 }}
        />
      )}

      <Collapse
        activeKey={activeKeys}
        onChange={(keys) => setActiveKeys(keys as string[])}
        items={collapseItems}
        size="small"
        style={{ flexShrink: 0 }}
      />

      <div style={{ flex: '1 1 0', minHeight: 0, display: 'flex', flexDirection: 'column' }}>
        <Gh3036DataView />
      </div>
    </div>
  );
};

export default Gh3036Panel;
