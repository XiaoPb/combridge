import React, { useEffect, useMemo } from 'react';
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Divider,
  InputNumber,
  Modal,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Tooltip,
  Typography,
  message,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useGh3220Store } from '../../../stores/gh3220Store';
import { useGh3220CommandConfig } from '../../../stores/gh3220CommandConfig';
import { useWaveformStore } from '../../../stores/waveformStore';
import { usePageTabsStore } from '../../../stores/pageTabsStore';
import type { Gh3220Record } from '../../../types/gh3220';
import { formatErrorMessage } from '../../../utils/errorMessage';
import { encodeBoundedNumber } from '../../../constants/gh3220CommandSchema';
import {
  boundedOptions,
  channelOptions,
  customOption,
  customValue,
  encodeFunctionMask,
  formatFunctionMaskPreview,
  formatWorkModeLabel,
  functionOptions,
  getFunctionName,
  GH3220_WORK_MODE_LIST,
  isCustom,
  resolveGh3220Lang,
} from '../../../constants/gh3220Functions';

/** 调试页分区①：采集与 CSV（原 CaptureTab 主体） */
const CaptureSection: React.FC = () => {
  const { t, i18n } = useTranslation('gh3220');
  const lang = resolveGh3220Lang(i18n.language);
  const navigate = useNavigate();
  const { refreshBuffers, setCurrentBuffer, readData } = useWaveformStore();
  const { setWaveformActiveTab } = usePageTabsStore();
  const {
    status,
    capture,
    stats,
    records,
    loading,
    startCapture,
    stopCapture,
    refreshCapture,
    refreshStats,
    clearStats,
    clearRecords,
  } = useGh3220Store();
  const cfg = useGh3220CommandConfig();
  const mask = encodeFunctionMask(cfg.selectedFunctions);

  useEffect(() => {
    void refreshCapture();
    void refreshStats();
  }, [refreshCapture, refreshStats]);

  const handleStart = async () => {
    if (cfg.autoStart === 'true' && mask === 0) {
      message.error(t('capture.maskRequired'));
      return;
    }
    try {
      await startCapture({
        autoStart: cfg.autoStart === 'true',
        mode: Number(cfg.workMode),
        slotEn: encodeBoundedNumber(cfg.slotEn) ?? 0,
        funcMask: mask,
        channelsOverride: encodeBoundedNumber(cfg.channelsOverride),
      });
      message.success(t('capture.started'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('capture.startFailed')));
    }
  };

  const handleStop = () => {
    Modal.confirm({
      title: t('commands.confirmTitles.captureStop'),
      content: t('commands.confirmContents.captureStop'),
      okText: t('commands.confirm'),
      cancelText: t('commands.cancel'),
      okButtonProps: { danger: true },
      onOk: async () => {
        await stopCapture();
        message.info(t('capture.stopped'));
      },
    });
  };

  const numericSelect = (
    value: string,
    options: ReturnType<typeof boundedOptions>,
    onChange: (next: string) => void,
    min = 0,
    max = 255
  ) => (
    <>
      <Select
        size="small"
        style={{ width: 140 }}
        value={isCustom(value) ? 'custom' : value}
        options={options}
        onChange={(next: string) =>
          onChange(next === 'custom' ? customOption(customValue(value) || '0') : next)
        }
      />
      {isCustom(value) ? (
        <InputNumber
          size="small"
          min={min}
          max={max}
          value={Number(customValue(value, '0'))}
          onChange={(next) => onChange(customOption(next ?? 0))}
          style={{ width: 92 }}
        />
      ) : null}
    </>
  );

  const columns: ColumnsType<Gh3220Record> = [
    {
      title: t('capture.table.func'),
      dataIndex: 'functionId',
      width: 170,
      render: (value: number) => getFunctionName(value, lang),
    },
    { title: t('capture.table.frame'), dataIndex: 'frameId', width: 70 },
    {
      title: 'RAW',
      dataIndex: 'raw',
      render: (value: number[]) => value.join(', '),
    },
    {
      title: 'AGC',
      dataIndex: 'agc',
      render: (value: number[]) => value.join(', '),
    },
    {
      title: 'GS',
      dataIndex: 'gs',
      width: 140,
      render: (value: [number, number, number] | null | undefined) =>
        value ? value.join(', ') : '--',
    },
    { title: 'ts(ms)', dataIndex: 'tsMs', width: 110 },
  ];

  const connected = status.state !== 'disconnected';
  const waveformFunctions = useMemo(
    () => Array.from(new Set(records.map((record) => record.functionId))).sort((a, b) => a - b),
    [records]
  );

  const openWaveform = async (functionId: number) => {
    const bufferId = `gh3220:func${functionId}`;
    try {
      await refreshBuffers();
      setCurrentBuffer(bufferId);
      await readData(bufferId);
      // 确保落在实时波形页签（缓冲在 realtime 视图中展示）
      setWaveformActiveTab('realtime');
      navigate('/waveform');
    } catch (error) {
      message.error(formatErrorMessage(error, t('capture.openWaveformFailed')));
    }
  };

  return (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
      {!connected && <Alert type="warning" showIcon message={t('capture.notConnected')} />}

      <Card title={t('capture.title')}>
        <Space wrap size="middle">
          <Space>
            <Typography.Text>{t('capture.autoStart')}</Typography.Text>
            <Switch
              checked={cfg.autoStart === 'true'}
              onChange={(checked) => cfg.setConfig({ autoStart: String(checked) })}
            />
          </Space>
          <Space>
            <Typography.Text>{t('control.mode')}</Typography.Text>
            <Select
              size="small"
              style={{ width: 240 }}
              value={cfg.workMode}
              onChange={(next: string) => cfg.setConfig({ workMode: next })}
              options={GH3220_WORK_MODE_LIST.map((info) => ({
                value: String(info.mode),
                label: formatWorkModeLabel(info.mode, lang),
              }))}
            />
          </Space>
          <Space>
            <Typography.Text>{t('control.slotEn')}</Typography.Text>
            {numericSelect(cfg.slotEn, boundedOptions('slotEn', lang), (next) =>
              cfg.setConfig({ slotEn: next })
            )}
          </Space>
          <Space>
            <Typography.Text>{t('control.funcMask')}</Typography.Text>
            <Select
              size="small"
              mode="multiple"
              maxTagCount={2}
              style={{ minWidth: 220 }}
              value={cfg.selectedFunctions}
              onChange={(keys: string[]) => cfg.setSelectedFunctions(keys)}
              options={functionOptions(lang)}
              placeholder={t('commands.preview.selectFunctions')}
            />
            <Tooltip title={formatFunctionMaskPreview(mask, lang)}>
              <Tag color="blue">{formatFunctionMaskPreview(mask, lang)}</Tag>
            </Tooltip>
          </Space>
          <Space>
            <Typography.Text>{t('capture.channels')}</Typography.Text>
            {numericSelect(
              cfg.channelsOverride,
              channelOptions(cfg.selectedFunctions, lang),
              (next) => cfg.setConfig({ channelsOverride: next }),
              1,
              32
            )}
          </Space>
        </Space>

        <Divider />

        <Space>
          <Button
            type="primary"
            loading={loading}
            disabled={!connected || capture.active}
            onClick={handleStart}
          >
            {t('capture.start')}
          </Button>
          <Button danger disabled={!capture.active} onClick={handleStop}>
            {t('capture.stop')}
          </Button>
          <Button onClick={() => void refreshCapture()}>{t('capture.refresh')}</Button>
        </Space>

        <Descriptions size="small" column={2} bordered style={{ marginTop: 16 }}>
          <Descriptions.Item label={t('capture.active')}>
            {capture.active ? t('common.yes') : t('common.no')}
          </Descriptions.Item>
          <Descriptions.Item label={t('capture.records')}>{capture.records}</Descriptions.Item>
          <Descriptions.Item label={t('capture.csvEnabled')}>
            {capture.csvEnabled ? t('common.yes') : t('common.no')}
          </Descriptions.Item>
          <Descriptions.Item label={t('capture.channelsUsed')}>
            {capture.channelsOverride ?? t('capture.auto')}
          </Descriptions.Item>
          <Descriptions.Item label={t('capture.files')} span={2}>
            {capture.files.length > 0 ? (
              <Space direction="vertical" size={0}>
                {capture.files.map((file) => (
                  <Typography.Text key={file} copyable>
                    {file}
                  </Typography.Text>
                ))}
              </Space>
            ) : (
              '--'
            )}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Card
        title={t('capture.statsTitle')}
        extra={
          <Space>
            <Button size="small" onClick={() => void refreshStats()}>
              {t('capture.refresh')}
            </Button>
            <Button size="small" onClick={() => void clearStats()}>
              {t('capture.clearStats')}
            </Button>
          </Space>
        }
      >
        <Descriptions size="small" column={3} bordered>
          <Descriptions.Item label={t('capture.stats.frames')}>{stats.frames}</Descriptions.Item>
          <Descriptions.Item label={t('capture.stats.records')}>{stats.records}</Descriptions.Item>
          <Descriptions.Item label={t('capture.stats.packets')}>{stats.packets}</Descriptions.Item>
          <Descriptions.Item label={t('capture.stats.crc')}>{stats.crcErrors}</Descriptions.Item>
          <Descriptions.Item label={t('capture.stats.resync')}>{stats.resyncs}</Descriptions.Item>
          <Descriptions.Item label={t('capture.stats.badLen')}>{stats.badLen}</Descriptions.Item>
          <Descriptions.Item label={t('capture.stats.lost')}>{stats.lostFrames}</Descriptions.Item>
          <Descriptions.Item label={t('capture.stats.badPackets')}>
            {stats.badPackets}
          </Descriptions.Item>
          <Descriptions.Item label={t('capture.stats.splice')}>
            {stats.spliceFragments}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Card
        title={t('capture.latestTitle')}
        extra={
          <Space>
            {waveformFunctions.map((functionId) => (
              <Button
                key={functionId}
                size="small"
                onClick={() => void openWaveform(functionId)}
              >
                {t('capture.openWaveform', { name: getFunctionName(functionId, lang) })}
              </Button>
            ))}
            <Button size="small" onClick={clearRecords}>
              {t('capture.clearRecords')}
            </Button>
          </Space>
        }
      >
        <Table<Gh3220Record>
          size="small"
          rowKey={(row) => `${row.functionId}-${row.frameId}-${row.tsMs}`}
          columns={columns}
          dataSource={records}
          pagination={{ pageSize: 10, size: 'small' }}
          scroll={{ x: 860 }}
          locale={{ emptyText: t('capture.empty') }}
        />
      </Card>
    </Space>
  );
};

export default CaptureSection;
