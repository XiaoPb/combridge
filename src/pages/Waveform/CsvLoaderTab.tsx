import React, { useCallback, useState } from 'react';
import { Card, Button, Switch, InputNumber, Space, Alert, Typography, Spin, Layout, Tooltip } from 'antd';
import { FileOutlined, FolderOpenOutlined, MenuFoldOutlined, MenuUnfoldOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useCsvChartStore } from '../../stores/csvChartStore';
import ChartSidebar from './ChartSidebar';
import MultiLineChart from './MultiLineChart';

const { Sider, Content } = Layout;
const { Text } = Typography;

const CsvLoaderTab: React.FC = () => {
  const { t } = useTranslation('waveform');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  const {
    csvData,
    filePath,
    parseConfig,
    chartGroups,
    yAxisConfigs,
    visiblePoints,
    isLoading,
    error,
    loadCsvFile,
    setParseConfig,
    clearError,
  } = useCsvChartStore();

  const handleSelectFile = useCallback(async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (selected && typeof selected === 'string') {
        await loadCsvFile(selected);
      }
    } catch (err) {
      console.error('Failed to open file dialog:', err);
    }
  }, [loadCsvFile]);

  const handleReloadFile = useCallback(async () => {
    if (filePath) {
      await loadCsvFile(filePath);
    }
  }, [filePath, loadCsvFile]);

  const columns = csvData?.columns ?? [];
  const rows = csvData?.rows ?? [];
  const totalRows = rows.length;

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', padding: 8, overflow: 'hidden' }}>
      {error && (
        <Alert
          message={t('common.error')}
          description={error}
          type="error"
          closable
          onClose={clearError}
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      <Card
        size="small"
        title={t('csvLoader.configTitle')}
        styles={{ body: { padding: 12 } }}
        style={{ marginBottom: 8, flexShrink: 0 }}
      >
        <Space wrap size="middle">
          <Button
            icon={<FolderOpenOutlined />}
            onClick={handleSelectFile}
            loading={isLoading}
          >
            {t('csvLoader.selectFile')}
          </Button>
          {filePath && (
            <>
              <Button
                icon={<FileOutlined />}
                onClick={handleReloadFile}
                loading={isLoading}
                disabled={!filePath}
              >
                {t('csvLoader.reloadFile')}
              </Button>
              <Text type="secondary" style={{ maxWidth: 300 }} ellipsis={{ tooltip: filePath }}>
                {filePath}
              </Text>
            </>
          )}
        </Space>

        <Space wrap size="middle" style={{ marginLeft: 24 }}>
          <Space>
            <Text>{t('csvLoader.skipFirstRow')}</Text>
            <Switch
              checked={parseConfig.skipFirstRow}
              onChange={(checked) => setParseConfig({ skipFirstRow: checked })}
            />
          </Space>

          <Space>
            <Text>{t('csvLoader.useSecondRowAsHeader')}</Text>
            <Switch
              checked={parseConfig.useSecondRowAsHeader}
              onChange={(checked) => setParseConfig({ useSecondRowAsHeader: checked })}
            />
          </Space>

          <Space>
            <Text>{t('csvLoader.splitColumn')}</Text>
            <Switch
              checked={parseConfig.splitColumn}
              onChange={(checked) => setParseConfig({ splitColumn: checked })}
            />
          </Space>

          {parseConfig.splitColumn && (
            <Space>
              <Text>{t('csvLoader.splitColumnIndex')}</Text>
              <InputNumber
                min={0}
                value={parseConfig.splitColumnIndex}
                onChange={(value) => setParseConfig({ splitColumnIndex: value ?? 0 })}
                style={{ width: 80 }}
              />
            </Space>
          )}
        </Space>
      </Card>

      <Layout style={{ flex: 1, minHeight: 0, background: 'transparent' }}>
        <Sider
          collapsible
          collapsed={sidebarCollapsed}
          onCollapse={setSidebarCollapsed}
          width={280}
          collapsedWidth={0}
          trigger={null}
          style={{
            background: 'var(--bg-secondary)',
            borderRadius: '8px',
            marginRight: sidebarCollapsed ? 0 : 8,
            overflow: 'hidden',
            transition: 'all 0.2s',
          }}
        >
          <ChartSidebar columns={columns} totalRows={totalRows} />
        </Sider>

        <Content style={{ display: 'flex', flexDirection: 'column', position: 'relative', flex: 1, minWidth: 0 }}>
          <Tooltip title={sidebarCollapsed ? t('sidebar.expand') : t('sidebar.collapse')}>
            <Button
              type="text"
              icon={sidebarCollapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
              onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
              style={{
                position: 'absolute',
                top: 8,
                left: 8,
                zIndex: 10,
                background: 'var(--bg-secondary)',
                borderRadius: 4,
              }}
            />
          </Tooltip>

          {isLoading && (
            <div
              style={{
                position: 'absolute',
                inset: 0,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: 'rgba(255, 255, 255, 0.7)',
                zIndex: 10,
              }}
            >
              <Spin size="large" />
            </div>
          )}

          {csvData && columns.length > 0 ? (
            <MultiLineChart
              columns={columns}
              rows={rows}
              chartGroups={chartGroups}
              yAxisConfigs={yAxisConfigs}
              visiblePoints={visiblePoints}
            />
          ) : (
            <div
              style={{
                height: '100%',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'var(--text-secondary)',
              }}
            >
              <FileOutlined style={{ fontSize: 48, marginBottom: 16 }} />
              <Text type="secondary">{t('csvLoader.noData')}</Text>
            </div>
          )}
        </Content>
      </Layout>
    </div>
  );
};

export default CsvLoaderTab;
