import React, { useEffect, useState } from 'react';
import {
  Card,
  Button,
  Progress,
  Tag,
  Modal,
  Descriptions,
  Space,
  Typography,
  Row,
  Col,
  theme,
  Divider,
  Table,
  Collapse,
} from 'antd';
import {
  PlayCircleOutlined,
  StopOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  FolderOpenOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { useGh3036Store } from '../../stores/gh3036Store';
import type { TestEvaluationResult, ChannelEvaluationResult } from '../../api/types';

const { Text, Paragraph } = Typography;

const FactoryTestTab: React.FC = () => {
  const { t } = useTranslation('gh3036');
  const { token } = theme.useToken();
  const {
    factoryTest,
    startFactoryTest,
    stopFactoryTest,
    continueFactoryTest,
    setFactoryTestConfigDirAsync,
    validateFactoryTestConfig,
    subscribeFactoryTestEvents,
    unsubscribeFactoryTestEvents,
    resetFactoryTest,
    validateThresholdConfig,
    loadThresholdConfig,
    loadEvaluationResult,
  } = useGh3036Store();

  const factoryTestListenerIdRef = React.useRef<number | null>(null);
  const [showEnvSwitchModal, setShowEnvSwitchModal] = useState(false);

  useEffect(() => {
    const listenerId = Date.now() + Math.random();
    factoryTestListenerIdRef.current = listenerId;

    subscribeFactoryTestEvents(listenerId);
    return () => {
      unsubscribeFactoryTestEvents(factoryTestListenerIdRef.current ?? undefined);
    };
  }, []);

  useEffect(() => {
    if (factoryTest.status === 'waiting_for_environment_switch') {
      setShowEnvSwitchModal(true);
    }
  }, [factoryTest.status]);

  useEffect(() => {
    if (factoryTest.configDir) {
      validateFactoryTestConfig();
      validateThresholdConfig();
      loadThresholdConfig();
    }
  }, [factoryTest.configDir, validateFactoryTestConfig, validateThresholdConfig, loadThresholdConfig]);

  useEffect(() => {
    if (factoryTest.status === 'completed' || factoryTest.status === 'failed') {
      loadEvaluationResult();
    }
  }, [factoryTest.status, loadEvaluationResult]);

  const handleSelectDir = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (selected && typeof selected === 'string') {
      await setFactoryTestConfigDirAsync(selected);
      validateFactoryTestConfig();
      validateThresholdConfig();
      loadThresholdConfig();
    }
  };

  const handleStart = async () => {
    const ts = () => new Date().toISOString().substr(11, 12);
    console.log(`[${ts()}] [FactoryTestTab] handleStart 被调用`);
    console.log(`[${ts()}] [FactoryTestTab] 当前状态: isRunning=${factoryTest.isRunning}, status=${factoryTest.status}`);
    console.log(`[${ts()}] [FactoryTestTab] configValidation:`, factoryTest.configValidation);
    
    resetFactoryTest();
    console.log(`[${ts()}] [FactoryTestTab] resetFactoryTest 完成，准备调用 startFactoryTest`);
    await startFactoryTest();
    console.log(`[${ts()}] [FactoryTestTab] startFactoryTest 完成`);
  };

  const handleStop = async () => {
    await stopFactoryTest();
  };

  const handleContinue = async () => {
    setShowEnvSwitchModal(false);
    await continueFactoryTest();
  };

  const getStatusTag = () => {
    const statusMap: Record<string, { color: string; text: string }> = {
      idle: { color: 'default', text: t('factory.statusIdle') },
      running: { color: 'processing', text: t('factory.statusRunning') },
      waiting_for_environment_switch: { color: 'warning', text: t('factory.statusWaiting') },
      completed: { color: 'success', text: t('factory.statusCompleted') },
      failed: { color: 'error', text: t('factory.statusFailed') },
      stopped: { color: 'default', text: t('factory.statusStopped') },
    };
    const { color, text } = statusMap[factoryTest.status] || statusMap.idle;
    return <Tag color={color}>{text}</Tag>;
  };

  const getConfigStatusTag = (configPath: string | null) => {
    if (configPath === null) {
      return <Tag color="error">{t('factory.configMissing')}</Tag>;
    }
    return <Tag color="success">{t('factory.configReady')}</Tag>;
  };

  const formatUuid = (uuid: number[]): string => {
    if (!uuid || uuid.length === 0) return '--';
    
    const formatSingleUuid = (bytes: number[]): string => {
      const hex = bytes.map((b) => b.toString(16).toUpperCase().padStart(2, '0')).join('');
      return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20, 32)}`;
    };

    if (uuid.length === 32) {
      const uuid1 = formatSingleUuid(uuid.slice(0, 16));
      const uuid2 = formatSingleUuid(uuid.slice(16, 32));
      return `${uuid1}\n${uuid2}`;
    }
    
    return formatSingleUuid(uuid);
  };

  const renderTestResult = (testResult: TestEvaluationResult) => {
    const testDisplayName = t(`factory.test_${testResult.test_name}`, { defaultValue: testResult.description || testResult.test_name });
    
    const columns = [
      {
        title: t('factory.channel'),
        dataIndex: 'channel_index',
        key: 'channel_index',
        width: 80,
        render: (idx: number) => `CH${idx}`,
      },
      {
        title: t('factory.value'),
        dataIndex: 'value',
        key: 'value',
        width: 100,
        render: (val: number, record: ChannelEvaluationResult) => (
          <Text style={{ color: record.pass ? token.colorText : token.colorError, fontWeight: 500 }}>
            {val} {testResult.unit || ''}
          </Text>
        ),
      },
      {
        title: t('factory.condition'),
        dataIndex: 'threshold_display',
        key: 'threshold_display',
        render: (display: string) => <Text code style={{ fontSize: 11 }}>{display}</Text>,
      },
      {
        title: t('factory.result'),
        dataIndex: 'pass',
        key: 'pass',
        width: 80,
        render: (pass: boolean) => pass ? (
          <Tag color="success" icon={<CheckCircleOutlined />}>{t('factory.pass')}</Tag>
        ) : (
          <Tag color="error" icon={<CloseCircleOutlined />}>{t('factory.fail')}</Tag>
        ),
      },
    ];

    return (
      <Collapse.Panel
        key={testResult.test_name}
        header={
          <Space>
            {testResult.pass ? (
              <CheckCircleOutlined style={{ color: token.colorSuccess }} />
            ) : (
              <CloseCircleOutlined style={{ color: token.colorError }} />
            )}
            <Text strong>{testDisplayName}</Text>
            <Tag color={testResult.pass ? 'success' : 'error'}>
              {testResult.pass ? t('factory.pass') : t('factory.fail')}
            </Tag>
          </Space>
        }
      >
        {testResult.enabled ? (
          <Table
            dataSource={testResult.channel_results}
            columns={columns}
            size="small"
            pagination={false}
            rowKey="channel_index"
            scroll={{ x: 'max-content' }}
          />
        ) : (
          <Text type="secondary">{t('factory.testDisabled')}</Text>
        )}
      </Collapse.Panel>
    );
  };

  const renderResultAndEvaluation = () => {
    const { result, evaluationResult } = factoryTest;
    if (!result && !evaluationResult) return null;

    const overallPass = evaluationResult?.overall_pass ?? (result?.overall_result === 'PASS');

    return (
      <Card size="small" title={t('factory.result')} style={{ marginTop: 8 }}>
        <div style={{ marginBottom: 8 }}>
          <Space>
            {overallPass ? (
              <CheckCircleOutlined style={{ color: token.colorSuccess, fontSize: 20 }} />
            ) : (
              <CloseCircleOutlined style={{ color: token.colorError, fontSize: 20 }} />
            )}
            <Text strong style={{ fontSize: 16 }}>
              {overallPass ? t('factory.allTestsPassed') : t('factory.someTestsFailed')}
            </Text>
            <Tag color={overallPass ? 'success' : 'error'} style={{ marginLeft: 8 }}>
              {overallPass ? t('factory.pass') : t('factory.fail')}
            </Tag>
          </Space>
        </div>

        {result && (
          <>
            <Descriptions size="small" column={1} bordered>
              <Descriptions.Item label={t('factory.chipInitStatus')}>
                {result.chip_init_status === 1 ? (
                  <CheckCircleOutlined style={{ color: token.colorSuccess }} />
                ) : (
                  <CloseCircleOutlined style={{ color: token.colorError }} />
                )}
              </Descriptions.Item>
              <Descriptions.Item label={t('factory.uuid')}>
                <Paragraph
                  copyable={{ text: formatUuid(result.uuid).replace('\n', '\n') }}
                  style={{ marginBottom: 0, fontFamily: 'monospace', fontSize: 13, whiteSpace: 'pre-wrap' }}
                >
                  {formatUuid(result.uuid)}
                </Paragraph>
              </Descriptions.Item>
            </Descriptions>
            
            <Divider style={{ margin: '12px 0' }} />
            
            <Collapse size="small" bordered={false} defaultActiveKey={evaluationResult?.test_results.map(r => r.test_name)}>
              {evaluationResult?.test_results.map(renderTestResult)}
            </Collapse>
          </>
        )}
      </Card>
    );
  };

  const cardStyle: React.CSSProperties = {
    background: token.colorBgContainer,
    borderRadius: token.borderRadius,
  };

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '8px 0' }}>
      <Row gutter={[8, 8]}>
        <Col span={24}>
          <Card size="small" title={t('factory.configDir')} style={cardStyle}>
            <Space orientation="vertical" style={{ width: '100%' }}>
              <Space.Compact style={{ width: '100%' }}>
                <Text
                  code
                  style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis' }}
                >
                  {factoryTest.configDir || t('factory.selectDir')}
                </Text>
                <Button
                  size="small"
                  icon={<FolderOpenOutlined />}
                  onClick={handleSelectDir}
                  disabled={factoryTest.isRunning}
                >
                  {t('factory.selectDir')}
                </Button>
              </Space.Compact>

              {factoryTest.configValidation && (
                <>
                  <Divider style={{ margin: '8px 0' }} />
                  <div>
                    <Text type="secondary" style={{ fontSize: 12, marginRight: 8 }}>
                      {t('factory.configStatus')}:
                    </Text>
                    <Space size={4}>
                      {getConfigStatusTag(factoryTest.configValidation.base_noise_config)}
                      {getConfigStatusTag(factoryTest.configValidation.ppg_noise_config)}
                      {getConfigStatusTag(factoryTest.configValidation.lpctr_config)}
                      {getConfigStatusTag(factoryTest.configValidation.lplctr_config)}
                    </Space>
                  </div>
                  {factoryTest.configValidation.errors.length > 0 && (
                    <Text type="danger" style={{ fontSize: 12 }}>
                      {factoryTest.configValidation.errors.join('; ')}
                    </Text>
                  )}
                </>
              )}
            </Space>
          </Card>
        </Col>

        <Col span={24}>
          <Card
            size="small"
            title={
              <Space>
                <span>{t('factory.title')}</span>
                {getStatusTag()}
              </Space>
            }
            extra={
              <Space>
                <Button
                  type="primary"
                  icon={<PlayCircleOutlined />}
                  onClick={handleStart}
                  disabled={
                    factoryTest.isRunning ||
                    !factoryTest.configValidation?.is_valid
                  }
                  size="small"
                >
                  {t('factory.start')}
                </Button>
                <Button
                  danger
                  icon={<StopOutlined />}
                  onClick={handleStop}
                  disabled={!factoryTest.isRunning}
                  size="small"
                >
                  {t('factory.stop')}
                </Button>
              </Space>
            }
            style={cardStyle}
          >
            <Space orientation="vertical" style={{ width: '100%' }}>
              <Progress
                percent={Math.round(factoryTest.progress * 100)}
                status={
                  factoryTest.status === 'failed'
                    ? 'exception'
                    : factoryTest.status === 'completed'
                    ? 'success'
                    : 'active'
                }
                size="small"
              />
              {factoryTest.message && (
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {factoryTest.message}
                </Text>
              )}
            </Space>
          </Card>
        </Col>

        {(factoryTest.result || factoryTest.evaluationResult) && (
          <Col span={24}>{renderResultAndEvaluation()}</Col>
        )}
      </Row>

      <Modal
        open={showEnvSwitchModal}
        title={t('factory.environmentSwitchTitle')}
        onOk={handleContinue}
        onCancel={() => setShowEnvSwitchModal(false)}
        okText={t('factory.confirmSwitch')}
        cancelText={t('common:cancel')}
        mask={{ closable: false }}
      >
        <Text>{t('factory.environmentSwitchMessage')}</Text>
      </Modal>
    </div>
  );
};

export default FactoryTestTab;
