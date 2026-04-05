import React, { useState, useEffect } from 'react';
import { Form, Select, Switch, Input, Button, Space, Divider, Typography, message } from 'antd';
import { FolderOpenOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { useSerialStore } from '../../stores/serialStore';
import { useBleStore } from '../../stores/bleStore';
import { open } from '@tauri-apps/plugin-dialog';

const { Text } = Typography;

const Gh3036ChannelConfig: React.FC = () => {
  const { t } = useTranslation('protocol');
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

  return (
    <div>
      <Text strong style={{ fontSize: 12 }}>{t('gh3036.txChannel')}</Text>
      <Form size="small" layout="vertical" style={{ marginTop: 8 }}>
        <Form.Item label={t('gh3036.channelType')} style={{ marginBottom: 8 }}>
          <Select
            value={txType}
            onChange={setTxType}
            options={[
              { label: t('gh3036.serial'), value: 'serial' },
              { label: t('gh3036.ble'), value: 'ble' },
            ]}
          />
        </Form.Item>
        <Form.Item label={t('gh3036.device')} style={{ marginBottom: 8 }}>
          <Select
            value={txDevice}
            onChange={setTxDevice}
            options={txType === 'serial' ? serialOptions : bleOptions}
            placeholder={t('gh3036.selectDevice')}
          />
        </Form.Item>
        {txType === 'ble' && (
          <Form.Item label={t('gh3036.characteristic')} style={{ marginBottom: 8 }}>
            <Input
              value={txChar}
              onChange={(e) => setTxChar(e.target.value)}
              placeholder={t('gh3036.charUuidPlaceholder')}
            />
          </Form.Item>
        )}
        <Form.Item style={{ marginBottom: 0 }}>
          <Button type="primary" size="small" onClick={handleSaveTxChannel} block>
            {t('gh3036.saveTxChannel')}
          </Button>
        </Form.Item>
      </Form>

      <Divider style={{ margin: '12px 0' }} />

      <Text strong style={{ fontSize: 12 }}>{t('gh3036.rxChannel')}</Text>
      <Form size="small" layout="vertical" style={{ marginTop: 8 }}>
        <Form.Item label={t('gh3036.channelType')} style={{ marginBottom: 8 }}>
          <Select
            value={rxType}
            onChange={setRxType}
            options={[
              { label: t('gh3036.serial'), value: 'serial' },
              { label: t('gh3036.ble'), value: 'ble' },
            ]}
          />
        </Form.Item>
        <Form.Item label={t('gh3036.device')} style={{ marginBottom: 8 }}>
          <Select
            value={rxDevice}
            onChange={setRxDevice}
            options={rxType === 'serial' ? serialOptions : bleOptions}
            placeholder={t('gh3036.selectDevice')}
          />
        </Form.Item>
        {rxType === 'ble' && (
          <Form.Item label={t('gh3036.characteristic')} style={{ marginBottom: 8 }}>
            <Input
              value={rxChar}
              onChange={(e) => setRxChar(e.target.value)}
              placeholder={t('gh3036.charUuidPlaceholder')}
            />
          </Form.Item>
        )}
        <Form.Item style={{ marginBottom: 0 }}>
          <Button type="primary" size="small" onClick={handleSaveRxChannel} block>
            {t('gh3036.saveRxChannel')}
          </Button>
        </Form.Item>
      </Form>

      <Divider style={{ margin: '12px 0' }} />

      <Text strong style={{ fontSize: 12 }}>{t('gh3036.csvSave')}</Text>
      <Form size="small" layout="vertical" style={{ marginTop: 8 }}>
        <Form.Item label={t('gh3036.enableCsv')} style={{ marginBottom: 8 }}>
          <Switch checked={csvEnabled} onChange={setCsvEnabled} />
        </Form.Item>
        <Form.Item label={t('gh3036.outputDir')} style={{ marginBottom: 8 }}>
          <Space.Compact style={{ width: '100%' }}>
            <Input value={csvDir} onChange={(e) => setCsvDir(e.target.value)} />
            <Button icon={<FolderOpenOutlined />} onClick={handleSelectCsvDir} />
          </Space.Compact>
        </Form.Item>
        <Form.Item style={{ marginBottom: 0 }}>
          <Button type="primary" size="small" onClick={handleSaveCsvConfig} block>
            {t('gh3036.saveCsvConfig')}
          </Button>
        </Form.Item>
      </Form>
    </div>
  );
};

export default Gh3036ChannelConfig;
