import React from 'react';
import { Card, Form, Input, InputNumber, Select, Switch, ColorPicker, Button, Space, Row, Col } from 'antd';
import { DeleteOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { DatasetConfig } from '../../../types/dashboard';
import { WIDGET_SUPPORT_MATRIX } from '../../../types/dashboard';

interface DatasetEditorProps {
  dataset: DatasetConfig;
  onChange: (updates: Partial<DatasetConfig>) => void;
  onRemove: () => void;
}

const DatasetEditor: React.FC<DatasetEditorProps> = ({ dataset, onChange, onRemove }) => {
  const { t } = useTranslation('dashboard');

  const widgetOptions = [
    { value: 'x', label: 'X轴' },
    { value: 'y', label: 'Y轴' },
    { value: 'z', label: 'Z轴' },
    { value: 'bar', label: t('widgetTypes.bar') || '柱状图' },
    { value: 'gauge', label: t('widgetTypes.gauge') || '仪表盘' },
    { value: 'text', label: t('widgetTypes.text') || '文本' },
    { value: 'led', label: t('widgetTypes.led') || 'LED' },
  ];

  const getSupportConfig = (widgetType: string): boolean => {
    const matrix = WIDGET_SUPPORT_MATRIX[widgetType] || WIDGET_SUPPORT_MATRIX.text;
    return matrix[widgetType as keyof typeof matrix] ?? false;
  };

  const getWidgetSupport = (field: string): boolean => {
    const widgetType = dataset.widget || 'text';
    const matrix = WIDGET_SUPPORT_MATRIX[widgetType] || WIDGET_SUPPORT_MATRIX.text;
    return matrix[field as keyof typeof matrix] ?? false;
  };

  return (
    <Card
      size="small"
      style={{ marginBottom: 8, background: '#fafafa' }}
      extra={
        <Button
          type="text"
          danger
          icon={<DeleteOutlined />}
          onClick={onRemove}
          size="small"
        />
      }
    >
      <Form layout="vertical" size="small">
        <Row gutter={12}>
          <Col span={4}>
            <Form.Item label={t('jsonEditor.index') || '索引'}>
              <InputNumber
                value={dataset.index}
                onChange={(value) => onChange({ index: value ?? 0 })}
                min={0}
                style={{ width: '100%' }}
              />
            </Form.Item>
          </Col>
          <Col span={6}>
            <Form.Item label={t('jsonEditor.title') || '标题'}>
              <Input
                value={dataset.title}
                onChange={(e) => onChange({ title: e.target.value })}
              />
            </Form.Item>
          </Col>
          <Col span={4}>
            <Form.Item label={t('jsonEditor.units') || '单位'}>
              <Input
                value={dataset.units}
                onChange={(e) => onChange({ units: e.target.value })}
              />
            </Form.Item>
          </Col>
          <Col span={4}>
            <Form.Item label={t('jsonEditor.widget') || '组件'}>
              <Select
                value={dataset.widget}
                onChange={(value) => onChange({ widget: value })}
                options={widgetOptions}
              />
            </Form.Item>
          </Col>
          <Col span={3}>
            <Form.Item label={t('jsonEditor.color') || '颜色'}>
              <ColorPicker
                value={dataset.color}
                onChange={(color) => onChange({ color: color.toHexString() })}
                disabled={!getWidgetSupport('color')}
              />
            </Form.Item>
          </Col>
        </Row>

        <Row gutter={12}>
          <Col span={4}>
            <Form.Item label={t('jsonEditor.min') || '最小值'}>
              <InputNumber
                value={dataset.min}
                onChange={(value) => onChange({ min: value ?? 0 })}
                disabled={!getWidgetSupport('min')}
                style={{ width: '100%' }}
              />
            </Form.Item>
          </Col>
          <Col span={4}>
            <Form.Item label={t('jsonEditor.max') || '最大值'}>
              <InputNumber
                value={dataset.max}
                onChange={(value) => onChange({ max: value ?? 100 })}
                disabled={!getWidgetSupport('max')}
                style={{ width: '100%' }}
              />
            </Form.Item>
          </Col>
          <Col span={3}>
            <Form.Item label={t('jsonEditor.graph') || '图表'}>
              <Switch
                checked={dataset.graph}
                onChange={(checked) => onChange({ graph: checked })}
                disabled={!getWidgetSupport('graph')}
              />
            </Form.Item>
          </Col>
          <Col span={3}>
            <Form.Item label={t('jsonEditor.led') || 'LED'}>
              <Switch
                checked={dataset.led}
                onChange={(checked) => onChange({ led: checked })}
                disabled={!getWidgetSupport('led')}
              />
            </Form.Item>
          </Col>
          <Col span={3}>
            <Form.Item label={t('jsonEditor.ledHigh') || 'LED阈值'}>
              <InputNumber
                value={dataset.ledHigh}
                onChange={(value) => onChange({ ledHigh: value ?? 1 })}
                disabled={!getWidgetSupport('ledHigh')}
                style={{ width: '100%' }}
                step={0.1}
              />
            </Form.Item>
          </Col>
          <Col span={3}>
            <Form.Item label={t('jsonEditor.alarm') || '报警'}>
              <InputNumber
                value={dataset.alarm}
                onChange={(value) => onChange({ alarm: value ?? 0 })}
                disabled={!getWidgetSupport('alarm')}
                style={{ width: '100%' }}
                min={0}
              />
            </Form.Item>
          </Col>
          <Col span={3}>
            <Form.Item label={t('jsonEditor.log') || '日志'}>
              <Switch
                checked={dataset.log}
                onChange={(checked) => onChange({ log: checked })}
                disabled={!getWidgetSupport('log')}
              />
            </Form.Item>
          </Col>
        </Row>

        <Row gutter={12}>
          <Col span={4}>
            <Form.Item label={t('jsonEditor.fft') || 'FFT'}>
              <Switch
                checked={dataset.fft}
                onChange={(checked) => onChange({ fft: checked })}
                disabled={!getWidgetSupport('fft')}
              />
            </Form.Item>
          </Col>
          <Col span={5}>
            <Form.Item label={t('jsonEditor.fftSamples') || 'FFT采样数'}>
              <InputNumber
                value={dataset.fftSamples}
                onChange={(value) => onChange({ fftSamples: value ?? 1024 })}
                disabled={!getWidgetSupport('fft') || !dataset.fft}
                style={{ width: '100%' }}
              />
            </Form.Item>
          </Col>
          <Col span={5}>
            <Form.Item label={t('jsonEditor.fftSamplingRate') || 'FFT采样率'}>
              <InputNumber
                value={dataset.fftSamplingRate}
                onChange={(value) => onChange({ fftSamplingRate: value ?? 100 })}
                disabled={!getWidgetSupport('fft') || !dataset.fft}
                style={{ width: '100%' }}
              />
            </Form.Item>
          </Col>
        </Row>
      </Form>
    </Card>
  );
};

export default DatasetEditor;
