import React, { useEffect, useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Input,
  Space,
  Switch,
  Typography,
  message,
} from 'antd';
import { useTranslation } from 'react-i18next';
import { gh3220Api } from '../../../api/gh3220';
import { useGh3220Store } from '../../../stores/gh3220Store';
import { formatErrorMessage } from '../../../utils/errorMessage';
import {
  formatBitNames,
  getFunctionName,
  parseFrameHeader,
  resolveGh3220Lang,
} from '../../../constants/gh3220Functions';

/**
 * 调试页分区④：TCP 调试通道 + 原始帧 / 回显测试 + 0x0B 帧头解析
 * （原 ConfigTab 的 DebugChannelCard 与独立 DebugTab 合并而来）
 */
const ChannelSection: React.FC = () => {
  const { t, i18n } = useTranslation('gh3220');
  const lang = resolveGh3220Lang(i18n.language);
  const { status, preferences, connect, disconnect, loadPreferences, savePreferences } =
    useGh3220Store();

  const [enabled, setEnabled] = useState(false);
  const [address, setAddress] = useState('127.0.0.1:9000');
  const [rawHex, setRawHex] = useState('');
  const [testHex, setTestHex] = useState('0102');
  const [packageResult, setPackageResult] = useState<string>('');
  const [headerHex, setHeaderHex] = useState('');

  useEffect(() => {
    void loadPreferences();
  }, [loadPreferences]);

  useEffect(() => {
    if (!preferences) return;
    setEnabled(preferences.debugTcp.enabled);
    setAddress(preferences.debugTcp.address);
  }, [preferences]);

  const connected = status.state !== 'disconnected' && status.state !== 'connecting';
  const frameHeader = useMemo(() => parseFrameHeader(headerHex), [headerHex]);
  const hexByte = (value: number) => `0x${(value & 0xff).toString(16).padStart(2, '0')}`;

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

  // U7：把 0x0B 帧头的 data_type / package_type 位解析成可读名称，仅本地展示
  const frameInspector = (
    <Card size="small" title={t('debug.frameTitle')}>
      <Space direction="vertical" size="small" style={{ width: '100%' }}>
        <Input
          value={headerHex}
          onChange={(e) => setHeaderHex(e.target.value)}
          addonBefore={t('debug.frameInput')}
          placeholder={t('debug.framePlaceholder')}
        />
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {t('debug.frameHint')}
        </Typography.Text>
        {headerHex.trim() && !frameHeader ? (
          <Alert type="warning" showIcon message={t('debug.frameInvalid')} />
        ) : null}
        {frameHeader ? (
          <Descriptions size="small" column={2} bordered>
            <Descriptions.Item label={t('debug.frameFunction')}>
              {getFunctionName(frameHeader.functionId, lang)}
            </Descriptions.Item>
            <Descriptions.Item label={t('debug.frameDataLen')}>
              {frameHeader.dataLen}
            </Descriptions.Item>
            <Descriptions.Item label={t('debug.frameDataType')}>
              {`${hexByte(frameHeader.dataType)} · ${formatBitNames(frameHeader.dataTypeItems, lang)}`}
            </Descriptions.Item>
            <Descriptions.Item label={t('debug.framePackageType')}>
              {`${hexByte(frameHeader.packageType)} · ${formatBitNames(frameHeader.packageTypeItems, lang)}`}
            </Descriptions.Item>
            <Descriptions.Item label={t('debug.frameChannelMask')} span={2}>
              {`0x${(frameHeader.channelMask >>> 0).toString(16).padStart(8, '0')} · ${frameHeader.channelCount} CH`}
            </Descriptions.Item>
          </Descriptions>
        ) : null}
      </Space>
    </Card>
  );

  if (!import.meta.env.DEV) {
    return (
      <Space direction="vertical" size="middle" style={{ width: '100%' }}>
        <Alert type="info" showIcon message={t('debug.devOnly')} />
        {frameInspector}
      </Space>
    );
  }

  return (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
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

      {frameInspector}
    </Space>
  );
};

export default ChannelSection;
