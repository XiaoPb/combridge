import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Descriptions, Input, Progress, Select, Space, Steps, Switch, Tag, Typography } from 'antd';
import { open } from '@tauri-apps/plugin-dialog';
import { CloudUploadOutlined, ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useOtaStore } from '../../stores/otaStore';
import { useBleStore } from '../../stores/bleStore';
import type { OtaStartOptions, OtaStage } from '../../types/ota';

const STAGE_INDEX: Record<OtaStage, number> = {
  idle: 0, unpacking: 0, validating: 0, preparing: 1, transferring: 2,
  activating: 3, rebooting: 4, completed: 4, failed: 0,
};

export const UpgradeTab: React.FC = () => {
  const { t } = useTranslation('ota');
  const connections = useBleStore((s) => s.connections);
  const [address, setAddress] = useState<string>();
  const [forceConfirm, setForceConfirm] = useState(true);
  const [upgradeOnly, setUpgradeOnly] = useState(true);
  const [route, setRoute] = useState<NonNullable<OtaStartOptions['route']>>('auto');
  const [transferMode, setTransferMode] = useState<NonNullable<OtaStartOptions['transferMode']>>('auto');
  const [targetAddress, setTargetAddress] = useState('');
  const [runImmediately, setRunImmediately] = useState(true);

  const {
    status, protocols, selectedProtocol, deviceInfo, packageInfo, preflighting, validating, error,
    loadProtocols, setProtocol, validatePackage, clearPackage, preflight, start, cancel, refreshStatus,
  } = useOtaStore();

  const connected = useMemo(() => connections.filter((c) => c.isConnected), [connections]);
  const running = status.state === 'running';
  const targetConnected = address != null && connected.some((c) => c.address === address);
  const isGoodix = selectedProtocol === 'goodix';

  useEffect(() => {
    loadProtocols();
    refreshStatus();
  }, [loadProtocols, refreshStatus]);

  useEffect(() => {
    if (address && !running) {
      preflight(address);
    }
  }, [address, running, preflight, selectedProtocol]);

  const pickPackage = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Firmware Package', extensions: ['zip'] }],
    });
    if (typeof selected !== 'string') {
      return;
    }
    clearPackage();
    await validatePackage(selected);
  };

  const startWithOptions = () => {
    if (!address) {
      return;
    }
    const options: OtaStartOptions = isGoodix
      ? {
          forceConfirm,
          upgradeOnly,
          route,
          transferMode,
          targetAddress: targetAddress.trim() || undefined,
          runImmediately,
        }
      : { forceConfirm, upgradeOnly };
    void start(address, options);
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      {error && <Alert type="error" showIcon message={error} />}

      {protocols.length > 1 && (
        <Card size="small" title={t('protocol.title')}>
          <Space direction="vertical" style={{ width: '100%' }}>
            <Select
              style={{ width: '100%' }}
              value={selectedProtocol}
              disabled={running}
              onChange={(value) => void setProtocol(value)}
              options={protocols.map((item) => ({
                value: item.id,
                label: `${item.name}（${item.id}）`,
              }))}
            />
            <Typography.Text type="secondary">
              {protocols.find((item) => item.id === selectedProtocol)?.description ?? ''}
            </Typography.Text>
          </Space>
        </Card>
      )}

      <Card
        title={t('device.title')}
        extra={
          <Button
            size="small"
            icon={<ReloadOutlined />}
            disabled={running}
            onClick={() => {
              void refreshStatus();
              if (!running && address) {
                void preflight(address);
              }
            }}
          />
        }
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Select
            style={{ width: '100%' }}
            placeholder={t('device.placeholder')}
            value={address}
            disabled={running}
            onChange={setAddress}
            options={connected.map((c) => ({
              value: c.address,
              label: c.name ? `${c.name} (${c.address})` : c.address,
            }))}
          />
          {connected.length === 0 && <Alert type="warning" showIcon message={t('device.none')} />}
          {deviceInfo && (
            <Descriptions size="small" column={2}>
              <Descriptions.Item label={t('device.protocol')}>
                <Tag color="blue">{deviceInfo.protocol}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label={t('device.mode')}>
                <Tag color={deviceInfo.mode === 'native' ? 'green' : 'orange'}>
                  {deviceInfo.mode ?? '-'}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label={t('device.mtu')}>{deviceInfo.mtu ?? '-'}</Descriptions.Item>
              <Descriptions.Item label={t('device.version')}>{deviceInfo.version ?? '-'}</Descriptions.Item>
              {deviceInfo.fields.map((item) => (
                <Descriptions.Item key={item.key} label={t(`device.fields.${item.key}`, item.key)}>
                  {item.value}
                </Descriptions.Item>
              ))}
            </Descriptions>
          )}
          {preflighting && <Typography.Text type="secondary">{t('device.preflighting')}</Typography.Text>}
        </Space>
      </Card>

      <Card title={t('firmware.title')}>
        <Space direction="vertical" style={{ width: '100%' }}>
          <Space>
            <Button icon={<CloudUploadOutlined />} onClick={pickPackage} disabled={running} loading={validating}>
              {t('firmware.select')}
            </Button>
            {packageInfo && <Tag color="green">{t('firmware.valid')}</Tag>}
          </Space>
          <Typography.Text type="secondary">
            {isGoodix ? t('firmware.hintGoodix') : t('firmware.hint')}
          </Typography.Text>
          {validating && <Typography.Text type="secondary">{t('firmware.validating')}</Typography.Text>}
          {packageInfo && (
            <Descriptions size="small" column={1} bordered>
              <Descriptions.Item label={t('firmware.package')}>
                {packageInfo.fileName}（{(packageInfo.packageSize / 1024).toFixed(1)} KB）
              </Descriptions.Item>
              <Descriptions.Item label={t('firmware.image')}>
                {packageInfo.imageName}（{(packageInfo.imageSize / 1024).toFixed(1)} KB）
              </Descriptions.Item>
              <Descriptions.Item label={t('firmware.protocol')}>
                <Tag color="blue">{packageInfo.protocol}</Tag>
              </Descriptions.Item>
              {packageInfo.fields.map((item) => (
                <Descriptions.Item key={item.key} label={t(`firmware.fields.${item.key}`, item.key)}>
                  {item.value}
                </Descriptions.Item>
              ))}
              <Descriptions.Item label={t('firmware.sha256')}>
                <Typography.Text code copyable={{ text: packageInfo.imageSha256 }}>
                  {packageInfo.imageSha256.slice(0, 16)}…
                </Typography.Text>
              </Descriptions.Item>
            </Descriptions>
          )}
          {isGoodix ? (
            <Space direction="vertical" style={{ width: '100%' }} size={8}>
              <Typography.Text strong>{t('goodix.title')}</Typography.Text>
              <Typography.Text type="secondary">{t('goodix.route')}</Typography.Text>
              <Select
                style={{ width: '100%' }}
                value={route}
                disabled={running}
                onChange={setRoute}
                options={[
                  { value: 'auto', label: t('goodix.routeAuto') },
                  { value: 'normal', label: t('goodix.routeNormal') },
                  { value: 'fastService', label: t('goodix.routeFast') },
                ]}
              />
              <Typography.Text type="secondary">{t('goodix.transferMode')}</Typography.Text>
              <Select
                style={{ width: '100%' }}
                value={transferMode}
                disabled={running}
                onChange={setTransferMode}
                options={[
                  { value: 'auto', label: t('goodix.modeAuto') },
                  { value: 'normal', label: t('goodix.modeNormal') },
                  { value: 'fast', label: t('goodix.modeFast') },
                ]}
              />
              <Typography.Text type="secondary">{t('goodix.targetAddress')}</Typography.Text>
              <Input
                placeholder={t('goodix.targetPlaceholder')}
                value={targetAddress}
                disabled={running}
                onChange={(event) => setTargetAddress(event.target.value)}
              />
              <Space>
                <Switch checked={runImmediately} onChange={setRunImmediately} disabled={running} />
                <Typography.Text>{t('goodix.runImmediately')}</Typography.Text>
              </Space>
            </Space>
          ) : (
            <>
              <Space>
                <Switch checked={forceConfirm} onChange={setForceConfirm} disabled={running} />
                <Typography.Text>{t('options.forceConfirm')}</Typography.Text>
              </Space>
              <Space>
                <Switch checked={upgradeOnly} onChange={setUpgradeOnly} disabled={running} />
                <Typography.Text>{t('options.upgradeOnly')}</Typography.Text>
              </Space>
            </>
          )}
        </Space>
      </Card>

      <Card title={t('progress.title')}>
        <Steps
          size="small"
          current={STAGE_INDEX[status.stage]}
          status={status.state === 'failed' ? 'error' : status.state === 'success' ? 'finish' : running ? 'process' : 'wait'}
          items={[
            { title: t('stages.validate') },
            { title: t('stages.prepare') },
            { title: t('stages.transfer') },
            { title: t('stages.activate') },
            { title: t('stages.reboot') },
          ]}
        />
        <Progress
          style={{ marginTop: 16 }}
          percent={Math.round(status.percent)}
          status={status.state === 'failed' ? 'exception' : status.state === 'success' ? 'success' : running ? 'active' : 'normal'}
        />
        <Typography.Text type="secondary">{status.message}</Typography.Text>
        <Space style={{ marginTop: 16 }}>
          <Button
            type="primary"
            disabled={!targetConnected || !packageInfo || running || preflighting}
            onClick={startWithOptions}
          >
            {t('actions.start')}
          </Button>
          <Button danger disabled={!running} onClick={cancel}>
            {t('actions.cancel')}
          </Button>
        </Space>
      </Card>
    </Space>
  );
};
