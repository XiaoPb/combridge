import React, { useState, useEffect } from 'react';
import { Select, Switch, Input, Button, Space, Typography, message, Row, Col, theme, Divider } from 'antd';
import { FolderOpenOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { useConnectedDevices } from '../../hooks/useConnectedDevices';
import { open } from '@tauri-apps/plugin-dialog';

const { Text } = Typography;

const DEFAULT_TX_UUID = '00000004-0000-1000-8000-00805f9b34fb';
const DEFAULT_RX_UUID = '00000003-0000-1000-8000-00805f9b34fb';

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
  const connectedDevices = useConnectedDevices();

  const [connectionType, setConnectionType] = useState<'serial' | 'ble'>(
    txChannel?.channel_type === 'Ble' || rxChannel?.channel_type === 'Ble' ? 'ble' : 'serial'
  );
  
  const [serialPort, setSerialPort] = useState<string>('');
  
  const [bleDevice, setBleDevice] = useState<string>('');
  const [txChar, setTxChar] = useState(txChannel?.characteristic_uuid || DEFAULT_TX_UUID);
  const [rxChar, setRxChar] = useState(rxChannel?.characteristic_uuid || DEFAULT_RX_UUID);

  const [csvEnabled, setCsvEnabled] = useState(csvConfig.enabled);
  const [csvDir, setCsvDir] = useState(csvConfig.output_dir);

  useEffect(() => {
    if (txChannel?.channel_type === 'Serial' && rxChannel?.channel_type === 'Serial') {
      if (txChannel.device_id === rxChannel.device_id) {
        setSerialPort(txChannel.device_id);
      }
    }
    if (txChannel?.channel_type === 'Ble' || rxChannel?.channel_type === 'Ble') {
      setConnectionType('ble');
      if (txChannel?.channel_type === 'Ble') {
        setBleDevice(txChannel.device_id);
        setTxChar(txChannel.characteristic_uuid || DEFAULT_TX_UUID);
      }
      if (rxChannel?.channel_type === 'Ble') {
        if (!txChannel || txChannel.channel_type !== 'Ble') {
          setBleDevice(rxChannel.device_id);
        }
        setRxChar(rxChannel.characteristic_uuid || DEFAULT_RX_UUID);
      }
    }
  }, [txChannel, rxChannel]);

  useEffect(() => {
    setCsvEnabled(csvConfig.enabled);
    setCsvDir(csvConfig.output_dir);
  }, [csvConfig]);

  const serialOptions = connectedDevices
    .filter((d) => d.type === 'serial')
    .map((d) => ({ label: d.name, value: d.id }));

  const bleOptions = connectedDevices
    .filter((d) => d.type === 'ble')
    .map((d) => ({ label: d.name, value: d.id }));

  const handleSaveSerialChannel = async () => {
    if (!serialPort) {
      message.error(t('gh3036.selectSerialPort'));
      return;
    }
    
    const txSuccess = await configureTxChannel('serial', serialPort);
    if (!txSuccess) return;
    
    const rxSuccess = await configureRxChannel('serial', serialPort);
    if (rxSuccess) {
      message.success(t('gh3036.channelSaved'));
    }
  };

  const handleSaveBleChannel = async () => {
    if (!bleDevice) {
      message.error(t('gh3036.selectBleDevice'));
      return;
    }
    if (!txChar) {
      message.error(t('gh3036.inputTxUuid'));
      return;
    }
    if (!rxChar) {
      message.error(t('gh3036.inputRxUuid'));
      return;
    }
    
    const txSuccess = await configureTxChannel('ble', bleDevice, txChar);
    if (!txSuccess) return;
    
    const rxSuccess = await configureRxChannel('ble', bleDevice, rxChar);
    if (rxSuccess) {
      message.success(t('gh3036.channelSaved'));
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
    padding: '12px 16px',
    background: token.colorFillSecondary,
    borderRadius: 8,
  };

  const labelStyle: React.CSSProperties = {
    fontSize: 13,
    fontWeight: 500,
    marginBottom: 8,
    color: token.colorTextSecondary,
  };

  return (
    <Row gutter={[12, 12]}>
      <Col span={16}>
        <div style={sectionStyle}>
          <div style={labelStyle}>{t('gh3036.connectionType')}</div>
          <Space direction="vertical" style={{ width: '100%' }} size={12}>
            <Select
              size="small"
              value={connectionType}
              onChange={setConnectionType}
              options={[
                { label: t('gh3036.serialConnection'), value: 'serial' },
                { label: t('gh3036.bleConnection'), value: 'ble' },
              ]}
              style={{ width: '100%' }}
            />
            
            {connectionType === 'serial' ? (
              <Space direction="vertical" style={{ width: '100%' }} size={8}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t('gh3036.serialHint')}
                </Text>
                <Select
                  size="small"
                  value={serialPort}
                  onChange={setSerialPort}
                  options={serialOptions}
                  placeholder={t('gh3036.selectSerialPort')}
                  style={{ width: '100%' }}
                />
                <Button size="small" type="primary" onClick={handleSaveSerialChannel} block>
                  {t('gh3036.saveChannel')}
                </Button>
              </Space>
            ) : (
              <Space direction="vertical" style={{ width: '100%' }} size={8}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t('gh3036.bleHint')}
                </Text>
                <Select
                  size="small"
                  value={bleDevice}
                  onChange={setBleDevice}
                  options={bleOptions}
                  placeholder={t('gh3036.selectBleDevice')}
                  style={{ width: '100%' }}
                />
                <Divider style={{ margin: '4px 0' }} />
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Text style={{ fontSize: 12, width: 60 }}>{t('gh3036.txUuid')}</Text>
                  <Input
                    size="small"
                    value={txChar}
                    onChange={(e) => setTxChar(e.target.value)}
                    placeholder={t('gh3036.charUuidPlaceholder')}
                    style={{ flex: 1 }}
                  />
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Text style={{ fontSize: 12, width: 60 }}>{t('gh3036.rxUuid')}</Text>
                  <Input
                    size="small"
                    value={rxChar}
                    onChange={(e) => setRxChar(e.target.value)}
                    placeholder={t('gh3036.charUuidPlaceholder')}
                    style={{ flex: 1 }}
                  />
                </div>
                <Button size="small" type="primary" onClick={handleSaveBleChannel} block>
                  {t('gh3036.saveChannel')}
                </Button>
              </Space>
            )}
          </Space>
        </div>
      </Col>

      <Col span={8}>
        <div style={sectionStyle}>
          <div style={labelStyle}>{t('gh3036.csvSave')}</div>
          <Space direction="vertical" style={{ width: '100%' }} size={8}>
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
