import React, { useEffect, useState } from 'react';
import {
  Card,
  Button,
  Progress,
  Tag,
  Modal,
  Descriptions,
  List,
  Space,
  Typography,
  Row,
  Col,
  theme,
  Empty,
  Divider,
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
import type { FactoryTestStep, FactoryTestStepResult } from '../../api/types';

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
  } = useGh3036Store();

  const [showEnvSwitchModal, setShowEnvSwitchModal] = useState(false);

  useEffect(() => {
    subscribeFactoryTestEvents();
    return () => {
      unsubscribeFactoryTestEvents();
    };
  }, [subscribeFactoryTestEvents, unsubscribeFactoryTestEvents]);

  useEffect(() => {
    if (factoryTest.status === 'waiting_for_environment_switch') {
      setShowEnvSwitchModal(true);
    }
  }, [factoryTest.status]);

  useEffect(() => {
    if (factoryTest.configDir) {
      validateFactoryTestConfig();
    }
  }, [factoryTest.configDir, validateFactoryTestConfig]);

  const handleSelectDir = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (selected && typeof selected === 'string') {
      await setFactoryTestConfigDirAsync(selected);
    }
  };

  const handleStart = async () => {
    resetFactoryTest();
    await startFactoryTest();
  };

  const handleStop = async () => {
    await stopFactoryTest();
  };

  const handleContinue = async () => {
    setShowEnvSwitchModal(false);
    await continueFactoryTest();
  };

  const getStepLabel = (step: FactoryTestStep): string => {
    const key = step.replace(/_/g, '');
    const stepKey = `step${key.charAt(0).toUpperCase()}${key.slice(1)}` as keyof typeof t;
    return t(`factory.${stepKey}`);
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
    return uuid.map((b) => b.toString(16).toUpperCase().padStart(2, '0')).join(':');
  };

  const formatDataArray = (data: number[]): string => {
    if (!data || data.length === 0) return '--';
    return data.map((v) => v.toFixed(4)).join(', ');
  };

  const renderStepResult = (result: FactoryTestStepResult) => {
    const icon = result.success ? (
      <CheckCircleOutlined style={{ color: token.colorSuccess }} />
    ) : (
      <CloseCircleOutlined style={{ color: token.colorError }} />
    );

    return (
      <List.Item>
        <List.Item.Meta
          avatar={icon}
          title={
            <Space>
              <Text>{getStepLabel(result.step)}</Text>
              <Tag color={result.success ? 'success' : 'error'}>
                {result.success ? t('factory.pass') : t('factory.fail')}
              </Tag>
            </Space>
          }
          description={result.message}
        />
        {result.data.length > 0 && (
          <Text code style={{ fontSize: 11 }}>
            {formatDataArray(result.data)}
          </Text>
        )}
      </List.Item>
    );
  };

  const renderResultDetails = () => {
    const { result } = factoryTest;
    if (!result) return null;

    return (
      <Card size="small" title={t('factory.result')} style={{ marginTop: 8 }}>
        <Descriptions size="small" column={1} bordered>
          <Descriptions.Item label={t('factory.overallResult')}>
            <Tag color={result.overall_result === 'PASS' ? 'success' : 'error'}>
              {result.overall_result}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label={t('factory.chipInitStatus')}>
            {result.chip_init_status === 0 ? (
              <CheckCircleOutlined style={{ color: token.colorSuccess }} />
            ) : (
              <CloseCircleOutlined style={{ color: token.colorError }} />
            )}
          </Descriptions.Item>
          <Descriptions.Item label={t('factory.uuid')}>
            <Paragraph
              copyable={{ text: formatUuid(result.uuid) }}
              style={{ marginBottom: 0, fontFamily: 'monospace', fontSize: 12 }}
            >
              {formatUuid(result.uuid)}
            </Paragraph>
          </Descriptions.Item>
          <Descriptions.Item label={t('factory.baseNoiseData')}>
            <Text code style={{ fontSize: 11 }}>
              {formatDataArray(result.base_noise)}
            </Text>
          </Descriptions.Item>
          <Descriptions.Item label={t('factory.ppgNoiseData')}>
            <Text code style={{ fontSize: 11 }}>
              {formatDataArray(result.ppg_noise)}
            </Text>
          </Descriptions.Item>
          <Descriptions.Item label={t('factory.lpctrData')}>
            <Text code style={{ fontSize: 11 }}>
              {formatDataArray(result.lpctr)}
            </Text>
          </Descriptions.Item>
          <Descriptions.Item label={t('factory.lplctrData')}>
            <Text code style={{ fontSize: 11 }}>
              {formatDataArray(result.lplctr)}
            </Text>
          </Descriptions.Item>
        </Descriptions>
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
            <Space direction="vertical" style={{ width: '100%' }}>
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
            <Space direction="vertical" style={{ width: '100%' }}>
              <Progress
                percent={factoryTest.progress}
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

        <Col span={24}>
          <Card size="small" title={t('factory.testSteps')} style={cardStyle}>
            {factoryTest.stepResults.length > 0 ? (
              <List
                size="small"
                dataSource={factoryTest.stepResults}
                renderItem={renderStepResult}
              />
            ) : (
              <Empty
                description={t('factory.statusIdle')}
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              />
            )}
          </Card>
        </Col>

        {factoryTest.result && (
          <Col span={24}>{renderResultDetails()}</Col>
        )}
      </Row>

      <Modal
        open={showEnvSwitchModal}
        title={t('factory.environmentSwitchTitle')}
        onOk={handleContinue}
        onCancel={() => setShowEnvSwitchModal(false)}
        okText={t('factory.confirmSwitch')}
        cancelText={t('common:cancel')}
        maskClosable={false}
      >
        <Text>{t('factory.environmentSwitchMessage')}</Text>
      </Modal>
    </div>
  );
};

export default FactoryTestTab;
