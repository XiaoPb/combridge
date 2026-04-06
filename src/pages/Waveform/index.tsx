import React, { useState, useEffect, useCallback } from 'react';
import { Card, Row, Col, Button, Space, InputNumber, Alert, Statistic, Switch, Tabs } from 'antd';
import {
  PlayCircleOutlined,
  PauseCircleOutlined,
  ClearOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useWaveformStore, DEFAULT_PARSER_CONFIG } from '../../stores/waveformStore';
import type { ParserConfig } from '../../api/waveform';
import WaveformChart from './WaveformChart';
import BufferConfigPanel from './BufferConfigPanel';
import ParserConfigPanel from './ParserConfigPanel';
import CsvLoaderTab from './CsvLoaderTab';

const RealtimeDataTab: React.FC = () => {
  const { t } = useTranslation('waveform');
  const [displayRows, setDisplayRows] = useState(500);
  const [refreshInterval, setRefreshInterval] = useState(33);

  const store = useWaveformStore();

  const {
    currentBuffer,
    status,
    data,
    isRunning,
    error,
    clearError,
    configureParser,
    clearBuffer,
    readData,
    refreshBuffers,
    setDisplayRows: setStoreDisplayRows,
    setRefreshInterval: setStoreRefreshInterval,
  } = store;

  const handleParserConfigChange = useCallback(
    async (config: ParserConfig) => {
      if (currentBuffer) {
        await configureParser(currentBuffer, config);
      }
    },
    [currentBuffer, configureParser]
  );

  const handleClearBuffer = async () => {
    if (currentBuffer) {
      await clearBuffer(currentBuffer);
    }
  };

  const toggleRunning = () => {
    if (isRunning) {
      store.stopRefresh();
    } else {
      store.startRefresh();
    }
  };

  useEffect(() => {
    refreshBuffers();
  }, [refreshBuffers]);

  useEffect(() => {
    if (currentBuffer && isRunning) {
      const interval = setInterval(() => {
        readData(currentBuffer);
      }, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [currentBuffer, isRunning, refreshInterval, readData]);

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', padding: 8 }}>
      {error && (
        <Alert
          message={t('common.error')}
          description={error}
          type="error"
          closable
          onClose={clearError}
          style={{ marginBottom: 8 }}
        />
      )}

      <Row gutter={8} style={{ flex: '1 1 0', minHeight: 0 }}>
        <Col span={6} style={{ height: '100%', overflow: 'auto' }}>
          <Space direction="vertical" style={{ width: '100%' }} size="small">
            <BufferConfigPanel />
            {currentBuffer && (
              <ParserConfigPanel
                initialConfig={DEFAULT_PARSER_CONFIG}
                onConfigChange={handleParserConfigChange}
              />
            )}
          </Space>
        </Col>

        <Col span={18} style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
          <Card
            size="small"
            styles={{ body: { padding: 8, display: 'flex', flexDirection: 'column', height: 'calc(100% - 40px)' } }}
            title={
              <Space>
                <span>{t('chart.title')}</span>
                {status && (
                  <Statistic
                    title={t('chart.dataPoints')}
                    value={status.row_count}
                    suffix={`/ ${status.capacity}`}
                    style={{ marginLeft: 16 }}
                  />
                )}
              </Space>
            }
            extra={
              <Space>
                <span>{t('chart.displayRows')}</span>
                <InputNumber
                  min={100}
                  max={5000}
                  value={displayRows}
                  onChange={(v) => {
                    const rows = v || 500;
                    setDisplayRows(rows);
                    setStoreDisplayRows(rows);
                  }}
                  style={{ width: 80 }}
                />
                <span>{t('chart.refreshInterval')} (ms)</span>
                <InputNumber
                  min={10}
                  max={1000}
                  value={refreshInterval}
                  onChange={(v) => {
                    const ms = v || 33;
                    setRefreshInterval(ms);
                    setStoreRefreshInterval(ms);
                  }}
                  style={{ width: 80 }}
                />
                <Switch
                  checkedChildren={t('chart.running')}
                  unCheckedChildren={t('chart.paused')}
                  checked={isRunning}
                  onChange={toggleRunning}
                />
                <Button
                  icon={isRunning ? <PauseCircleOutlined /> : <PlayCircleOutlined />}
                  onClick={toggleRunning}
                >
                  {isRunning ? t('chart.pause') : t('chart.start')}
                </Button>
                <Button icon={<ClearOutlined />} onClick={handleClearBuffer}>
                  {t('chart.clear')}
                </Button>
                <Button
                  icon={<ReloadOutlined />}
                  onClick={() => currentBuffer && readData(currentBuffer)}
                >
                  {t('chart.refresh')}
                </Button>
              </Space>
            }
          >
            <div style={{ flex: 1, minHeight: 0 }}>
              {data ? (
                <WaveformChart
                  columns={data.columns}
                  rows={data.rows}
                  displayRows={displayRows}
                />
              ) : (
                <div
                  style={{
                    height: '100%',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: 'var(--text-secondary)',
                  }}
                >
                  {currentBuffer ? t('chart.noData') : t('chart.selectBuffer')}
                </div>
              )}
            </div>
          </Card>
        </Col>
      </Row>
    </div>
  );
};

const WaveformPage: React.FC = () => {
  const { t } = useTranslation('waveform');

  const items = [
    {
      key: 'realtime',
      label: t('tabs.realtime'),
      children: <RealtimeDataTab />,
    },
    {
      key: 'csvLoader',
      label: t('tabs.csvLoader'),
      children: <CsvLoaderTab />,
    },
  ];

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Tabs
        defaultActiveKey="realtime"
        items={items}
        style={{ height: '100%' }}
        styles={{
          content: { height: 'calc(100% - 46px)', overflow: 'auto' },
        }}
      />
    </div>
  );
};

export default WaveformPage;
