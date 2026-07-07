import React, { useMemo, useState, useEffect } from 'react';
import { Card, Row, Col, Empty, Select, Space, Button, InputNumber, Tooltip } from 'antd';
import { ClearOutlined, HeartOutlined, ThunderboltOutlined, SettingOutlined } from '@ant-design/icons';
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

const DISPLAY_DURATION_SECONDS = 6;
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
      sampleRate,
      ipdRawDataType,
      DISPLAY_DURATION_SECONDS
    );
  }, [currentFrames, sampleRate, ipdRawDataType]);

  const gsensorChartData = useMemo(() => {
    const columns = ['ACC_X', 'ACC_Y', 'ACC_Z'];
    const rows: number[][] = [];

    const maxPoints = DISPLAY_DURATION_SECONDS * DEFAULT_SAMPLE_RATE;
    const len = Math.min(
      gsensorData.acc_x.length,
      gsensorData.acc_y.length,
      gsensorData.acc_z.length,
      maxPoints
    );

    const startIndex = Math.max(0, Math.min(
      gsensorData.acc_x.length,
      gsensorData.acc_y.length,
      gsensorData.acc_z.length
    ) - maxPoints);

    for (let i = startIndex; i < startIndex + len; i++) {
      rows.push([
        gsensorData.acc_x[i],
        gsensorData.acc_y[i],
        gsensorData.acc_z[i],
      ]);
    }

    return { columns, rows };
  }, [gsensorData]);

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
