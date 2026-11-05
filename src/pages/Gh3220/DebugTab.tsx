import React, { useEffect, useState } from 'react';
import { Alert, Button, Card, Input, Space, Switch, Typography, message } from 'antd';
import { useTranslation } from 'react-i18next';
import { gh3220Api } from '../../api/gh3220';
import { useGh3220Store } from '../../stores/gh3220Store';
import { formatErrorMessage } from '../../utils/errorMessage';

const DebugTab: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const { status, preferences, connect, disconnect, loadPreferences, savePreferences } =
    useGh3220Store();

  const [enabled, setEnabled] = useState(false);
  const [address, setAddress] = useState('127.0.0.1:9000');
  const [rawHex, setRawHex] = useState('');
  const [testHex, setTestHex] = useState('0102');
  const [packageResult, setPackageResult] = useState<string>('');

  useEffect(() => {
    void loadPreferences();
  }, [loadPreferences]);

  useEffect(() => {
    if (!preferences) return;
    setEnabled(preferences.debugTcp.enabled);
    setAddress(preferences.debugTcp.address);
  }, [preferences]);

  const connected = status.state !== 'disconnected' && status.state !== 'connecting';

  const handleSave = async () => {
    if (!preferences) return;
    try {
      await savePreferences({
        ...preferences,
        debugTcp: { enabled, address },
      });
      message.success(t('debug.saved'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('debug.saveFailed')));
    }
  };

  const handleConnect = async () => {
    try {
      await connect({ kind: 'tcp', address });
      message.success(t('debug.connected'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('debug.connectFailed')));
    }
  };

  const handleRaw = async () => {
    try {
      await gh3220Api.sendRaw(rawHex);
      message.success(t('debug.rawSent'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('debug.rawFailed')));
    }
  };

  const handlePackageTest = async () => {
    try {
      const payload = await gh3220Api.packageTest(testHex);
      setPackageResult(payload.map((v) => v.toString(16).padStart(2, '0')).join(' '));
    } catch (error) {
      message.error(formatErrorMessage(error, t('debug.packageFailed')));
    }
  };

  if (!import.meta.env.DEV) {
    return <Alert type="info" showIcon message={t('debug.devOnly')} />;
  }

  return (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
      <Alert type="warning" showIcon message={t('debug.warning')} />

      <Card title={t('debug.channelTitle')}>
        <Space wrap size="middle">
          <Space>
            <Typography.Text>{t('debug.enable')}</Typography.Text>
            <Switch checked={enabled} onChange={setEnabled} />
          </Space>
          <Space>
            <Typography.Text>{t('debug.address')}</Typography.Text>
            <Input
              style={{ width: 220 }}
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder="127.0.0.1:9000"
            />
          </Space>
          <Button onClick={handleSave} disabled={!preferences}>
            {t('debug.save')}
          </Button>
          <Button type="primary" disabled={!enabled || connected} onClick={handleConnect}>
            {t('debug.connect')}
          </Button>
          <Button danger disabled={!connected} onClick={() => void disconnect()}>
            {t('debug.disconnect')}
          </Button>
        </Space>
      </Card>

      <Card title={t('debug.rawTitle')}>
        <Space.Compact style={{ width: '100%' }}>
          <Input
            value={rawHex}
            onChange={(e) => setRawHex(e.target.value)}
            placeholder="AA11A00100BF"
          />
          <Button onClick={handleRaw} disabled={!connected}>
            {t('debug.send')}
          </Button>
        </Space.Compact>
      </Card>

      <Card title={t('debug.packageTitle')}>
        <Space wrap>
          <Input
            style={{ width: 220 }}
            value={testHex}
            onChange={(e) => setTestHex(e.target.value)}
            placeholder="0102"
          />
          <Button onClick={handlePackageTest} disabled={!connected}>
            {t('debug.send')}
          </Button>
          <Typography.Text>{packageResult || '--'}</Typography.Text>
        </Space>
      </Card>
    </Space>
  );
};

export default DebugTab;
