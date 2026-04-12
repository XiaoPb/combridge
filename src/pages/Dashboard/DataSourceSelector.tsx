import React from 'react';
import { Select, Space, Input } from 'antd';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import { useSerialStore } from '../../stores/serialStore';
import { useBleStore } from '../../stores/bleStore';
import type { DataSourceType } from '../../types/dashboard';

const DataSourceSelector: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const {
    dataSourceType,
    setDataSourceType,
    connectedDevice,
    setConnectedDevice,
  } = useDashboardStore();
  const { ports } = useSerialStore();
  const { connections } = useBleStore();

  const dataSourceOptions = [
    { label: t('dataSource.serial'), value: 'serial' as DataSourceType },
    { label: t('dataSource.ble'), value: 'ble' as DataSourceType },
    { label: t('dataSource.file'), value: 'file' as DataSourceType },
    { label: t('dataSource.manual'), value: 'manual' as DataSourceType },
  ];

  const deviceOptions =
    dataSourceType === 'serial'
      ? ports.map((p) => ({ label: p.name, value: p.name }))
      : connections.map((c) => ({ label: c.name || c.address, value: c.address }));

  return (
    <Space>
      <Select
        value={dataSourceType}
        onChange={setDataSourceType}
        options={dataSourceOptions}
        style={{ width: 120 }}
      />
      {dataSourceType === 'serial' && (
        <Select
          value={connectedDevice}
          onChange={setConnectedDevice}
          options={deviceOptions}
          placeholder={t('selectDevice')}
          style={{ width: 150 }}
        />
      )}
      {dataSourceType === 'ble' && (
        <Select
          value={connectedDevice}
          onChange={setConnectedDevice}
          options={deviceOptions}
          placeholder={t('selectDevice')}
          style={{ width: 150 }}
        />
      )}
      {dataSourceType === 'file' && (
        <Input
          placeholder={t('selectFile')}
          style={{ width: 200 }}
          readOnly
          onClick={() => {}}
        />
      )}
    </Space>
  );
};

export default DataSourceSelector;
