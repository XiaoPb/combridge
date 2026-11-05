import React, { useEffect, useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Divider,
  Input,
  InputNumber,
  Space,
  Switch,
  Table,
  Typography,
  message,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useGh3220Store } from '../../stores/gh3220Store';
import { useWaveformStore } from '../../stores/waveformStore';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { parseFuncMask } from '../../types/gh3220';
import type { Gh3220Record } from '../../types/gh3220';
import { formatErrorMessage } from '../../utils/errorMessage';

const CaptureTab: React.FC = () => {
  const { t } = useTranslation('gh3220');
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

  const [autoStart, setAutoStart] = useState(true);
  const [mode, setMode] = useState(0);
  const [slotEn, setSlotEn] = useState(0);
  const [funcMaskText, setFuncMaskText] = useState('hr');
  const [channelsOverride, setChannelsOverride] = useState<number | null>(null);

  useEffect(() => {
    void refreshCapture();
    void refreshStats();
  }, [refreshCapture, refreshStats]);

  const handleStart = async () => {
    let mask = 0;
    try {
      mask = parseFuncMask(funcMaskText);
    } catch (error) {
      message.error((error as Error).message);
      return;
    }
    if (autoStart && mask === 0) {
      message.error(t('capture.maskRequired'));
      return;
    }
    try {
      await startCapture({
        autoStart,
        mode,
        slotEn,
        funcMask: mask,
        channelsOverride,
      });
      message.success(t('capture.started'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('capture.startFailed')));
    }
  };

  const handleStop = async () => {
    await stopCapture();
    message.info(t('capture.stopped'));
  };

  const columns: ColumnsType<Gh3220Record> = [
    { title: t('capture.table.func'), dataIndex: 'functionId', width: 70 },
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
            <Switch checked={autoStart} onChange={setAutoStart} />
          </Space>
          <Space>
            <Typography.Text>{t('control.mode')}</Typography.Text>
            <InputNumber min={0} max={255} value={mode} onChange={(v) => setMode(v ?? 0)} />
          </Space>
          <Space>
            <Typography.Text>{t('control.slotEn')}</Typography.Text>
            <InputNumber min={0} max={255} value={slotEn} onChange={(v) => setSlotEn(v ?? 0)} />
          </Space>
          <Space>
            <Typography.Text>{t('control.funcMask')}</Typography.Text>
            <Input
              style={{ width: 160 }}
              value={funcMaskText}
              onChange={(e) => setFuncMaskText(e.target.value)}
              placeholder="hr,spo2"
            />
          </Space>
          <Space>
            <Typography.Text>{t('capture.channels')}</Typography.Text>
            <InputNumber
              min={1}
              max={32}
              value={channelsOverride}
              onChange={(v) => setChannelsOverride(v ?? null)}
              placeholder={t('capture.auto')}
            />
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
                {t('capture.openWaveform', { func: functionId })}
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
          scroll={{ x: 760 }}
          locale={{ emptyText: t('capture.empty') }}
        />
      </Card>
    </Space>
  );
};

export default CaptureTab;
