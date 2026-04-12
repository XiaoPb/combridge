import React, { useState } from 'react';
import { Card, Radio, Button, Divider, message, Space, Select } from 'antd';
import { DownloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import { useDashboardStore } from '../../stores/dashboardStore';
import type { DataSourceType } from '../../types/dashboard';

const SettingsPanel: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const {
    dataSourceType,
    setDataSourceType,
    serialConfig,
    bleConfig,
    exportToCsv,
    parsedDataBuffer,
  } = useDashboardStore();

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
      style={{ height: '100%' }}
    >
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
          {dataSourceType === 'serial' ? (t('settings.serialConfig') || '串口配置') : 
           dataSourceType === 'ble' ? (t('settings.bleConfig') || '蓝牙配置') :
           dataSourceType === 'file' ? (t('settings.fileConfig') || '文件配置') :
           (t('settings.manualConfig') || '手动输入配置')}
        </div>
        
        {dataSourceType === 'serial' && (
          <div style={{ padding: 12, background: '#fafafa', borderRadius: 4 }}>
            <p style={{ margin: 0, color: '#666', fontSize: 12 }}>
              串口配置区域（待实现）
            </p>
            <p style={{ margin: '4px 0 0', color: '#999', fontSize: 11 }}>
              端口: {serialConfig.port || '未选择'} | 
              波特率: {serialConfig.baudRate}
            </p>
          </div>
        )}

        {dataSourceType === 'ble' && (
          <div style={{ padding: 12, background: '#fafafa', borderRadius: 4 }}>
            <p style={{ margin: 0, color: '#666', fontSize: 12 }}>
              蓝牙配置区域（待实现）
            </p>
            {bleConfig && (
              <p style={{ margin: '4px 0 0', color: '#999', fontSize: 11 }}>
                设备: {bleConfig.deviceName || '未连接'}
              </p>
            )}
          </div>
        )}

        {dataSourceType === 'file' && (
          <div style={{ padding: 12, background: '#fafafa', borderRadius: 4 }}>
            <p style={{ margin: 0, color: '#666', fontSize: 12 }}>
              文件回放配置区域（待实现）
            </p>
          </div>
        )}

        {dataSourceType === 'manual' && (
          <div style={{ padding: 12, background: '#fafafa', borderRadius: 4 }}>
            <p style={{ margin: 0, color: '#666', fontSize: 12 }}>
              手动输入配置区域（待实现）
            </p>
          </div>
        )}
      </div>

      <Divider style={{ margin: '12px 0' }} />

      <div>
        <div style={{ marginBottom: 8, fontWeight: 500 }}>
          {t('settings.dataExport') || '数据导出'}
        </div>
        <Space direction="vertical" style={{ width: '100%' }}>
          <div style={{ fontSize: 12, color: '#666' }}>
            {t('settings.dataPoints') || '数据点'}: {parsedDataBuffer.length}
          </div>
          <Button
            type="primary"
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
