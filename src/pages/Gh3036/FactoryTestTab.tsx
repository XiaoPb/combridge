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
  }, []);

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

  const formatDataArray = (data: number[]): string => {
    if (!data || data.length === 0) return '--';
    return data.map((v) => v.toFixed(4)).join(', ');
  };

  const renderChannelData = (label: string, data: number[], color: string) => {
    if (!data || data.length === 0) return null;
    
    return (
      <Card 
        size="small" 
        style={{ marginBottom: 8 }}
        styles={{
          body: { padding: '8px 12px' }
        }}
      >
        <Space orientation="vertical" size={4} style={{ width: '100%' }}>
          <Text type="secondary" style={{ fontSize: 14, fontWeight: 500 }}>{label}</Text>
          <Row gutter={[8, 4]}>
            {data.map((value, index) => (
              <Col key={index} span={6}>
                <div style={{ 
                  padding: '6px 10px', 
                  background: token.colorBgContainer,
                  borderRadius: token.borderRadiusSM,
                  border: `1px solid ${token.colorBorderSecondary}`
                }}>
                  <Text style={{ fontSize: 14, fontFamily: 'monospace', fontWeight: 500 }}>
                    {value.toFixed(4)}
                  </Text>
                </div>
              </Col>
            ))}
          </Row>
        </Space>
      </Card>
    );
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
        
        <div style={{ marginTop: 12 }}>
          {renderChannelData(t('factory.baseNoiseData'), result.base_noise, '#1890ff')}
          {renderChannelData(t('factory.ppgNoiseData'), result.ppg_noise, '#52c41a')}
          {renderChannelData(t('factory.lpctrData'), result.lpctr, '#faad14')}
          {renderChannelData(t('factory.lplctrData'), result.lplctr, '#eb2f96')}
        </div>
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
        mask={{ closable: false }}
      >
        <Text>{t('factory.environmentSwitchMessage')}</Text>
      </Modal>
    </div>
  );
};

export default FactoryTestTab;
