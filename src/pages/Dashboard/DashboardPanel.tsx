import React, { useState } from 'react';
import { Tabs, Table, Input, Typography, Button, Space, message } from 'antd';
import { SendOutlined, ClearOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import type { DataPoint } from '../../types/dashboard';

const { Text } = Typography;
const { TextArea } = Input;

const DashboardPanel: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { dataBuffer, addDataPoint, clearDataBuffer, parserScript } = useDashboardStore();
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

  const handleSendManualData = () => {
    if (!manualInput.trim()) {
      message.warning(t('noManualInput') || 'Please enter data');
      return;
    }

    const lines = manualInput.split('\n').filter((line) => line.trim());

    for (const line of lines) {
      try {
        const parsed = JSON.parse(line);
        const values: Record<string, number> = {};
        for (const [key, value] of Object.entries(parsed)) {
          if (typeof value === 'number') {
            values[key] = value;
          }
        }
        addDataPoint({
          timestamp: Date.now(),
          values,
        });
      } catch {
        const numValue = parseFloat(line);
        if (!isNaN(numValue)) {
          addDataPoint({
            timestamp: Date.now(),
            values: { value: numValue },
          });
        } else {
          addDataPoint({
            timestamp: Date.now(),
            values: { raw: 0 },
          });
        }
      }
    }

    message.success(t('dataSent') || 'Data sent');
    setManualInput('');
  };

  const handleClearData = () => {
    clearDataBuffer();
    message.success(t('dataCleared') || 'Data cleared');
  };

  const tabItems = [
    {
      key: 'data',
      label: t('dataView'),
      children: (
        <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
          <div style={{ flex: 1, minHeight: 0 }}>
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
          </div>
          <Space style={{ marginTop: 8 }}>
            <Text type="secondary">
              {t('totalPoints') || 'Total'}: {dataBuffer.length}
            </Text>
            <Button
              size="small"
              icon={<ClearOutlined />}
              onClick={handleClearData}
            >
              {t('clear') || 'Clear'}
            </Button>
          </Space>
        </div>
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
          <Text type="secondary">
            {t('manualInputHint') || 'Enter JSON or numeric data, one per line'}
          </Text>
          <TextArea
            value={manualInput}
            onChange={(e) => setManualInput(e.target.value)}
            placeholder={`{"temperature": 25.6, "humidity": 65.2}\n{"temperature": 26.1, "humidity": 64.8}\n123.45`}
            style={{ height: 150, fontFamily: 'monospace', fontSize: 11 }}
          />
          <Button
            type="primary"
            icon={<SendOutlined />}
            onClick={handleSendManualData}
          >
            {t('send') || 'Send'}
          </Button>
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
