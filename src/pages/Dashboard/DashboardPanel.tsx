import React, { useState, useEffect } from 'react';
import { Tabs, Table, Input, Typography, Button, Space, message, Form, InputNumber, ColorPicker, Popconfirm, Empty } from 'antd';
import type { Color } from 'antd/es/color-picker';
import { SendOutlined, ClearOutlined, DeleteOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import type { DataPoint, WidgetConfig } from '../../types/dashboard';

const { Text } = Typography;
const { TextArea } = Input;

const DashboardPanel: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { dataBuffer, addDataPoint, clearDataBuffer } = useDashboardStore();
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

  const { selectedWidget, getSelectedWidget, updateWidget, removeWidget, setSelectedWidget } = useDashboardStore();
  const [widgetForm] = Form.useForm();
  const [selectedWidgetConfig, setSelectedWidgetConfig] = useState<WidgetConfig | null>(null);

  useEffect(() => {
    const widget = getSelectedWidget();
    setSelectedWidgetConfig(widget);
    if (widget) {
      widgetForm.setFieldsValue({
        title: widget.title,
        dataKey: widget.dataKey,
        min: widget.min,
        max: widget.max,
        unit: widget.unit,
        color: widget.color || '#1890ff',
      });
    } else {
      widgetForm.resetFields();
    }
  }, [selectedWidget, getSelectedWidget, widgetForm]);

  const handleWidgetFormChange = (_: Record<string, unknown>, allValues: Record<string, unknown>) => {
    if (!selectedWidget) return;
    const colorValue = allValues.color;
    const colorString = typeof colorValue === 'string' 
      ? colorValue 
      : (colorValue as Color)?.toHexString?.() || allValues.color;
    updateWidget(selectedWidget, {
      title: allValues.title as string,
      dataKey: allValues.dataKey as string,
      min: allValues.min as number | undefined,
      max: allValues.max as number | undefined,
      unit: allValues.unit as string | undefined,
      color: colorString as string | undefined,
    });
  };

  const handleDeleteWidget = () => {
    if (!selectedWidget) return;
    removeWidget(selectedWidget);
    setSelectedWidget(null);
    widgetForm.resetFields();
    message.success(t('widget.delete') || 'Widget deleted');
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
    {
      key: 'widget',
      label: t('widget.title'),
      children: selectedWidgetConfig ? (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <Text strong>{t('widget.properties')}</Text>
          <Form
            form={widgetForm}
            layout="vertical"
            size="small"
            onValuesChange={handleWidgetFormChange}
          >
            <Form.Item name="title" label={t('widget.title')}>
              <Input placeholder={t('widget.title')} />
            </Form.Item>
            <Form.Item
              name="dataKey"
              label={t('widget.dataKey')}
              rules={[{ required: true, message: t('widget.noDataKey') }]}
            >
              <Input placeholder={t('widget.dataKey')} />
            </Form.Item>
            <Form.Item name="min" label={t('widget.min')}>
              <InputNumber style={{ width: '100%' }} placeholder={t('widget.min')} />
            </Form.Item>
            <Form.Item name="max" label={t('widget.max')}>
              <InputNumber style={{ width: '100%' }} placeholder={t('widget.max')} />
            </Form.Item>
            <Form.Item name="unit" label={t('widget.unit')}>
              <Input placeholder={t('widget.unit')} />
            </Form.Item>
            <Form.Item name="color" label={t('widget.color')}>
              <ColorPicker format="hex" showText />
            </Form.Item>
          </Form>
          <Popconfirm
            title={t('widget.deleteConfirm')}
            onConfirm={handleDeleteWidget}
            okText={t('widget.delete')}
            cancelText={t('jsonImport.cancel')}
          >
            <Button
              danger
              icon={<DeleteOutlined />}
              style={{ marginTop: 8 }}
            >
              {t('widget.delete')}
            </Button>
          </Popconfirm>
        </div>
      ) : (
        <Empty
          description={t('widget.selectType')}
          style={{ marginTop: 40 }}
        />
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
