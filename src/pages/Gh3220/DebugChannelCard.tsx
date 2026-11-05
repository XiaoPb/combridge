import React, { useEffect, useState } from 'react';
import { Button, Card, Input, Space, Switch, Typography, message } from 'antd';
import { useTranslation } from 'react-i18next';
import { useGh3220Store } from '../../stores/gh3220Store';
import { formatErrorMessage } from '../../utils/errorMessage';

/** TCP 调试通道配置卡片，ConfigTab 与 DebugTab 共用（仅 DEV 构建使用） */
const DebugChannelCard: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const { status, preferences, connect, disconnect, loadPreferences, savePreferences } =
    useGh3220Store();

  const [enabled, setEnabled] = useState(false);
  const [address, setAddress] = useState('127.0.0.1:9000');

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

  return (
    <Card size="small" title={t('debug.channelTitle')}>
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
  );
};

export default DebugChannelCard;
