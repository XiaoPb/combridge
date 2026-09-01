import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Card, Button, Switch, InputNumber, Space, Alert, Typography, Spin, Select } from 'antd';
import { FileOutlined, FolderOpenOutlined, DownOutlined, RightOutlined, PlusOutlined, DeleteOutlined, DownloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useCsvChartStore } from '../../stores/csvChartStore';
import MultiLineChart from './MultiLineChart';
import type { MultiLineChartHandle } from './MultiLineChart';

const { Text } = Typography;

const CsvLoaderTab: React.FC = () => {
  const { t } = useTranslation('waveform');
  const [configCollapsed, setConfigCollapsed] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const chartRef = useRef<MultiLineChartHandle>(null);

  const {
    csvData,
    filePath,
    parseConfig,
    chartGroups,
    sampleRate,
    dataZoomState,
    isLoading,
    error,
    loadCsvFile,
    setParseConfig,
    setSampleRate,
    setDataZoomState,
    clearError,
    addChartGroup,
    removeChartGroup,
    updateChartGroup,
  } = useCsvChartStore();

  const handleSelectFile = useCallback(async () => {
    if (isLoading || isExporting) return;
    setExportError(null);
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
  }, [isExporting, isLoading, loadCsvFile]);

  const handleReloadFile = useCallback(async () => {
    if (filePath && !isLoading && !isExporting) {
      setExportError(null);
      await loadCsvFile(filePath, { resetZoom: true });
    }
  }, [filePath, isExporting, isLoading, loadCsvFile]);

  const handleAddChartGroup = useCallback(() => {
    addChartGroup();
  }, [addChartGroup]);

  const handleExportError = useCallback((exportFailure: Error) => {
    setExportError(exportFailure.message || t('csvLoader.exportAllPngError'));
  }, [t]);

  const handleClearError = useCallback(() => {
    clearError();
    setExportError(null);
  }, [clearError]);

  const handleExportAllPng = useCallback(async () => {
    if (!chartRef.current || isExporting || isLoading) return;

    setIsExporting(true);
    setExportError(null);
    try {
      await chartRef.current.exportAllPng();
    } catch (err) {
      setExportError(
        err instanceof Error && err.message
          ? err.message
          : t('csvLoader.exportAllPngError'),
      );
    } finally {
      setIsExporting(false);
    }
  }, [isExporting, isLoading, t]);

  const columns = csvData?.columns ?? [];
  const rows = csvData?.rows ?? [];
  const visibleError = exportError ?? error;

  useEffect(() => {
    setExportError(null);
  }, [csvData, filePath]);

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', padding: 8, overflow: 'hidden' }}>
      {visibleError && (
        <Alert
          message={exportError ? t('csvLoader.exportFailed') : t('common.error')}
          description={visibleError}
          type="error"
          closable
          onClose={handleClearError}
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      <Card
        size="small"
        styles={{ body: { padding: configCollapsed ? 8 : 12 } }}
        style={{ marginBottom: 8, flexShrink: 0 }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <Button
            type="text"
            icon={configCollapsed ? <RightOutlined /> : <DownOutlined />}
            onClick={() => setConfigCollapsed(!configCollapsed)}
            size="small"
          />
          <Space wrap size="middle">
            <Button
              icon={<FolderOpenOutlined />}
              onClick={handleSelectFile}
              loading={isLoading}
              disabled={isLoading || isExporting}
              size="small"
            >
              {t('csvLoader.selectFile')}
            </Button>
            {filePath && (
              <>
                <Button
                  icon={<FileOutlined />}
                  onClick={handleReloadFile}
                  loading={isLoading}
                  disabled={!filePath || isLoading || isExporting}
                  size="small"
                >
                  {t('csvLoader.reloadFile')}
                </Button>
                <Text type="secondary" style={{ maxWidth: 200 }} ellipsis={{ tooltip: filePath }}>
                  {filePath}
                </Text>
              </>
            )}
            {csvData && columns.length > 0 && rows.length > 0 && (
              <Button
                icon={<DownloadOutlined />}
                onClick={handleExportAllPng}
                loading={isExporting}
                disabled={isExporting || isLoading}
                size="small"
              >
                {t('csvLoader.exportAllPng')}
              </Button>
            )}
          </Space>
        </div>

        {!configCollapsed && (
          <>
            <div style={{ marginTop: 12, paddingTop: 12, borderTop: '1px solid var(--border-color)' }}>
              <Space wrap size="middle">
                <Space>
                  <Text>{t('csvLoader.sampleRate')}</Text>
                  <InputNumber
                    min={1}
                    max={10000}
                    value={sampleRate}
                    onChange={(value) => setSampleRate(value ?? 25)}
                    style={{ width: 80 }}
                    size="small"
                  />
                  <Text type="secondary">Hz</Text>
                </Space>

                <Space>
                  <Text>{t('csvLoader.skipInfoRows')}</Text>
                  <InputNumber
                    min={0}
                    max={100}
                    value={parseConfig.skipInfoRows}
                    onChange={(value) => setParseConfig({ skipInfoRows: value ?? 0 })}
                    style={{ width: 70 }}
                    size="small"
                  />
                  <Text type="secondary">{t('csvLoader.rows')}</Text>
                </Space>

                <Space>
                  <Text>{t('csvLoader.noHeader')}</Text>
                  <Switch
                    checked={parseConfig.noHeader}
                    onChange={(checked) => setParseConfig({ noHeader: checked })}
                    size="small"
                  />
                </Space>
              </Space>
            </div>

            {columns.length > 0 && (
              <div style={{ marginTop: 12, paddingTop: 12, borderTop: '1px solid var(--border-color)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
                  {chartGroups.map((group) => (
                    <Space key={group.id} style={{ background: 'var(--bg-secondary)', padding: '4px 8px', borderRadius: 4 }}>
                      <Text strong style={{ fontSize: 12 }}>{group.name}</Text>
                      <Select
                        mode="multiple"
                        allowClear
                        placeholder={t('sidebar.selectColumns')}
                        value={group.columns}
                        onChange={(cols) => updateChartGroup(group.id, { columns: cols })}
                        options={columns.map(col => ({ label: col, value: col }))}
                        maxTagCount="responsive"
                        size="small"
                        style={{ minWidth: 150 }}
                      />
                      <InputNumber
                        min={150}
                        max={600}
                        value={group.height}
                        onChange={(v) => updateChartGroup(group.id, { height: v || 300 })}
                        style={{ width: 60 }}
                        size="small"
                        addonAfter="px"
                      />
                      {chartGroups.length > 1 && (
                        <Button
                          type="text"
                          icon={<DeleteOutlined />}
                          onClick={() => removeChartGroup(group.id)}
                          size="small"
                          danger
                        />
                      )}
                    </Space>
                  ))}
                  <Button
                    type="dashed"
                    icon={<PlusOutlined />}
                    onClick={handleAddChartGroup}
                    size="small"
                  >
                    {t('sidebar.addChartGroup')}
                  </Button>
                </div>
              </div>
            )}
          </>
        )}
      </Card>

      <div style={{ flex: 1, minHeight: 0, position: 'relative' }}>
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
            sampleRate={sampleRate}
            initialDataZoom={dataZoomState}
            onDataZoomChange={setDataZoomState}
            legendScope="csv"
            ref={chartRef}
            onExportError={handleExportError}
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
      </div>
    </div>
  );
};

export default CsvLoaderTab;
