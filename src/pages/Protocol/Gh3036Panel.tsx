import React, { useEffect } from 'react';
import { Card, Row, Col, Alert, Spin } from 'antd';
import { listen } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import Gh3036RpcList from './Gh3036RpcList';
import Gh3036ChannelConfig from './Gh3036ChannelConfig';
import Gh3036DataView from './Gh3036DataView';
import type { Gh3036FrameData } from '../../api/types';

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
      unlisten = await listen<Gh3036FrameData>('gh3036-frame', (event) => {
        addFrameData(event.payload);
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

      <Row gutter={8} style={{ flex: '1 1 0', minHeight: 0 }}>
        <Col span={6} style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
          <Card
            size="small"
            title={t('gh3036.channelConfig')}
            style={{ flex: '0 0 auto', marginBottom: 8 }}
            styles={{ body: { padding: 8 } }}
          >
            <Gh3036ChannelConfig />
          </Card>
          <Card
            size="small"
            title={t('gh3036.rpcCommands')}
            style={{ flex: '1 1 0', overflow: 'hidden' }}
            styles={{ body: { padding: 8, height: 'calc(100% - 40px)', overflow: 'auto' } }}
          >
            <Gh3036RpcList />
          </Card>
        </Col>

        <Col span={18} style={{ height: '100%' }}>
          <Card
            size="small"
            title={t('gh3036.dataView')}
            style={{ height: '100%' }}
            styles={{ body: { padding: 8, height: 'calc(100% - 40px)', overflow: 'auto' } }}
          >
            <Gh3036DataView />
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default Gh3036Panel;
