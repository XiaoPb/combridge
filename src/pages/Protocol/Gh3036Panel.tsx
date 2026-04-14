import React, { useEffect, useState } from 'react';
import { Collapse, Alert, Spin } from 'antd';
import { listen } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { SettingOutlined, CodeOutlined } from '@ant-design/icons';
import { useGh3036Store } from '../../stores/gh3036Store';
import Gh3036RpcList from './Gh3036RpcList';
import Gh3036ChannelConfig from './Gh3036ChannelConfig';
import Gh3036DataView from './Gh3036DataView';
import type { Gh3036FrameData } from '../../api/types';

interface EventBusEvent {
  topic: string;
  payload: string;
  timestamp: number;
}

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
    addFrameData,
  } = useGh3036Store();

  const [activeKeys, setActiveKeys] = useState<string[]>(['channel', 'commands']);

  useEffect(() => {
    if (!isInitialized) {
      initialize();
    }
    loadChannels();
    loadCsvConfig();
    loadRpcCommands();
  }, [isInitialized, initialize, loadChannels, loadCsvConfig, loadRpcCommands]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen<EventBusEvent>('event-bus', (event) => {
        if (event.payload.topic === 'gh3036:frame') {
          try {
            const frameData = JSON.parse(event.payload.payload) as Gh3036FrameData;
            addFrameData(frameData);
          } catch (err) {
            console.error('[Gh3036Panel] Failed to parse gh3036:frame payload:', err);
          }
        }
      });
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [addFrameData]);

  if (isLoading && !isInitialized) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}>
        <Spin tip={t('gh3036.initializing')} />
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
