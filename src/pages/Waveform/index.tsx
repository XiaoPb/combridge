import React, { useEffect, useCallback } from 'react';
import { Card, Button, Space, InputNumber, Alert, Statistic, Switch, Layout, Tooltip } from 'antd';
import {
  PlayCircleOutlined,
  PauseCircleOutlined,
  ClearOutlined,
  ReloadOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useWaveformStore, DEFAULT_PARSER_CONFIG } from '../../stores/waveformStore';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import type { ParserConfig } from '../../api/waveform';
import WaveformChart from './WaveformChart';
import BufferConfigPanel from './BufferConfigPanel';
import ParserConfigPanel from './ParserConfigPanel';
import CsvLoaderTab from './CsvLoaderTab';

const { Sider, Content } = Layout;

const RealtimeDataTab: React.FC = () => {
  const { t } = useTranslation('waveform');

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
    displayRows,
    refreshInterval,
    preferences,
    loadPreferences,
    updatePreferences,
    startRefresh,
    stopRefresh,
  } = useWaveformStore();

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
      stopRefresh();
    } else {
      startRefresh();
    }
  };

  useEffect(() => {
    loadPreferences();
    refreshBuffers();
  }, [loadPreferences, refreshBuffers]);

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

      <Layout style={{ flex: '1 1 0', minHeight: 0, background: 'transparent' }}>
        <Sider
          collapsible
          collapsed={preferences.sidebarCollapsed}
          onCollapse={(collapsed) => updatePreferences({ sidebarCollapsed: collapsed })}
          width={280}
          collapsedWidth={0}
          trigger={null}
          style={{
            background: 'var(--bg-secondary)',
            borderRadius: '8px',
            marginRight: preferences.sidebarCollapsed ? 0 : 8,
            overflow: 'hidden',
            transition: 'all 0.2s',
          }}
        >
          <div style={{ padding: 8, height: '100%', overflow: 'auto' }}>
            <Space orientation="vertical" style={{ width: '100%' }} size="small">
              <BufferConfigPanel />
              {currentBuffer && (
                <ParserConfigPanel
                  initialConfig={DEFAULT_PARSER_CONFIG}
                  onConfigChange={handleParserConfigChange}
                />
              )}
            </Space>
          </div>
        </Sider>

        <Content style={{ display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <Card
            size="small"
            styles={{ body: { padding: 8, display: 'flex', flexDirection: 'column', height: 'calc(100% - 40px)' } }}
            title={
              <Space>
                <Tooltip title={preferences.sidebarCollapsed ? t('sidebar.expand') : t('sidebar.collapse')}>
                  <Button
                    type="text"
                    icon={preferences.sidebarCollapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
                    onClick={() => updatePreferences({ sidebarCollapsed: !preferences.sidebarCollapsed })}
                    style={{ marginRight: 4 }}
                  />
                </Tooltip>
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
                    updatePreferences({ displayRows: rows });
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
                    updatePreferences({ refreshInterval: ms });
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
        </Content>
      </Layout>
    </div>
  );
};

const WaveformPage: React.FC = () => {
  const { waveformActiveTab } = usePageTabsStore();

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {waveformActiveTab === 'realtime' ? <RealtimeDataTab /> : <CsvLoaderTab />}
    </div>
  );
};

export default WaveformPage;
