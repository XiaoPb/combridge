import React, { useState } from 'react';
import { Tabs, Table, Input, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import type { DataPoint } from '../../types/dashboard';

const { Text } = Typography;
const { TextArea } = Input;

const DashboardPanel: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { dataBuffer } = useDashboardStore();
  const [manualInput, setManualInput] = useState('');

  const columns = [
    {
      title: t('timestamp'),
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 100,
      render: (ts: number) => new Date(ts).toLocaleTimeString(),
    },
    {
      title: t('data'),
      dataIndex: 'values',
      key: 'values',
      render: (values: Record<string, number>) => (
        <Text style={{ fontSize: 11, fontFamily: 'monospace' }}>
          {JSON.stringify(values)}
        </Text>
      ),
    },
  ];

  const tabItems = [
    {
      key: 'data',
      label: t('dataView'),
      children: (
        <Table
          size="small"
          dataSource={dataBuffer}
          columns={columns}
          rowKey={(record: DataPoint, index?: number) =>
            `${record.timestamp}-${index}`
          }
          pagination={false}
          scroll={{ y: 200 }}
        />
      ),
    },
    {
      key: 'raw',
      label: t('rawData'),
      children: (
        <TextArea
          readOnly
          value={dataBuffer
            .map((d) => JSON.stringify(d.values))
            .join('\n')}
          style={{ height: 200, fontFamily: 'monospace', fontSize: 11 }}
        />
      ),
    },
    {
      key: 'manual',
      label: t('manualInput'),
      children: (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <TextArea
            value={manualInput}
            onChange={(e) => setManualInput(e.target.value)}
            placeholder={t('manualInputPlaceholder')}
            style={{ height: 150, fontFamily: 'monospace', fontSize: 11 }}
          />
        </div>
      ),
    },
  ];

  return (
    <div style={{ height: '100%', padding: 8 }}>
      <Tabs defaultActiveKey="data" items={tabItems} size="small" />
    </div>
  );
};

export default DashboardPanel;
