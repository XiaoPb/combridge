import React, { useEffect, useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Input,
  Select,
  Space,
  Switch,
  Typography,
  message,
} from 'antd';
import { useTranslation } from 'react-i18next';
import { gh3220Api } from '../../../api/gh3220';
import { useGh3220Store } from '../../../stores/gh3220Store';
import { formatErrorMessage } from '../../../utils/errorMessage';
import { encodeHex, GH3220_SELECT_OPTIONS } from '../../../constants/gh3220CommandSchema';
import {
  customOption,
  customValue,
  formatBitNames,
  getFunctionName,
  GH3220_HEX_TEMPLATES,
  GH3220_RAW_FRAME_TEMPLATES,
  isCustom,
  type Gh3220HexTemplate,
  parseFrameHeader,
  resolveGh3220Lang,
} from '../../../constants/gh3220Functions';

/**
 * 调试页分区④：TCP 调试通道 + 原始帧 / 回显测试 + 0x0B 帧头解析
 * V3：地址改下拉预设 + 自定义，三类 HEX 输入附加模板下拉与格式校验。
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
    const saved = preferences.debugTcp.address;
    if (!saved) return;
    const presets = GH3220_SELECT_OPTIONS.tcpAddress(lang).map((item) => item.value);
    setAddress(presets.includes(saved) ? saved : customOption(saved));
  }, [preferences, lang]);

  const connected = status.state !== 'disconnected' && status.state !== 'connecting';
  const frameHeader = useMemo(() => parseFrameHeader(headerHex), [headerHex]);
  const hexByte = (value: number) => `0x${(value & 0xff).toString(16).padStart(2, '0')}`;
  const resolvedAddress = isCustom(address) ? customValue(address) : address;

  const validHex = (text: string): boolean => {
    const cleaned = encodeHex(text);
    return cleaned.length > 0 && cleaned.length % 2 === 0 && /^[0-9a-f]+$/i.test(cleaned);
  };

  /** HEX 模板下拉：sendRaw 用完整帧模板，帧头解析 / 回显用示例模板 */
  const templateSelect = (
    onPick: (hex: string) => void,
    templates: Gh3220HexTemplate[] = GH3220_HEX_TEMPLATES
  ) => (
    <Select
      size="small"
      style={{ width: 170 }}
      value={undefined}
      placeholder={t('debug.template')}
      options={templates.map((item) => ({
        value: item.key,
        label: lang === 'zh' ? item.nameZh : item.nameEn,
      }))}
      onChange={(key: string) => {
        const template = templates.find((item) => item.key === key);
        if (template) onPick(template.hex);
      }}
    />
  );

  const handleSave = async () => {
    if (!preferences) return;
    try {
      await savePreferences({
        ...preferences,
        debugTcp: { enabled, address: resolvedAddress },
      });
      message.success(t('debug.saved'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('debug.saveFailed')));
    }
  };

  const handleConnect = async () => {
    try {
      await connect({ kind: 'tcp', address: resolvedAddress });
      message.success(t('debug.connected'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('debug.connectFailed')));
    }
  };

  const handleRaw = async () => {
    if (!validHex(rawHex)) {
      message.error(t('debug.frameInvalid'));
      return;
    }
    try {
      await gh3220Api.sendRaw(rawHex);
      message.success(t('debug.rawSent'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('debug.rawFailed')));
    }
  };

  const handlePackageTest = async () => {
    if (!validHex(testHex)) {
      message.error(t('debug.frameInvalid'));
      return;
    }
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
        <Space.Compact style={{ width: '100%' }}>
          <Input
            value={headerHex}
            onChange={(e) => setHeaderHex(e.target.value)}
            addonBefore={t('debug.frameInput')}
            placeholder={t('debug.framePlaceholder')}
          />
          {templateSelect(setHeaderHex)}
        </Space.Compact>
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
            <Select
              size="small"
              style={{ width: 200 }}
              value={isCustom(address) ? 'custom' : address}
              options={GH3220_SELECT_OPTIONS.tcpAddress(lang)}
              onChange={(next: string) =>
                setAddress(next === 'custom' ? customOption(customValue(address) || '127.0.0.1:9000') : next)
              }
            />
            {isCustom(address) ? (
              <Input
                size="small"
                style={{ width: 180 }}
                value={customValue(address)}
                onChange={(e) => setAddress(customOption(e.target.value))}
                placeholder={t('channel.customValue')}
              />
            ) : null}
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
        <Space wrap>
          <Input
            style={{ width: 240 }}
            value={rawHex}
            onChange={(e) => setRawHex(e.target.value)}
            placeholder="AA11A00100BF"
          />
          {templateSelect(setRawHex, GH3220_RAW_FRAME_TEMPLATES)}
          <Button onClick={handleRaw} disabled={!connected}>
            {t('debug.send')}
          </Button>
        </Space>
      </Card>

      <Card size="small" title={t('debug.packageTitle')}>
        <Space wrap>
          <Input
            style={{ width: 220 }}
            value={testHex}
            onChange={(e) => setTestHex(e.target.value)}
            placeholder="0102"
          />
          {templateSelect(setTestHex)}
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
