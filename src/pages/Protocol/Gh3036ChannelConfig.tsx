import React, { useState, useEffect } from 'react';
import { Select, Switch, Input, Button, Space, Typography, message, Row, Col, theme, Divider } from 'antd';
import { FolderOpenOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { useBleStore } from '../../stores/bleStore';
import { useConnectedDevices } from '../../hooks/useConnectedDevices';
import { open } from '@tauri-apps/plugin-dialog';
import type { BleCharacteristic } from '../../types';

const { Text } = Typography;

const Gh3036ChannelConfig: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { token } = theme.useToken();
  const {
    channelConfig,
    csvConfig,
    loadChannelConfig,
    updateChannelConfig,
    configureTxChannel,
    configureRxChannel,
    updateCsvConfig,
  } = useGh3036Store();
  const currentBleDevice = useBleStore((state) => state.currentDevice);
  const bleDeviceTabs = useBleStore((state) => state.deviceTabs);
  const connectedDevices = useConnectedDevices();

  const [csvEnabled, setCsvEnabled] = useState(csvConfig.enabled);
  const [csvDir, setCsvDir] = useState(csvConfig.output_dir);

  useEffect(() => {
    loadChannelConfig();
  }, [loadChannelConfig]);

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

  const preferCharacteristic = (
    characteristics: BleCharacteristic[],
    preferredUuidPart: string,
    predicate: (char: BleCharacteristic) => boolean
  ) => {
    const candidates = characteristics.filter(predicate);
    return (
      candidates.find((char) => char.uuid.toLowerCase().includes(preferredUuidPart)) ??
      candidates[0]
    );
  };

  useEffect(() => {
    if (!currentBleDevice || channelConfig.connectionType !== 'ble') return;

    const tab = bleDeviceTabs[currentBleDevice];
    const characteristics = tab?.characteristics ?? [];
    const txChar = preferCharacteristic(
      characteristics,
      '00000004',
      (char) => char.properties.write || char.properties.writeWithoutResponse
    );
    const rxChar = preferCharacteristic(
      characteristics,
      '00000003',
      (char) => char.properties.notify || char.properties.indicate
    );

    const nextConfig = {
      bleDevice: currentBleDevice,
      txChar: txChar?.uuid ?? channelConfig.txChar,
      rxChar: rxChar?.uuid ?? channelConfig.rxChar,
    };

    if (
      nextConfig.bleDevice !== channelConfig.bleDevice ||
      nextConfig.txChar !== channelConfig.txChar ||
      nextConfig.rxChar !== channelConfig.rxChar
    ) {
      void updateChannelConfig(nextConfig);
    }
  }, [
    currentBleDevice,
    bleDeviceTabs,
    channelConfig.connectionType,
    channelConfig.bleDevice,
    channelConfig.txChar,
    channelConfig.rxChar,
    updateChannelConfig,
  ]);

  const handleSaveSerialChannel = async () => {
    if (!channelConfig.serialPort) {
      message.error(t('gh3036.selectSerialPort'));
      return;
    }
    
    const txSuccess = await configureTxChannel('serial', channelConfig.serialPort);
    if (!txSuccess) return;
    
    const rxSuccess = await configureRxChannel('serial', channelConfig.serialPort);
    if (rxSuccess) {
      await updateChannelConfig({ connectionType: 'serial' });
      message.success(t('gh3036.channelSaved'));
    }
  };

  const handleSaveBleChannel = async () => {
    if (!channelConfig.bleDevice) {
      message.error(t('gh3036.selectBleDevice'));
      return;
    }
    if (!channelConfig.txChar) {
      message.error(t('gh3036.inputTxUuid'));
      return;
    }
    if (!channelConfig.rxChar) {
      message.error(t('gh3036.inputRxUuid'));
      return;
    }
    
    const txSuccess = await configureTxChannel('ble', channelConfig.bleDevice, channelConfig.txChar);
    if (!txSuccess) return;
    
    const rxSuccess = await configureRxChannel('ble', channelConfig.bleDevice, channelConfig.rxChar);
    if (rxSuccess) {
      await updateChannelConfig({ connectionType: 'ble' });
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
          <Space orientation="vertical" style={{ width: '100%' }} size={12}>
            <Select
              size="small"
              value={channelConfig.connectionType}
              onChange={(value) => updateChannelConfig({ connectionType: value })}
              options={[
                { label: t('gh3036.serialConnection'), value: 'serial' },
                { label: t('gh3036.bleConnection'), value: 'ble' },
              ]}
              style={{ width: '100%' }}
            />
            
            {channelConfig.connectionType === 'serial' ? (
              <Space orientation="vertical" style={{ width: '100%' }} size={8}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t('gh3036.serialHint')}
                </Text>
                <Select
                  size="small"
                  value={channelConfig.serialPort}
                  onChange={(value) => updateChannelConfig({ serialPort: value })}
                  options={serialOptions}
                  placeholder={t('gh3036.selectSerialPort')}
                  style={{ width: '100%' }}
                />
                <Button size="small" type="primary" onClick={handleSaveSerialChannel} block>
                  {t('gh3036.saveChannel')}
                </Button>
              </Space>
            ) : (
              <Space orientation="vertical" style={{ width: '100%' }} size={8}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t('gh3036.bleHint')}
                </Text>
                <Select
                  size="small"
                  value={channelConfig.bleDevice}
                  onChange={(value) => updateChannelConfig({ bleDevice: value })}
                  options={bleOptions}
                  placeholder={t('gh3036.selectBleDevice')}
                  style={{ width: '100%' }}
                />
                <Divider style={{ margin: '4px 0' }} />
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Text style={{ fontSize: 12, width: 60 }}>{t('gh3036.txUuid')}</Text>
                  <Input
                    size="small"
                    value={channelConfig.txChar}
                    onChange={(e) => updateChannelConfig({ txChar: e.target.value })}
                    placeholder={t('gh3036.charUuidPlaceholder')}
                    style={{ flex: 1 }}
                  />
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Text style={{ fontSize: 12, width: 60 }}>{t('gh3036.rxUuid')}</Text>
                  <Input
                    size="small"
                    value={channelConfig.rxChar}
                    onChange={(e) => updateChannelConfig({ rxChar: e.target.value })}
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
          <Space orientation="vertical" style={{ width: '100%' }} size={8}>
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
