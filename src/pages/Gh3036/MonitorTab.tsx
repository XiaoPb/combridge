import React, { useMemo, useState, useEffect, useRef } from 'react';
import { Card, Row, Col, Empty, Select, Space, Button, InputNumber, Tooltip } from 'antd';
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

const DEFAULT_SAMPLE_RATE = 25;

const FUNCTION_ID_TO_NAME: Record<number, string> = {
  0: 'ADT',
  1: 'HR',
  2: 'SPO2',
  3: 'HRV',
  4: 'GNADT',
};

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
    gsensorData,
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
    sharedDataZoomState,
    setSharedDataZoomState,
  } = useGh3036Store();

  const [hrRefDialogOpen, setHrRefDialogOpen] = useState(false);
  const [hrRefMonitoring, setHrRefMonitoring] = useState(false);
  const [hrRefCollectedCount, setHrRefCollectedCount] = useState(0);
  const [hrRefCurrentValue, setHrRefCurrentValue] = useState<number | null>(null);
  const [spo2RefValue, setSpo2RefValue] = useState<number>(95);

  useEffect(() => {
    const unlisten = listen<{ value: number }>('spo2-ref-updated', (event) => {
      console.debug('[MonitorTab] 收到血氧金标更新:', event.payload.value);
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
    if (selectedFunctionId === null) return DEFAULT_SAMPLE_RATE;
    return sampleRateConfig[selectedFunctionId] ?? DEFAULT_SAMPLE_RATE_CONFIG[selectedFunctionId] ?? DEFAULT_SAMPLE_RATE;
  }, [selectedFunctionId, sampleRateConfig]);

  const handleSampleRateChange = (value: number | null) => {
    if (selectedFunctionId === null || value === null) return;
    setSampleRateConfig({
      ...sampleRateConfig,
      [selectedFunctionId]: value,
    });
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

    const currentGsensor = selectedFunctionId ? gsensorData.get(selectedFunctionId) : null;
    if (!currentGsensor) {
      return { columns, rows };
    }

    const len = Math.min(
      currentGsensor.acc_x.length,
      currentGsensor.acc_y.length,
      currentGsensor.acc_z.length
    );

    for (let i = 0; i < len; i++) {
      rows.push([
        currentGsensor.acc_x[i],
        currentGsensor.acc_y[i],
        currentGsensor.acc_z[i],
      ]);
    }

    return { columns, rows };
  }, [gsensorData, selectedFunctionId]);

  const ipdPaChartGroups = useMemo(() => {
    if (!currentFrames || currentFrames.channel_count === 0) return [];
    
    const columns: string[] = [];
    for (let i = 0; i < Math.min(currentFrames.channel_count, 4); i++) {
      columns.push(`CH${i}`);
    }
    
    return [{
      name: ipdRawDataType === 'ipd' ? t('monitor.ipdPaChart') : t('monitor.rawdataChart'),
      columns,
      height: 250,
    }];
  }, [currentFrames, t, ipdRawDataType]);

  const gsensorChartGroups = useMemo(() => {
    return [{
      name: t('monitor.gsensorChart'),
      columns: ['ACC_X', 'ACC_Y', 'ACC_Z'],
      height: 200,
    }];
  }, [t]);

  const initialDataZoomState = useMemo(() => {
    if (!ipdPaChartData.rows.length) {
      return { start: 0, end: 100 };
    }

    const sampleRateValue = sampleRate;
    const totalPoints = ipdPaChartData.rows.length;
    const displayPoints = displayDurationSeconds * sampleRateValue;

    console.log('[MonitorTab] 计算初始 dataZoom 状态:', {
      totalPoints,
      sampleRate: sampleRateValue,
      displayDurationSeconds,
      displayPoints,
    });

    if (totalPoints <= displayPoints) {
      console.log('[MonitorTab] 数据量小于显示量，显示全部数据');
      return { start: 0, end: 100 };
    }

    const endPercent = (displayPoints / totalPoints) * 100;
    const result = { start: 0, end: endPercent };
    console.log('[MonitorTab] 最终 dataZoom 状态:', result);
    return result;
  }, [ipdPaChartData.rows.length, sampleRate, displayDurationSeconds]);

  // 使用 ref 跟踪是否已初始化，避免重复设置
  const isDataZoomInitializedRef = useRef(false);

  useEffect(() => {
    console.log('[MonitorTab] useEffect 触发:', {
      rowsLength: ipdPaChartData.rows.length,
      sharedDataZoomState,
      initialDataZoomState,
      isInitialized: isDataZoomInitializedRef.current,
    });
    
    // 只在数据首次到达时设置初始状态
    if (ipdPaChartData.rows.length > 0 && !isDataZoomInitializedRef.current) {
      console.log('[MonitorTab] 首次初始化 sharedDataZoomState:', initialDataZoomState);
      setSharedDataZoomState(initialDataZoomState);
      isDataZoomInitializedRef.current = true;
    }
  }, [ipdPaChartData.rows.length, initialDataZoomState, setSharedDataZoomState]);

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', gap: 8, overflow: 'auto' }}>
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
        title={ipdRawDataType === 'ipd' ? t('monitor.ipdPaChart') : t('monitor.rawdataChart')}
        extra={
          <Space>
            <Tooltip title={t('monitor.displayDurationHint')}>
              <Space size={4}>
                <ClockCircleOutlined style={{ color: 'var(--text-secondary)' }} />
                <InputNumber
                  size="small"
                  min={1}
                  max={60}
                  value={displayDurationSeconds}
                  onChange={(value) => setDisplayDurationSeconds(value ?? 10)}
                  style={{ width: 60 }}
                  addonAfter={t('monitor.seconds')}
                />
              </Space>
            </Tooltip>
            <Tooltip title={t('monitor.sampleRateHint')}>
              <Space size={4}>
                <SettingOutlined style={{ color: 'var(--text-secondary)' }} />
                <span style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
                  {selectedFunctionId !== null ? FUNCTION_ID_TO_NAME[selectedFunctionId] : ''}:
                </span>
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
        }
        style={{ flex: '0 0 auto' }}
        styles={{ body: { padding: 8, height: 280 } }}
      >
        {ipdPaChartData.columns.length > 0 && ipdPaChartData.rows.length > 0 ? (
          <MultiLineChart
            columns={ipdPaChartData.columns}
            rows={ipdPaChartData.rows}
            chartGroups={ipdPaChartGroups}
            sampleRate={sampleRate}
            initialDataZoom={sharedDataZoomState}
            onDataZoomChange={setSharedDataZoomState}
          />
        ) : (
          <Empty description={t('monitor.noData')} style={{ marginTop: 80 }} />
        )}
      </Card>

      <Card
        size="small"
        title={t('monitor.gsensorChart')}
        style={{ flex: '0 0 auto' }}
        styles={{ body: { padding: 8, height: 230 } }}
      >
        {gsensorChartData.rows.length > 0 ? (
          <MultiLineChart
            columns={gsensorChartData.columns}
            rows={gsensorChartData.rows}
            chartGroups={gsensorChartGroups}
            sampleRate={DEFAULT_SAMPLE_RATE}
            initialDataZoom={sharedDataZoomState}
            onDataZoomChange={setSharedDataZoomState}
          />
        ) : (
          <Empty description={t('monitor.noGsensorData')} style={{ marginTop: 60 }} />
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
