import React, { useEffect, useState } from 'react';
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
import { useGh3220Store } from '../../stores/gh3220Store';
import { parseFuncMask } from '../../types/gh3220';
import type { Gh3220Record } from '../../types/gh3220';
import { formatErrorMessage } from '../../utils/errorMessage';

const CaptureTab: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const {
    status,
    capture,
    stats,
    records,
    preferences,
    loading,
    startCapture,
    stopCapture,
    refreshCapture,
    refreshStats,
    clearStats,
    clearRecords,
    loadPreferences,
    savePreferences,
  } = useGh3220Store();

  const [autoStart, setAutoStart] = useState(true);
  const [autoStop, setAutoStop] = useState(false);
  const [mode, setMode] = useState(0);
  const [slotEn, setSlotEn] = useState(0);
  const [funcMaskText, setFuncMaskText] = useState('hr');
  const [channelsOverride, setChannelsOverride] = useState<number | null>(null);
  const [csvEnabled, setCsvEnabled] = useState(true);
  const [outputDir, setOutputDir] = useState('');
  const [idleCloseSecs, setIdleCloseSecs] = useState(10);

  useEffect(() => {
    void loadPreferences();
    void refreshCapture();
    void refreshStats();
  }, [loadPreferences, refreshCapture, refreshStats]);

  useEffect(() => {
    if (!preferences) return;
    setCsvEnabled(preferences.csv.enabled);
    setOutputDir(preferences.csv.outputDir);
    setIdleCloseSecs(preferences.csv.idleCloseSecs);
    setChannelsOverride(preferences.capture.channelsOverride);
    setAutoStop(preferences.capture.autoStop);
  }, [preferences]);

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
        autoStop,
        mode,
        slotEn,
        funcMask: mask,
        channelsOverride,
        csvEnabled,
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

  const handleSavePreferences = async () => {
    if (!preferences) return;
    try {
      await savePreferences({
        csv: {
          enabled: csvEnabled,
          outputDir,
          idleCloseSecs,
        },
        capture: {
          channelsOverride,
          autoStop,
        },
        debugTcp: preferences.debugTcp,
      });
      message.success(t('capture.preferencesSaved'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('capture.preferencesSaveFailed')));
    }
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
            <Typography.Text>{t('capture.autoStop')}</Typography.Text>
            <Switch checked={autoStop} onChange={setAutoStop} />
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

      <Card title={t('capture.csvTitle')}>
        <Space wrap size="middle">
          <Space>
            <Typography.Text>{t('capture.csvEnabled')}</Typography.Text>
            <Switch checked={csvEnabled} onChange={setCsvEnabled} />
          </Space>
          <Space>
            <Typography.Text>{t('capture.outputDir')}</Typography.Text>
            <Input
              style={{ width: 320 }}
              value={outputDir}
              onChange={(e) => setOutputDir(e.target.value)}
            />
          </Space>
          <Space>
            <Typography.Text>{t('capture.idleClose')}</Typography.Text>
            <InputNumber
              min={0}
              max={3600}
              value={idleCloseSecs}
              onChange={(v) => setIdleCloseSecs(v ?? 0)}
            />
          </Space>
          <Button onClick={handleSavePreferences} disabled={!preferences}>
            {t('capture.savePreferences')}
          </Button>
        </Space>
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
          <Button size="small" onClick={clearRecords}>
            {t('capture.clearRecords')}
          </Button>
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
