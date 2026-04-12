import React, { useState, useEffect } from 'react';
import { Select, Switch, Input, Button, Space, Typography, message, Row, Col, theme } from 'antd';
import { FolderOpenOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { useSerialStore } from '../../stores/serialStore';
import { useBleStore } from '../../stores/bleStore';
import { open } from '@tauri-apps/plugin-dialog';

const { Text } = Typography;

const Gh3036ChannelConfig: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { token } = theme.useToken();
  const {
    txChannel,
    rxChannel,
    csvConfig,
    configureTxChannel,
    configureRxChannel,
    updateCsvConfig,
  } = useGh3036Store();
  const { ports } = useSerialStore();
  const { connections } = useBleStore();

  const [txType, setTxType] = useState<'serial' | 'ble'>(txChannel?.channel_type === 'Ble' ? 'ble' : 'serial');
  const [txDevice, setTxDevice] = useState(txChannel?.device_id || '');
  const [txChar, setTxChar] = useState(txChannel?.characteristic_uuid || '');

  const [rxType, setRxType] = useState<'serial' | 'ble'>(rxChannel?.channel_type === 'Ble' ? 'ble' : 'serial');
  const [rxDevice, setRxDevice] = useState(rxChannel?.device_id || '');
  const [rxChar, setRxChar] = useState(rxChannel?.characteristic_uuid || '');

  const [csvEnabled, setCsvEnabled] = useState(csvConfig.enabled);
  const [csvDir, setCsvDir] = useState(csvConfig.output_dir);

  useEffect(() => {
    if (txChannel) {
      setTxType(txChannel.channel_type === 'Ble' ? 'ble' : 'serial');
      setTxDevice(txChannel.device_id);
      setTxChar(txChannel.characteristic_uuid || '');
    }
  }, [txChannel]);

  useEffect(() => {
    if (rxChannel) {
      setRxType(rxChannel.channel_type === 'Ble' ? 'ble' : 'serial');
      setRxDevice(rxChannel.device_id);
      setRxChar(rxChannel.characteristic_uuid || '');
    }
  }, [rxChannel]);

  useEffect(() => {
    setCsvEnabled(csvConfig.enabled);
    setCsvDir(csvConfig.output_dir);
  }, [csvConfig]);

  const serialOptions = ports.map((p) => ({
    label: p.name,
    value: p.name,
  }));

  const bleOptions = connections.map((c) => ({
    label: c.name || c.address,
    value: c.address,
  }));

  const handleSaveTxChannel = async () => {
    if (!txDevice) {
      message.error(t('gh3036.selectDevice'));
      return;
    }
    const success = await configureTxChannel(txType, txDevice, txType === 'ble' ? txChar : undefined);
    if (success) {
      message.success(t('gh3036.txChannelSaved'));
    }
  };

  const handleSaveRxChannel = async () => {
    if (!rxDevice) {
      message.error(t('gh3036.selectDevice'));
      return;
    }
    const success = await configureRxChannel(rxType, rxDevice, rxType === 'ble' ? rxChar : undefined);
    if (success) {
      message.success(t('gh3036.rxChannelSaved'));
    }
  };

  const handleSelectCsvDir = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (selected && typeof selected === 'string') {
      setCsvDir(selected);
    }
  };

  const handleSaveCsvConfig = async () => {
    const success = await updateCsvConfig(csvEnabled, csvDir);
    if (success) {
      message.success(t('gh3036.csvConfigSaved'));
    }
  };

  const sectionStyle: React.CSSProperties = {
    padding: '8px 12px',
    background: token.colorFillSecondary,
    borderRadius: 4,
  };

  const labelStyle: React.CSSProperties = {
    fontSize: 12,
    fontWeight: 500,
    marginBottom: 6,
    color: token.colorTextSecondary,
  };

  return (
    <Row gutter={[12, 12]}>
      <Col span={8}>
        <div style={sectionStyle}>
          <div style={labelStyle}>{t('gh3036.txChannel')}</div>
          <Space direction="vertical" style={{ width: '100%' }} size={6}>
            <Select
              size="small"
              value={txType}
              onChange={setTxType}
              options={[
                { label: t('gh3036.serial'), value: 'serial' },
                { label: t('gh3036.ble'), value: 'ble' },
              ]}
              style={{ width: '100%' }}
            />
            <Select
              size="small"
              value={txDevice}
              onChange={setTxDevice}
              options={txType === 'serial' ? serialOptions : bleOptions}
              placeholder={t('gh3036.selectDevice')}
              style={{ width: '100%' }}
            />
            {txType === 'ble' && (
              <Input
                size="small"
                value={txChar}
                onChange={(e) => setTxChar(e.target.value)}
                placeholder={t('gh3036.charUuidPlaceholder')}
              />
            )}
            <Button size="small" type="primary" onClick={handleSaveTxChannel} block>
              {t('gh3036.saveTxChannel')}
            </Button>
          </Space>
        </div>
      </Col>

      <Col span={8}>
        <div style={sectionStyle}>
          <div style={labelStyle}>{t('gh3036.rxChannel')}</div>
          <Space direction="vertical" style={{ width: '100%' }} size={6}>
            <Select
              size="small"
              value={rxType}
              onChange={setRxType}
              options={[
                { label: t('gh3036.serial'), value: 'serial' },
                { label: t('gh3036.ble'), value: 'ble' },
              ]}
              style={{ width: '100%' }}
            />
            <Select
              size="small"
              value={rxDevice}
              onChange={setRxDevice}
              options={rxType === 'serial' ? serialOptions : bleOptions}
              placeholder={t('gh3036.selectDevice')}
              style={{ width: '100%' }}
            />
            {rxType === 'ble' && (
              <Input
                size="small"
                value={rxChar}
                onChange={(e) => setRxChar(e.target.value)}
                placeholder={t('gh3036.charUuidPlaceholder')}
              />
            )}
            <Button size="small" type="primary" onClick={handleSaveRxChannel} block>
              {t('gh3036.saveRxChannel')}
            </Button>
          </Space>
        </div>
      </Col>

      <Col span={8}>
        <div style={sectionStyle}>
          <div style={labelStyle}>{t('gh3036.csvSave')}</div>
          <Space direction="vertical" style={{ width: '100%' }} size={6}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <Text style={{ fontSize: 12 }}>{t('gh3036.enableCsv')}</Text>
              <Switch size="small" checked={csvEnabled} onChange={setCsvEnabled} />
            </div>
            <Space.Compact style={{ width: '100%' }}>
              <Input
                size="small"
                value={csvDir}
                onChange={(e) => setCsvDir(e.target.value)}
                placeholder={t('gh3036.outputDir')}
              />
              <Button size="small" icon={<FolderOpenOutlined />} onClick={handleSelectCsvDir} />
            </Space.Compact>
            <Button size="small" type="primary" onClick={handleSaveCsvConfig} block>
              {t('gh3036.saveCsvConfig')}
            </Button>
          </Space>
        </div>
      </Col>
    </Row>
  );
};

export default Gh3036ChannelConfig;
