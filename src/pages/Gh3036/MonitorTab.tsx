import React, { useMemo, useState, useEffect } from 'react';
import { Card, Row, Col, Empty, Select, Space, Button, InputNumber, Tooltip, Alert } from 'antd';
import { ClearOutlined, HeartOutlined, ThunderboltOutlined, SettingOutlined, ClockCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { useGh3036Store } from '../../stores/gh3036Store';
import VitalSignCard from './components/VitalSignCard';
import HrvCard from './components/HrvCard';
import StatusCombinedCard from './components/StatusCombinedCard';
import MultiLineChart from '../Waveform/MultiLineChart';
import HrRefDeviceDialog from './components/HrRefDeviceDialog';
import { gh3036Api } from '../../api/gh3036';
import { openSpo2RefWindow } from '../../utils/spo2RefWindow';
import { buildIpdPaChartData } from './monitorChartData';

const DEFAULT_SAMPLE_RATE_CONFIG: Record<number, number> = {
  0: 5,
  1: 25,
  2: 25,
  3: 25,
  4: 25,
};

const MonitorTab: React.FC = () => {
  const { t } = useTranslation('gh3036');
  const {
    framesData,
    vitalSigns,
    selectedFunctionId,
    clearWaveformData,
    setSelectedFunctionId,
    ipdRawDataType,
    setIpdRawDataType,
    sampleRateConfig,
    setSampleRateConfig,
    displayDurationSeconds,
    setDisplayDurationSeconds,
    setMaxFramesCount,
    chartLegendSelected,
    setChartLegendSelected,
    streamStalled,
    rpcConfig,
  } = useGh3036Store();

  const [hrRefDialogOpen, setHrRefDialogOpen] = useState(false);
  const [hrRefMonitoring, setHrRefMonitoring] = useState(false);
  const [hrRefCollectedCount, setHrRefCollectedCount] = useState(0);
  const [hrRefCurrentValue, setHrRefCurrentValue] = useState<number | null>(null);
  const [spo2RefValue, setSpo2RefValue] = useState<number>(95);

  useEffect(() => {
    const unlisten = listen<{ value: number }>('spo2-ref-updated', (event) => {
      setSpo2RefValue(event.payload.value);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleHrRefDeviceSelect = async (deviceAddress: string) => {
    setHrRefMonitoring(true);
    setHrRefCollectedCount(0);
    setHrRefCurrentValue(null);
    try {
      await gh3036Api.startHrRefMonitor(deviceAddress);
    } catch (err) {
      setHrRefMonitoring(false);
      throw err;
    }
  };

  const sampleRate = useMemo(() => {
    if (selectedFunctionId === null) return 25;
    return sampleRateConfig[selectedFunctionId] ?? DEFAULT_SAMPLE_RATE_CONFIG[selectedFunctionId] ?? 25;
  }, [selectedFunctionId, sampleRateConfig]);

  const handleSampleRateChange = (value: number | null) => {
    if (selectedFunctionId === null || value === null) return;
    setSampleRateConfig({
      ...sampleRateConfig,
      [selectedFunctionId]: value,
    });
  };

  const handleDisplayDurationChange = (value: number | null) => {
    const seconds = value ?? 10;
    setDisplayDurationSeconds(seconds);

    // 计算新的缓存帧数：秒数 × 采样率 ÷ 10
    // FRAME_CACHE_MULTIPLIER = 10，所以 maxFramesCount = seconds × sampleRate ÷ 10
    const currentSampleRate = sampleRate;
    const newMaxCount = Math.ceil((seconds * currentSampleRate) / 10);

    // 更新 PPG 和 ACC 缓存大小（统一使用 framesData 缓存）
    setMaxFramesCount(newMaxCount);
  };

  const functionOptions = useMemo(() => {
    return Array.from(framesData.entries()).map(([id, frames]) => ({
      value: id,
      label: `${frames.function_name} (ID: ${id})`,
    }));
  }, [framesData]);

  const currentFrames = useMemo(() => {
    if (selectedFunctionId === null) return null;
    return framesData.get(selectedFunctionId) ?? null;
  }, [framesData, selectedFunctionId]);

  const ipdPaChartData = useMemo(() => {
    return buildIpdPaChartData(
      currentFrames,
      ipdRawDataType
    );
  }, [currentFrames, ipdRawDataType]);

  const gsensorChartData = useMemo(() => {
    const columns = ['ACC_X', 'ACC_Y', 'ACC_Z'];
    const rows: number[][] = [];

    if (!currentFrames) {
      return { columns, rows };
    }

    const len = Math.min(
      currentFrames.acc_x.length,
      currentFrames.acc_y.length,
      currentFrames.acc_z.length
    );

    for (let i = 0; i < len; i++) {
      rows.push([
        currentFrames.acc_x[i],
        currentFrames.acc_y[i],
        currentFrames.acc_z[i],
      ]);
    }

    return { columns, rows };
  }, [currentFrames]);

  const chartGroups = useMemo(() => {
    const groups = [];

    // PPG/IPD 数据图表组
    if (ipdPaChartData.columns.length > 0 && ipdPaChartData.rows.length > 0) {
      groups.push({
        id: 'gh3036-ipd',
        name: ipdRawDataType === 'ipd' ? t('monitor.ipdPaChart') : t('monitor.rawdataChart'),
        columns: ipdPaChartData.columns,
        height: 250,
      });
    }

    // ACC 数据图表组
    if (gsensorChartData.columns.length > 0 && gsensorChartData.rows.length > 0) {
      groups.push({
        id: 'gh3036-acc',
        name: t('monitor.gsensorChart'),
        columns: gsensorChartData.columns,
        height: 200,
      });
    }

    return groups;
  }, [ipdRawDataType, gsensorChartData, t, ipdPaChartData]);

  const allChartData = useMemo(() => {
    const columns: string[] = [];
    const rows: number[][] = [];

    const sources = [ipdPaChartData, gsensorChartData].filter(
      (source) => source.columns.length > 0 && source.rows.length > 0
    );

    for (const source of sources) {
      columns.push(...source.columns);
    }

    if (sources.length === 0) {
      return { columns, rows };
    }

    // 各数据源按最短的长度对齐，缺失部分不使用 0 填充，
    // 避免在图上绘制出虚假的恒定值平线
    const alignedRows = Math.min(...sources.map((source) => source.rows.length));

    for (let i = 0; i < alignedRows; i++) {
      const row: number[] = [];
      for (const source of sources) {
        row.push(...source.rows[i]);
      }
      rows.push(row);
    }

    return { columns, rows };
  }, [ipdPaChartData, gsensorChartData]);

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', gap: 8, overflow: 'auto' }}>
      {streamStalled && rpcConfig.isRunning && (
        <Alert
          type="warning"
          showIcon
          message={t('monitor.streamStalledTitle', '设备已停止发送数据')}
          description={t(
            'monitor.streamStalledDesc',
            '波形已定格，但连接仍在。可能是设备端测量被自动停止或采样管线异常，请检查设备后重新开始采样。'
          )}
        />
      )}
      <Row gutter={8}>
        <Col xs={12} sm={6}>
          <VitalSignCard
            title={t('monitor.hr')}
            value={vitalSigns.hr}
            unit="bpm"
            status="normal"
            icon={<HeartOutlined />}
            confidence={vitalSigns.hrConfidence}
            onConfig={() => setHrRefDialogOpen(true)}
            configLabel={t('monitor.hrRefConfig')}
          />
        </Col>
        <Col xs={12} sm={6}>
          <VitalSignCard
            title={t('monitor.spo2')}
            value={vitalSigns.spo2}
            unit="%"
            status="normal"
            icon={<ThunderboltOutlined />}
            confidence={vitalSigns.spo2Confidence}
            subValue={vitalSigns.spo2RValue}
            subLabel="R"
            onConfig={() => openSpo2RefWindow(spo2RefValue)}
            configLabel={t('monitor.spo2RefConfig')}
          />
        </Col>
        <Col xs={12} sm={6}>
          <HrvCard />
        </Col>
        <Col xs={12} sm={6}>
          <StatusCombinedCard />
        </Col>
      </Row>

      <Card
        size="small"
        title={
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span>{t('monitor.dataMonitor')}</span>
            <Space>
              <Tooltip title={t('monitor.displayDurationTooltip')}>
                <Space size={4}>
                  <ClockCircleOutlined />
                  <InputNumber
                    size="small"
                    min={1}
                    max={60}
                    value={displayDurationSeconds}
                    onChange={handleDisplayDurationChange}
                    style={{ width: 60 }}
                    addonAfter="s"
                    disabled={selectedFunctionId === null}
                  />
                </Space>
              </Tooltip>
              <Tooltip title={t('monitor.sampleRateTooltip')}>
                <Space size={4}>
                  <SettingOutlined />
                  <InputNumber
                    size="small"
                    min={1}
                    max={1000}
                    value={sampleRate}
                    onChange={handleSampleRateChange}
                    style={{ width: 70 }}
                    addonAfter="Hz"
                    disabled={selectedFunctionId === null}
                  />
                </Space>
              </Tooltip>
              <Select
                size="small"
                style={{ width: 100 }}
                value={ipdRawDataType}
                onChange={setIpdRawDataType}
                options={[
                  { value: 'ipd', label: t('monitor.ipd') },
                  { value: 'rawdata', label: t('monitor.rawdata') },
                ]}
              />
              <Select
                size="small"
                style={{ width: 150 }}
                value={selectedFunctionId}
                onChange={setSelectedFunctionId}
                options={functionOptions}
                placeholder={t('monitor.selectFunction')}
              />
              <Button
                size="small"
                icon={<ClearOutlined />}
                onClick={clearWaveformData}
              >
                {t('monitor.clearData')}
              </Button>
            </Space>
          </div>
        }
        style={{ flex: '0 0 auto' }}
        styles={{ body: { padding: 8, height: 510 } }}
      >
        {allChartData.columns.length > 0 && allChartData.rows.length > 0 ? (
          <MultiLineChart
            columns={allChartData.columns}
            rows={allChartData.rows}
            chartGroups={chartGroups}
            sampleRate={sampleRate}
            legendScope="gh3036"
            legendSelected={chartLegendSelected}
            onLegendSelectedChange={setChartLegendSelected}
          />
        ) : (
          <Empty description={t('monitor.noData')} style={{ marginTop: 200 }} />
        )}
      </Card>

      <HrRefDeviceDialog
        open={hrRefDialogOpen}
        onSelect={handleHrRefDeviceSelect}
        onCancel={() => {
          setHrRefDialogOpen(false);
          if (hrRefMonitoring) {
            gh3036Api.stopHrRefMonitor().catch(console.error);
            setHrRefMonitoring(false);
          }
        }}
        isMonitoring={hrRefMonitoring}
        currentHrValue={hrRefCurrentValue}
        collectedCount={hrRefCollectedCount}
      />
    </div>
  );
};

export default MonitorTab;
