import React, { useMemo } from 'react';
import { Card, Row, Col, Empty, Select, Space, Button } from 'antd';
import { ClearOutlined, HeartOutlined, ThunderboltOutlined, EyeOutlined, SafetyOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import VitalSignCard from './components/VitalSignCard';
import MultiLineChart from '../Waveform/MultiLineChart';

const MonitorTab: React.FC = () => {
  const { t } = useTranslation('gh3036');
  const {
    framesData,
    gsensorData,
    vitalSigns,
    selectedFunctionId,
    clearWaveformData,
    setSelectedFunctionId,
  } = useGh3036Store();

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
    if (!currentFrames || currentFrames.channel_count === 0) {
      return { columns: [] as string[], rows: [] as number[][] };
    }

    const columns: string[] = [];
    for (let i = 0; i < currentFrames.channel_count; i++) {
      columns.push(`CH${i}`);
    }

    const rows: number[][] = [];
    for (let frameIdx = 0; frameIdx < currentFrames.frame_count; frameIdx++) {
      const row: number[] = [];
      for (let chIdx = 0; chIdx < currentFrames.channel_count; chIdx++) {
        row.push(currentFrames.ipd_pa[chIdx]?.[frameIdx] ?? 0);
      }
      rows.push(row);
    }

    return { columns, rows };
  }, [currentFrames]);

  const gsensorChartData = useMemo(() => {
    const columns = ['ACC_X', 'ACC_Y', 'ACC_Z'];
    const rows: number[][] = [];

    const len = Math.min(
      gsensorData.acc_x.length,
      gsensorData.acc_y.length,
      gsensorData.acc_z.length
    );

    for (let i = 0; i < len; i++) {
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
      name: t('monitor.ipdPaChart'),
      columns,
      height: 250,
    }];
  }, [currentFrames, t]);

  const gsensorChartGroups = useMemo(() => {
    return [{
      name: t('monitor.gsensorChart'),
      columns: ['ACC_X', 'ACC_Y', 'ACC_Z'],
      height: 200,
    }];
  }, [t]);

  const getAdtStatus = (): 'normal' | 'success' | 'warning' => {
    if (vitalSigns.adt === null) return 'normal';
    return vitalSigns.adt === '佩戴' ? 'success' : 'warning';
  };

  const getGnadtStatus = (): 'normal' | 'success' | 'error' => {
    if (vitalSigns.gnadt === null) return 'normal';
    return vitalSigns.gnadt === '活体' ? 'success' : 'error';
  };

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
          />
        </Col>
        <Col xs={12} sm={6}>
          <VitalSignCard
            title={t('monitor.spo2')}
            value={vitalSigns.spo2}
            unit="%"
            status="normal"
            icon={<ThunderboltOutlined />}
          />
        </Col>
        <Col xs={12} sm={6}>
          <VitalSignCard
            title={t('monitor.adt')}
            value={vitalSigns.adt}
            status={getAdtStatus()}
            icon={<EyeOutlined />}
          />
        </Col>
        <Col xs={12} sm={6}>
          <VitalSignCard
            title={t('monitor.gnadt')}
            value={vitalSigns.gnadt}
            status={getGnadtStatus()}
            icon={<SafetyOutlined />}
          />
        </Col>
      </Row>

      <Card
        size="small"
        title={t('monitor.ipdPaChart')}
        extra={
          <Space>
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
          />
        ) : (
          <Empty description={t('monitor.noGsensorData')} style={{ marginTop: 60 }} />
        )}
      </Card>
    </div>
  );
};

export default MonitorTab;
