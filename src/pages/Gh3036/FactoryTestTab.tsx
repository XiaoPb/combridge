import React, { useEffect, useState, useCallback, useRef } from 'react';
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
  Alert,
  Steps,
  Input,
} from 'antd';
import {
  PlayCircleOutlined,
  StopOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  FolderOpenOutlined,
  SearchOutlined,
  ApiOutlined,
  SettingOutlined,
  LoadingOutlined,
  UnorderedListOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { useGh3036Store } from '../../stores/gh3036Store';
import { useBleStore, formatMacAddress } from '../../stores/bleStore';
import { bleApi } from '../../api/tauri';
import type { TestEvaluationResult, ChannelEvaluationResult } from '../../api/types';
import type { BleDeviceInfo } from '../../types/ble';

const { Text, Paragraph } = Typography;

const CHELSEA_DEVICE_NAME = 'ChelseaA_OS';
const RX_CHAR_UUID = '00000003-0000-1000-8000-00805f9b34fb';
const TX_CHAR_UUID = '00000004-0000-1000-8000-00805f9b34fb';
const SCAN_TIMEOUT_MS = 15000;

type SetupStep = 'idle' | 'scanning' | 'connecting' | 'discovering' | 'subscribing' | 'configuring' | 'ready' | 'error';

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
    configureTxChannel,
    configureRxChannel,
    updateChannelConfig,
  } = useGh3036Store();

  const { connections, addConnection, setCurrentDevice, setDevices, clearDevices } = useBleStore();

  const factoryTestListenerIdRef = useRef<number | null>(null);
  const [showEnvSwitchModal, setShowEnvSwitchModal] = useState(false);

  const [setupStep, setSetupStep] = useState<SetupStep>('idle');
  const [setupError, setSetupError] = useState<string | null>(null);
  const [connectedDevice, setConnectedDevice] = useState<{ address: string; name?: string } | null>(null);
  const [nameFilter, setNameFilter] = useState(CHELSEA_DEVICE_NAME);
  const [scannedDevices, setScannedDevices] = useState<BleDeviceInfo[]>([]);
  const autoSetupDoneRef = useRef(false);

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

  // 检查是否已有连接的设备，如果有则跳过扫描直接进入ready状态
  useEffect(() => {
    if (autoSetupDoneRef.current) return;
    if (connections.length > 0 && setupStep === 'idle') {
      const conn = connections[0];
      setConnectedDevice({ address: conn.address, name: conn.name || undefined });
      autoSetupDoneRef.current = true;
      setSetupStep('ready');
    }
  }, [connections, setupStep]);

  const handleScan = useCallback(async () => {
    setSetupError(null);
    setSetupStep('scanning');
    autoSetupDoneRef.current = false;
    clearDevices();
    setScannedDevices([]);

    try {
      await bleApi.configureBle('native');
      const deviceList = await bleApi.scanBleDevices({ timeout: SCAN_TIMEOUT_MS });
      setDevices(deviceList);
      setScannedDevices(deviceList);

      const filter = nameFilter.trim();
      const target = deviceList.find((d) =>
        filter ? d.name?.toLowerCase().includes(filter.toLowerCase()) : true
      );

      if (!target) {
        setSetupError(`未找到名称包含 "${filter}" 的设备，请确认设备已开机并在附近`);
        setSetupStep('error');
        return;
      }

      // 自动连接
      setSetupStep('connecting');
      const connection = await bleApi.connectBle(target.address);
      addConnection(connection);
      setCurrentDevice(target.address);
      setConnectedDevice({ address: target.address, name: target.name });

      // 自动发现GATT服务
      setSetupStep('discovering');
      const services = await bleApi.discoverBleServices(target.address);

      // 自动订阅 RX 特征
      setSetupStep('subscribing');
      const rxChar = services
        .flatMap((s) => s.characteristics || [])
        .find((c) => c.uuid === RX_CHAR_UUID);

      if (!rxChar) {
        setSetupError(`未找到特征 ${RX_CHAR_UUID}，请确认设备固件版本`);
        setSetupStep('error');
        return;
      }

      await bleApi.subscribeBleNotify(target.address, RX_CHAR_UUID);

      // 自动配置GH3036通道
      setSetupStep('configuring');
      const txOk = await configureTxChannel('ble', target.address, TX_CHAR_UUID);
      if (!txOk) {
        setSetupError('配置TX通道失败');
        setSetupStep('error');
        return;
      }
      const rxOk = await configureRxChannel('ble', target.address, RX_CHAR_UUID);
      if (!rxOk) {
        setSetupError('配置RX通道失败');
        setSetupStep('error');
        return;
      }
      await updateChannelConfig({
        connectionType: 'ble',
        bleDevice: target.address,
        txChar: TX_CHAR_UUID,
        rxChar: RX_CHAR_UUID,
      });

      autoSetupDoneRef.current = true;
      setSetupStep('ready');
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setSetupError(msg);
      setSetupStep('error');
    }
  }, [nameFilter, clearDevices, setDevices, addConnection, setCurrentDevice, configureTxChannel, configureRxChannel, updateChannelConfig]);

  const handleDisconnect = useCallback(async () => {
    if (!connectedDevice) return;
    try {
      await bleApi.disconnectBle(connectedDevice.address);
    } catch {}
    setConnectedDevice(null);
    setSetupStep('idle');
    autoSetupDoneRef.current = false;
  }, [connectedDevice]);

  const handleSelectDir = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected && typeof selected === 'string') {
      await setFactoryTestConfigDirAsync(selected);
      validateFactoryTestConfig();
      validateThresholdConfig();
      loadThresholdConfig();
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
    if (configPath === null) return <Tag color="error">{t('factory.configMissing')}</Tag>;
    return <Tag color="success">{t('factory.configReady')}</Tag>;
  };

  const formatUuid = (uuid: number[]): string => {
    if (!uuid || uuid.length === 0) return '--';
    const formatSingleUuid = (bytes: number[]): string => {
      const hex = bytes.map((b) => b.toString(16).toUpperCase().padStart(2, '0')).join('');
      return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20, 32)}`;
    };
    if (uuid.length === 32) {
      return `${formatSingleUuid(uuid.slice(0, 16))}\n${formatSingleUuid(uuid.slice(16, 32))}`;
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

  const getSetupStepIndex = () => {
    const map: Record<SetupStep, number> = {
      idle: 0, scanning: 0, connecting: 1, discovering: 2,
      subscribing: 2, configuring: 2, ready: 3, error: -1,
    };
    return map[setupStep] ?? 0;
  };

  const isSetupInProgress = ['scanning', 'connecting', 'discovering', 'subscribing', 'configuring'].includes(setupStep);

  const renderBleSetup = () => (
    <Card
      size="small"
      title={
        <Space>
          <ApiOutlined />
          <span>蓝牙连接</span>
          {setupStep === 'ready' && connectedDevice && (
            <Tag color="success">
              已连接: {connectedDevice.name || formatMacAddress(connectedDevice.address)}
            </Tag>
          )}
        </Space>
      }
      style={cardStyle}
      extra={
        setupStep === 'ready' && connectedDevice ? (
          <Button size="small" danger onClick={handleDisconnect}>断开</Button>
        ) : null
      }
    >
      {setupStep !== 'ready' && (
        <Space style={{ width: '100%', marginBottom: 12 }} direction="vertical" size={8}>
          <Space>
            <Text style={{ fontSize: 12, whiteSpace: 'nowrap' }}>设备名称过滤:</Text>
            <Input
              size="small"
              value={nameFilter}
              onChange={(e) => setNameFilter(e.target.value)}
              placeholder="输入设备名称关键词"
              style={{ width: 180 }}
              disabled={isSetupInProgress}
            />
            <Button
              type="primary"
              size="small"
              icon={isSetupInProgress ? <LoadingOutlined /> : <SearchOutlined />}
              onClick={handleScan}
              disabled={isSetupInProgress}
              loading={isSetupInProgress}
            >
              {isSetupInProgress ? '连接中...' : '扫描并连接'}
            </Button>
          </Space>

          {isSetupInProgress && (
            <Steps
              size="small"
              current={getSetupStepIndex()}
              items={[
                { title: '扫描设备', icon: setupStep === 'scanning' ? <LoadingOutlined /> : undefined },
                { title: '建立连接', icon: setupStep === 'connecting' ? <LoadingOutlined /> : undefined },
                { title: '发现服务/订阅', icon: ['discovering', 'subscribing', 'configuring'].includes(setupStep) ? <LoadingOutlined /> : undefined },
                { title: '就绪' },
              ]}
            />
          )}

          {setupStep === 'error' && setupError && (
            <Alert
              type="error"
              message={setupError}
              showIcon
              action={
                <Button size="small" onClick={handleScan}>重试</Button>
              }
            />
          )}

          {scannedDevices.length > 0 && (
            <Collapse
              size="small"
              ghost
              items={[{
                key: 'devices',
                label: (
                  <Space size={4}>
                    <UnorderedListOutlined />
                    <span style={{ fontSize: 12 }}>扫描到的设备 ({scannedDevices.length})</span>
                  </Space>
                ),
                children: (
                  <Table
                    dataSource={scannedDevices}
                    rowKey="address"
                    size="small"
                    pagination={false}
                    scroll={{ y: 160 }}
                    columns={[
                      {
                        title: '设备名称',
                        dataIndex: 'name',
                        key: 'name',
                        render: (name?: string) => name || <Text type="secondary">未知</Text>,
                      },
                      {
                        title: '地址',
                        dataIndex: 'address',
                        key: 'address',
                        render: (addr: string) => <Text code style={{ fontSize: 11 }}>{formatMacAddress(addr)}</Text>,
                      },
                      {
                        title: 'RSSI',
                        dataIndex: 'rssi',
                        key: 'rssi',
                        width: 70,
                        render: (rssi?: number) => rssi != null ? `${rssi} dBm` : '--',
                      },
                    ]}
                  />
                ),
              }]}
            />
          )}
        </Space>
      )}

      {setupStep === 'ready' && connectedDevice && (
        <Space direction="vertical" size={4} style={{ width: '100%' }}>
          <Space size={4}>
            <SettingOutlined style={{ color: token.colorSuccess }} />
            <Text style={{ fontSize: 12 }}>已自动订阅 RX 特征并配置通道</Text>
          </Space>
          <Text type="secondary" style={{ fontSize: 11 }}>
            TX: {TX_CHAR_UUID}
          </Text>
          <Text type="secondary" style={{ fontSize: 11 }}>
            RX: {RX_CHAR_UUID}
          </Text>
        </Space>
      )}
    </Card>
  );

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '8px 0' }}>
      <Row gutter={[8, 8]}>
        <Col span={24}>{renderBleSetup()}</Col>

        <Col span={24}>
          <Card size="small" title={t('factory.configDir')} style={cardStyle}>
            <Space direction="vertical" style={{ width: '100%' }}>
              <Space.Compact style={{ width: '100%' }}>
                <Text code style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis' }}>
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
                    !factoryTest.configValidation?.is_valid ||
                    setupStep !== 'ready'
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
              {setupStep !== 'ready' && !factoryTest.isRunning && (
                <Text type="secondary" style={{ fontSize: 12 }}>
                  请先完成蓝牙连接，再选择配置目录，然后点击开始测试
                </Text>
              )}
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
