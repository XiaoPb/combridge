import React, { useMemo, useState } from 'react';
import {
  Button,
  Card,
  Col,
  Collapse,
  Form,
  Input,
  InputNumber,
  Row,
  Select,
  Space,
  Switch,
  Typography,
  message,
  theme,
} from 'antd';
import { DownloadOutlined, FileDoneOutlined, FolderOpenOutlined, PlusOutlined, DeleteOutlined } from '@ant-design/icons';
import { open, save } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';
import { formatErrorMessage } from '../../utils/errorMessage';
import { factoryTestApi } from '../../api/gh3036';
import type {
  ChannelRule,
  FactoryThresholdConfig,
  TestItemConfig,
  ThresholdConfig,
  ThresholdOperator,
} from '../../api/types';

const { TextArea } = Input;
const { Text } = Typography;

type TestKey = 'base_noise' | 'ppg_noise' | 'lpctr' | 'lplctr';

interface RuleFormValue {
  channels?: string;
  operator?: ThresholdOperator;
  value?: number;
  min?: number;
  max?: number;
  description?: string;
}

interface TestFormValue {
  enabled?: boolean;
  description?: string;
  unit?: string;
  operator?: ThresholdOperator;
  value?: number;
  min?: number;
  max?: number;
  rules?: RuleFormValue[];
}

interface ThresholdFormValue {
  project: string;
  version: string;
  description?: string;
  failAction: 'stop' | 'continue';
  tests: Record<TestKey, TestFormValue>;
}

const TEST_KEYS: TestKey[] = ['base_noise', 'ppg_noise', 'lpctr', 'lplctr'];
const OPERATORS: ThresholdOperator[] = ['lt', 'le', 'gt', 'ge', 'eq', 'ne', 'range'];
const OPERATOR_SYMBOLS: Record<ThresholdOperator, string> = {
  lt: '<',
  le: '<=',
  gt: '>',
  ge: '>=',
  eq: '=',
  ne: '!=',
  range: '[]',
};
const PROJECT_NAME_PATTERN = /^[\u4e00-\u9fa5A-Za-z0-9_-]+$/;

const DEFAULT_VALUES: ThresholdFormValue = {
  project: 'GH3036',
  version: '1.0',
  description: 'GH3036 产测卡控配置',
  failAction: 'stop',
  tests: {
    base_noise: {
      enabled: true,
      description: '底噪测试',
      unit: 'LSB',
      operator: 'lt',
      value: 100,
      rules: [],
    },
    ppg_noise: {
      enabled: true,
      description: 'PPG噪声测试',
      unit: 'LSB',
      operator: 'lt',
      value: 200,
      rules: [],
    },
    lpctr: {
      enabled: true,
      description: 'LPCTR测试',
      unit: 'count',
      operator: 'range',
      min: 0,
      max: 1000,
      rules: [],
    },
    lplctr: {
      enabled: true,
      description: 'LPLCTR测试',
      unit: 'count',
      operator: 'range',
      min: 0,
      max: 2000,
      rules: [],
    },
  },
};

const parseChannels = (value?: string): number[] =>
  (value || '')
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => Number(item))
    .filter((item) => Number.isInteger(item) && item >= 0);

const buildThreshold = (value: {
  operator?: ThresholdOperator;
  value?: number;
  min?: number;
  max?: number;
  description?: string;
}): ThresholdConfig => {
  const operator = value.operator || 'lt';
  if (operator === 'range') {
    return {
      operator,
      range: [Number(value.min ?? 0), Number(value.max ?? 0)],
      description: value.description || undefined,
    };
  }
  return {
    operator,
    value: Number(value.value ?? 0),
    description: value.description || undefined,
  };
};

const getThresholdFormValue = (threshold?: ThresholdConfig): Partial<TestFormValue> => {
  if (!threshold) {
    return {};
  }
  if (threshold.operator === 'range') {
    return {
      operator: threshold.operator,
      min: threshold.range?.[0] ?? 0,
      max: threshold.range?.[1] ?? 0,
      description: threshold.description,
    };
  }
  return {
    operator: threshold.operator,
    value: threshold.value ?? 0,
    description: threshold.description,
  };
};

const getRuleFormValue = (rule: ChannelRule): RuleFormValue => ({
  channels: rule.channels.join(','),
  ...getThresholdFormValue(rule),
});

const buildTestConfig = (test: TestFormValue): TestItemConfig => {
  const channelRules: ChannelRule[] = (test.rules || [])
    .map((rule) => ({
      channels: parseChannels(rule.channels),
      ...buildThreshold(rule),
    }))
    .filter((rule) => rule.channels.length > 0);

  return {
    enabled: test.enabled ?? true,
    description: test.description || undefined,
    unit: test.unit || undefined,
    global_threshold: buildThreshold(test),
    channel_rules: channelRules.length > 0 ? channelRules : undefined,
  };
};

const buildConfig = (values: ThresholdFormValue): FactoryThresholdConfig => ({
  project: values.project,
  version: values.version,
  description: values.description || undefined,
  global: {
    default_operator: 'lt',
    fail_action: values.failAction,
  },
  tests: {
    base_noise: buildTestConfig(values.tests.base_noise),
    ppg_noise: buildTestConfig(values.tests.ppg_noise),
    lpctr: buildTestConfig(values.tests.lpctr),
    lplctr: buildTestConfig(values.tests.lplctr),
  },
});

const buildDefaultFileName = (project: string) => `factory_config_${project.trim()}.yaml`;

const getDefaultTestValues = (testKey: TestKey): TestFormValue => ({
  ...DEFAULT_VALUES.tests[testKey],
  rules: [...(DEFAULT_VALUES.tests[testKey].rules || [])],
});

const getTestFormValue = (testKey: TestKey, test?: TestItemConfig): TestFormValue => {
  const defaults = getDefaultTestValues(testKey);
  if (!test) {
    return defaults;
  }

  return {
    ...defaults,
    enabled: test.enabled ?? defaults.enabled,
    description: test.description ?? defaults.description,
    unit: test.unit ?? defaults.unit,
    ...getThresholdFormValue(test.global_threshold),
    rules: (test.channel_rules || []).map(getRuleFormValue),
  };
};

const getFormValuesFromConfig = (config: FactoryThresholdConfig): ThresholdFormValue => ({
  project: config.project,
  version: config.version,
  description: config.description,
  failAction: config.global?.fail_action ?? 'stop',
  tests: {
    base_noise: getTestFormValue('base_noise', config.tests.base_noise),
    ppg_noise: getTestFormValue('ppg_noise', config.tests.ppg_noise),
    lpctr: getTestFormValue('lpctr', config.tests.lpctr),
    lplctr: getTestFormValue('lplctr', config.tests.lplctr),
  },
});

const getFileName = (filePath: string) => filePath.split(/[\\/]/).pop() || filePath;

const ThresholdConfigTab: React.FC = () => {
  const { t } = useTranslation('gh3036');
  const { token } = theme.useToken();
  const [form] = Form.useForm<ThresholdFormValue>();
  const [yamlPreview, setYamlPreview] = useState('');
  const [validating, setValidating] = useState(false);
  const [loadedFilePath, setLoadedFilePath] = useState<string | null>(null);

  const operatorOptions = useMemo(
    () =>
      OPERATORS.map((op) => ({
        value: op,
        label: `${t(`threshold.operators.${op}`)} (${OPERATOR_SYMBOLS[op]})`,
      })),
    [t]
  );

  const generateYaml = async () => {
    const values = await form.validateFields();
    const yaml = await factoryTestApi.generateThresholdYaml(buildConfig(values));
    setYamlPreview(yaml);
    return yaml;
  };

  const handlePreview = async () => {
    try {
      setValidating(true);
      const yaml = await generateYaml();
      const validation = await factoryTestApi.validateThresholdYaml(yaml);
      if (validation.is_valid) {
        message.success(t('threshold.valid'));
      } else {
        message.error(validation.errors.join('; ') || t('threshold.invalid'));
      }
    } catch (err) {
      message.error(formatErrorMessage(err, t('threshold.invalid')));
    } finally {
      setValidating(false);
    }
  };

  const handleLoad = async () => {
    try {
      setValidating(true);
      const selected = await open({
        multiple: false,
        filters: [{ name: 'YAML', extensions: ['yaml', 'yml'] }],
      });
      if (!selected || Array.isArray(selected)) return;

      const result = await factoryTestApi.loadThresholdYamlFile(selected);
      if (!result.validation.is_valid) {
        message.error(result.validation.errors.join('; ') || t('threshold.loadFailed'));
        return;
      }

      const formValues = getFormValuesFromConfig(result.config);
      form.setFieldsValue(formValues);
      setLoadedFilePath(result.file_path);
      const yaml = await factoryTestApi.generateThresholdYaml(buildConfig(formValues));
      setYamlPreview(yaml);
      message.success(t('threshold.loaded'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('threshold.loadFailed')));
    } finally {
      setValidating(false);
    }
  };

  const handleSave = async () => {
    try {
      setValidating(true);
      const yaml = await generateYaml();
      const validation = await factoryTestApi.validateThresholdYaml(yaml);
      if (!validation.is_valid) {
        message.error(validation.errors.join('; ') || t('threshold.invalid'));
        return;
      }

      if (loadedFilePath) {
        const saveValidation = await factoryTestApi.saveThresholdYamlFile(loadedFilePath, yaml);
        if (!saveValidation.is_valid) {
          message.error(saveValidation.errors.join('; ') || t('threshold.invalid'));
          return;
        }
        message.success(t('threshold.overwriteSaved'));
        return;
      }

      const values = await form.validateFields(['project']);
      const filePath = await save({
        defaultPath: buildDefaultFileName(values.project),
        filters: [{ name: 'YAML', extensions: ['yaml', 'yml'] }],
      });
      if (!filePath) return;
      const saveValidation = await factoryTestApi.saveThresholdYamlFile(filePath, yaml);
      if (!saveValidation.is_valid) {
        message.error(saveValidation.errors.join('; ') || t('threshold.invalid'));
        return;
      }
      message.success(t('threshold.saved'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('threshold.saveFailed')));
    } finally {
      setValidating(false);
    }
  };

  const renderThresholdFields = (namePath: (string | number)[]) => (
    <Row gutter={8}>
      <Col span={6}>
        <Form.Item name={[...namePath, 'operator']} label={t('threshold.operator')}>
          <Select options={operatorOptions} />
        </Form.Item>
      </Col>
      <Form.Item noStyle shouldUpdate>
        {({ getFieldValue }) => {
          const operator = getFieldValue([...namePath, 'operator']);
          if (operator === 'range') {
            return (
              <>
                <Col span={6}>
                  <Form.Item name={[...namePath, 'min']} label={t('threshold.min')}>
                    <InputNumber min={0} style={{ width: '100%' }} />
                  </Form.Item>
                </Col>
                <Col span={6}>
                  <Form.Item name={[...namePath, 'max']} label={t('threshold.max')}>
                    <InputNumber min={0} style={{ width: '100%' }} />
                  </Form.Item>
                </Col>
              </>
            );
          }
          return (
            <Col span={12}>
              <Form.Item name={[...namePath, 'value']} label={t('threshold.value')}>
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          );
        }}
      </Form.Item>
      <Col span={6}>
        <Form.Item name={[...namePath, 'description']} label={t('threshold.ruleDescription')}>
          <Input />
        </Form.Item>
      </Col>
    </Row>
  );

  const renderTestPanel = (testKey: TestKey) => (
    <Card size="small" title={t(`factory.test_${testKey}`)} style={{ marginBottom: 8 }}>
      <Row gutter={8}>
        <Col span={4}>
          <Form.Item name={['tests', testKey, 'enabled']} label={t('threshold.enabled')} valuePropName="checked">
            <Switch />
          </Form.Item>
        </Col>
        <Col span={10}>
          <Form.Item name={['tests', testKey, 'description']} label={t('threshold.description')}>
            <Input />
          </Form.Item>
        </Col>
        <Col span={10}>
          <Form.Item name={['tests', testKey, 'unit']} label={t('threshold.unit')}>
            <Input />
          </Form.Item>
        </Col>
      </Row>
      {renderThresholdFields(['tests', testKey])}
      <Form.List name={['tests', testKey, 'rules']}>
        {(fields, { add, remove }) => (
          <Space direction="vertical" style={{ width: '100%' }}>
            <Space style={{ justifyContent: 'space-between', width: '100%' }}>
              <Text strong>{t('threshold.channelRules')}</Text>
              <Button size="small" icon={<PlusOutlined />} onClick={() => add({ operator: 'lt', value: 0 })}>
                {t('threshold.addRule')}
              </Button>
            </Space>
            {fields.map((field) => (
              <Card
                key={field.key}
                size="small"
                styles={{ body: { padding: 8 } }}
                extra={
                  <Button
                    size="small"
                    danger
                    type="text"
                    icon={<DeleteOutlined />}
                    onClick={() => remove(field.name)}
                  />
                }
              >
                <Row gutter={8}>
                  <Col span={6}>
                    <Form.Item name={[field.name, 'channels']} label={t('threshold.channels')}>
                      <Input placeholder="0,1,2" />
                    </Form.Item>
                  </Col>
                  <Col span={18}>{renderThresholdFields(['tests', testKey, 'rules', field.name])}</Col>
                </Row>
              </Card>
            ))}
          </Space>
        )}
      </Form.List>
    </Card>
  );

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '8px 0' }}>
      <Row gutter={[8, 8]}>
        <Col span={14}>
          <Card
            size="small"
            title={t('threshold.title')}
            style={{ background: token.colorBgContainer, borderRadius: token.borderRadius }}
            extra={
              <Space>
                <Button icon={<FolderOpenOutlined />} loading={validating} onClick={handleLoad}>
                  {t('threshold.load')}
                </Button>
                <Button icon={<FileDoneOutlined />} loading={validating} onClick={handlePreview}>
                  {t('threshold.preview')}
                </Button>
                <Button type="primary" icon={<DownloadOutlined />} loading={validating} onClick={handleSave}>
                  {loadedFilePath ? t('threshold.save') : t('threshold.saveAs')}
                </Button>
              </Space>
            }
          >
            {loadedFilePath && (
              <Text
                type="secondary"
                ellipsis={{ tooltip: loadedFilePath }}
                style={{ display: 'block', marginBottom: 8 }}
              >
                {t('threshold.currentFile')}: {getFileName(loadedFilePath)}
              </Text>
            )}
            <Form form={form} layout="vertical" initialValues={DEFAULT_VALUES}>
              <Row gutter={8}>
                <Col span={8}>
                  <Form.Item
                    name="project"
                    label={t('threshold.project')}
                    rules={[
                      { required: true, whitespace: true, message: t('threshold.projectRequired') },
                      {
                        pattern: PROJECT_NAME_PATTERN,
                        message: t('threshold.projectInvalid'),
                      },
                    ]}
                  >
                    <Input />
                  </Form.Item>
                </Col>
                <Col span={8}>
                  <Form.Item name="version" label={t('threshold.version')} rules={[{ required: true }]}>
                    <Input />
                  </Form.Item>
                </Col>
                <Col span={8}>
                  <Form.Item name="failAction" label={t('threshold.failAction')}>
                    <Select
                      options={[
                        { value: 'stop', label: t('threshold.stop') },
                        { value: 'continue', label: t('threshold.continue') },
                      ]}
                    />
                  </Form.Item>
                </Col>
                <Col span={24}>
                  <Form.Item name="description" label={t('threshold.description')}>
                    <Input />
                  </Form.Item>
                </Col>
              </Row>
              <Collapse
                defaultActiveKey={TEST_KEYS}
                items={TEST_KEYS.map((key) => ({
                  key,
                  label: t(`factory.test_${key}`),
                  children: renderTestPanel(key),
                }))}
              />
            </Form>
          </Card>
        </Col>
        <Col span={10}>
          <Card size="small" title={t('threshold.yamlPreview')} style={{ height: '100%' }}>
            <TextArea
              value={yamlPreview}
              readOnly
              autoSize={{ minRows: 28, maxRows: 42 }}
              style={{ fontFamily: 'Consolas, Monaco, monospace', fontSize: 12 }}
            />
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default ThresholdConfigTab;
