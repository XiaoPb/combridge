import React, { useState, useEffect } from 'react';
import { Form, Input, Select, Switch, Button, Space, Card } from 'antd';
import { PlusOutlined, MinusCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { ParserConfig } from '../../api/waveform';

interface ParserConfigPanelProps {
  initialConfig?: ParserConfig;
  onConfigChange: (config: ParserConfig) => void;
}

const ParserConfigPanel: React.FC<ParserConfigPanelProps> = ({
  initialConfig,
  onConfigChange,
}) => {
  const { t } = useTranslation('waveform');
  const [form] = Form.useForm();
  const [parserType, setParserType] = useState<'delimiter' | 'regex'>(
    initialConfig?.parser_type || 'delimiter'
  );

  useEffect(() => {
    if (initialConfig) {
      form.setFieldsValue(initialConfig);
      setParserType(initialConfig.parser_type);
    }
  }, [initialConfig, form]);

  const handleValuesChange = () => {
    const values = form.getFieldsValue();
    const config: ParserConfig = {
      parser_type: values.parser_type,
      delimiter: values.delimiter || null,
      pattern: values.pattern || null,
      column_names: values.column_names || [],
      trim_whitespace: values.trim_whitespace ?? true,
    };
    onConfigChange(config);
  };

  return (
    <Card size="small" title={t('parser.title')} styles={{ body: { padding: 12 } }}>
      <Form
        form={form}
        layout="vertical"
        size="small"
        onValuesChange={handleValuesChange}
        initialValues={{
          parser_type: 'delimiter',
          delimiter: ',',
          trim_whitespace: true,
          column_names: ['CH0', 'CH1', 'CH2', 'CH3', 'CH4'],
        }}
      >
        <Form.Item name="parser_type" label={t('parser.type')}>
          <Select onChange={(value) => setParserType(value)}>
            <Select.Option value="delimiter">{t('parser.delimiter')}</Select.Option>
            <Select.Option value="regex">{t('parser.regex')}</Select.Option>
          </Select>
        </Form.Item>

        {parserType === 'delimiter' && (
          <Form.Item name="delimiter" label={t('parser.delimiterChar')}>
            <Input placeholder="," />
          </Form.Item>
        )}

        {parserType === 'regex' && (
          <Form.Item
            name="pattern"
            label={t('parser.pattern')}
            extra={t('parser.patternExample')}
          >
            <Input placeholder="(-?\d+),(-?\d+),(-?\d+)" />
          </Form.Item>
        )}

        <Form.Item name="trim_whitespace" label={t('parser.trimWhitespace')} valuePropName="checked">
          <Switch />
        </Form.Item>

        <Form.List name="column_names">
          {(fields, { add, remove }) => (
            <div>
              <div style={{ marginBottom: 8 }}>
                <span style={{ fontWeight: 500 }}>{t('parser.columnNames')}</span>
              </div>
              {fields.map((field) => (
                <Space key={field.key} style={{ display: 'flex', marginBottom: 8 }} align="baseline">
                  <Form.Item
                    {...field}
                    rules={[{ required: true, message: t('parser.columnNameRequired') }]}
                  >
                    <Input style={{ width: 120 }} />
                  </Form.Item>
                  {fields.length > 1 && (
                    <MinusCircleOutlined onClick={() => remove(field.name)} />
                  )}
                </Space>
              ))}
              <Button type="dashed" onClick={() => add()} icon={<PlusOutlined />} size="small">
                {t('parser.addColumn')}
              </Button>
            </div>
          )}
        </Form.List>
      </Form>
    </Card>
  );
};

export default ParserConfigPanel;
