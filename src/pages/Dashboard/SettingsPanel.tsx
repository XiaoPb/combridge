import React, { useEffect, useState } from 'react';
import { Card, Select, Button, Divider, message, Space, Empty, theme } from 'antd';
import { DownloadOutlined, FileOutlined, PlayCircleOutlined, PauseCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import { useDashboardStore } from '../../stores/dashboardStore';
import { dashboardApi } from '../../api/dashboard';
import type { DataSourceType } from '../../types/dashboard';

const SettingsPanel: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { token } = theme.useToken();
  const {
    dataSourceType,
    setDataSourceType,
    serialConfig,
    serialPort,
    bleConfig,
    exportToCsv,
    parsedDataBuffer,
    jsonFiles,
    setJsonFiles,
    selectedJsonFile,
    setSelectedJsonFile,
    setJsonConfig,
    isRunning,
    setIsRunning,
  } = useDashboardStore();

  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadJsonFiles();
  }, []);

  const loadJsonFiles = async () => {
    setLoading(true);
    try {
      const files = await dashboardApi.getJsonFiles();
      setJsonFiles(files);
    } catch (error) {
      console.error('Failed to load JSON files:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleJsonFileSelect = async (fileName: string) => {
    setLoading(true);
    try {
      const config = await dashboardApi.loadJsonFile(fileName);
      setJsonConfig(config);
      setSelectedJsonFile(fileName);
      message.success(t('settings.configLoaded') || `已加载配置: ${fileName}`);
    } catch (error) {
      console.error('Failed to load JSON config:', error);
      message.error(t('settings.configLoadError') || '加载配置失败');
    } finally {
      setLoading(false);
    }
  };

  const handleExportCsv = async () => {
    const csvContent = exportToCsv();
    if (!csvContent) {
      message.warning(t('settings.noData') || '暂无数据可导出');
      return;
    }

    try {
      const filePath = await save({
        filters: [{ name: 'CSV Files', extensions: ['csv'] }],
        defaultPath: `dashboard_data_${Date.now()}.csv`,
      });

      if (filePath) {
        const encoder = new TextEncoder();
        await writeFile(filePath, encoder.encode(csvContent));
        message.success(t('settings.exportSuccess') || '导出成功');
      }
    } catch (error) {
      console.error('Export CSV error:', error);
      message.error(t('settings.exportError') || '导出失败');
    }
  };

  const handleToggleRun = () => {
    if (isRunning) {
      setIsRunning(false);
      message.info(t('settings.stopped') || '数据接收已停止');
    } else {
      if (!selectedJsonFile) {
        message.warning(t('settings.selectConfigFirst') || '请先选择配置文件');
        return;
      }
      setIsRunning(true);
      message.success(t('settings.started') || '数据接收已启动');
    }
  };

  const dataSourceOptions: { value: DataSourceType; label: string }[] = [
    { value: 'serial', label: t('settings.serial') || '串口' },
    { value: 'ble', label: t('settings.ble') || '蓝牙' },
    { value: 'file', label: t('settings.file') || '文件' },
    { value: 'manual', label: t('settings.manual') || '手动输入' },
  ];

  return (
    <Card
      title={t('settings.title') || '设置'}
      size="small"
      style={{ height: '100%', overflow: 'auto' }}
    >
      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 8, fontWeight: 500 }}>
          {t('settings.configFile') || '配置文件'}
        </div>
        <Select
          value={selectedJsonFile}
          onChange={handleJsonFileSelect}
          placeholder={t('settings.selectConfig') || '选择配置文件'}
          loading={loading}
          style={{ width: '100%' }}
          suffixIcon={<FileOutlined />}
        >
          {jsonFiles.map((file) => (
            <Select.Option key={file} value={file}>
              {file}
            </Select.Option>
          ))}
        </Select>
        {jsonFiles.length === 0 && !loading && (
          <Empty
            description={t('settings.noConfigFiles') || '暂无配置文件，请在JSON编辑器中创建'}
            style={{ marginTop: 8, fontSize: 12 }}
          />
        )}
      </div>

      <Divider style={{ margin: '12px 0' }} />

      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 8, fontWeight: 500 }}>
          {t('settings.dataSource') || '数据源'}
        </div>
        <Select
          value={dataSourceType}
          onChange={setDataSourceType}
          options={dataSourceOptions}
          style={{ width: '100%' }}
        />
      </div>

      <Divider style={{ margin: '12px 0' }} />

      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 8, fontWeight: 500 }}>
          {t('settings.connectionStatus') || '连接状态'}
        </div>
        <Space direction="vertical" style={{ width: '100%' }}>
          <Button
            type={isRunning ? 'default' : 'primary'}
            icon={isRunning ? <PauseCircleOutlined /> : <PlayCircleOutlined />}
            onClick={handleToggleRun}
            loading={loading}
            block
          >
            {isRunning ? (t('settings.stop') || '停止') : (t('settings.start') || '开始')}
          </Button>
          {dataSourceType === 'serial' && (
            <div style={{ padding: 8, background: token.colorBgLayout, borderRadius: 4, fontSize: 12 }}>
              <div>{t('settings.port') || '端口'}: {serialPort || (t('settings.notSelected') || '未选择')}</div>
              <div>{t('settings.baudRate') || '波特率'}: {serialConfig.baudRate}</div>
            </div>
          )}
          {dataSourceType === 'ble' && bleConfig && (
            <div style={{ padding: 8, background: token.colorBgLayout, borderRadius: 4, fontSize: 12 }}>
              <div>{t('settings.device') || '设备'}: {bleConfig.deviceName || (t('settings.notConnected') || '未连接')}</div>
            </div>
          )}
        </Space>
      </div>

      <Divider style={{ margin: '12px 0' }} />

      <div>
        <div style={{ marginBottom: 8, fontWeight: 500 }}>
          {t('settings.dataExport') || '数据导出'}
        </div>
        <Space direction="vertical" style={{ width: '100%' }}>
          <div style={{ fontSize: 12, color: token.colorTextTertiary }}>
            {t('settings.dataPoints') || '数据点'}: {parsedDataBuffer.length}
          </div>
          <Button
            type="default"
            icon={<DownloadOutlined />}
            onClick={handleExportCsv}
            disabled={parsedDataBuffer.length === 0}
            block
          >
            {t('settings.exportCsv') || '导出CSV'}
          </Button>
        </Space>
      </div>
    </Card>
  );
};

export default SettingsPanel;
