import React, { useState } from 'react';
import { Alert, Button, Card, Input, Space, Typography, message } from 'antd';
import { useTranslation } from 'react-i18next';
import { gh3220Api } from '../../api/gh3220';
import { useGh3220Store } from '../../stores/gh3220Store';
import { formatErrorMessage } from '../../utils/errorMessage';
import DebugChannelCard from './DebugChannelCard';

const DebugTab: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const { status } = useGh3220Store();

  const [rawHex, setRawHex] = useState('');
  const [testHex, setTestHex] = useState('0102');
  const [packageResult, setPackageResult] = useState<string>('');

  const connected = status.state !== 'disconnected' && status.state !== 'connecting';

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

      <DebugChannelCard />

      <Card size="small" title={t('debug.rawTitle')}>
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

      <Card size="small" title={t('debug.packageTitle')}>
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
