import React, { useMemo, useState } from 'react';
import { Card, Col, Empty, InputNumber, Row, Select, Space, Tag, Tooltip, Button, Typography } from 'antd';
import {
  ApiOutlined,
  ClockCircleOutlined,
  DashboardOutlined,
  SettingOutlined,
  ThunderboltOutlined,
  ClearOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3220Store } from '../../stores/gh3220Store';
import { GH3220_SELECT_OPTIONS } from '../../constants/gh3220CommandSchema';
import {
  customOption,
  customValue,
  formatFunctionLabel,
  formatFunctionMaskNames,
  resolveGh3220Lang,
  type Gh3220Option,
} from '../../constants/gh3220Functions';
import MultiLineChart from '../Waveform/MultiLineChart';
import { buildChartGroups, buildWaveformChartData } from './monitorChartData';

const MonitorTab: React.FC = () => {
  const { t, i18n } = useTranslation('gh3220');
  const lang = resolveGh3220Lang(i18n.language);
  const {
    status,
    deviceInfo,
    stats,
    capture,
    frames,
    selectedFunctionId,
    dataMode,
    sampleRate,
    displayDurationSeconds,
    chartLegendSelected,
    setSelectedFunctionId,
    setDataMode,
    setSampleRate,
    setDisplayDurationSeconds,
    setChartLegendSelected,
    clearWaveformData,
  } = useGh3220Store();

  // V3：显示时长 / 采样率改常用档位下拉，选「自定义…」才展开数值输入
  const [customDuration, setCustomDuration] = useState<string | null>(null);
  const [customRate, setCustomRate] = useState<string | null>(null);

  const numericField = (
    value: number,
    options: Gh3220Option[],
    onSelect: (next: number) => void,
    customRaw: string | null,
    setCustomRaw: (next: string | null) => void,
    min: number,
    max: number
  ) => (
    <>
      <Select
        size="small"
        style={{ width: 100 }}
        value={customRaw === null ? String(value) : 'custom'}
        options={options}
        onChange={(next: string) => {
          if (next === 'custom') setCustomRaw(customOption(value));
          else {
            setCustomRaw(null);
            onSelect(Number(next));
          }
        }}
      />
      {customRaw !== null ? (
        <InputNumber
          size="small"
          min={min}
          max={max}
          value={Number(customValue(customRaw, String(value)))}
          onChange={(next) => {
            const resolved = next ?? min;
            setCustomRaw(customOption(resolved));
            onSelect(resolved);
          }}
          style={{ width: 76 }}
        />
      ) : null}
    </>
  );

  // U1：功能选择器显示「心率 HR (4 CH)」，未知 func 由语义层兜底为「功能 funcN」
  const functionOptions = useMemo(
    () =>
      Array.from(frames.entries()).map(([id, buffer]) => ({
        value: id,
        label: formatFunctionLabel(id, buffer.channelCount, lang),
      })),
    [frames, lang]
  );

  // U2：deviceInfo.openedFunc 是设备回读的已开启功能位（WORK_MODE 下发结果）
  const openedFunctions = useMemo(
    () => (deviceInfo ? formatFunctionMaskNames(deviceInfo.openedFunc, lang) : []),
    [deviceInfo, lang]
  );

  // 未选择功能时默认跟随第一个出现的功能
  const activeFunctionId = useMemo(() => {
    if (selectedFunctionId !== null && frames.has(selectedFunctionId)) {
      return selectedFunctionId;
    }
    const first = frames.keys().next();
    return first.done ? null : first.value;
  }, [selectedFunctionId, frames]);

  const currentBuffer = activeFunctionId === null ? null : frames.get(activeFunctionId) ?? null;

  const chartData = useMemo(
    () => buildWaveformChartData(currentBuffer, dataMode),
    [currentBuffer, dataMode]
  );

  const chartGroups = useMemo(
    () =>
      buildChartGroups(currentBuffer, dataMode, {
        mainGroup: dataMode === 'agc' ? t('monitor.agcGroup') : t('monitor.rawGroup'),
        accGroup: t('monitor.accGroup'),
      }),
    [currentBuffer, dataMode, t]
  );

  const stateTag = (
    <Tag color={status.state === 'disconnected' ? 'default' : status.state === 'capturing' ? 'warning' : 'success'}>
      {t(`monitor.state.${status.state}`)}
    </Tag>
  );

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', gap: 8, overflow: 'auto' }}>
      <Row gutter={8}>
        <Col xs={12} sm={6}>
          <Card size="small" title={<Space size={4}><ApiOutlined />{t('monitor.connState')}</Space>}>
            {stateTag}
            <div style={{ marginTop: 4 }}>
              <Tag>{status.link ? status.link.toUpperCase() : t('monitor.idle')}</Tag>
            </div>
          </Card>
        </Col>
        <Col xs={12} sm={6}>
          <Card size="small" title={<Space size={4}><ThunderboltOutlined />{t('monitor.captureState')}</Space>}>
            <Tag color={capture.active ? 'warning' : 'default'}>
              {capture.active ? t('monitor.active') : t('monitor.idle')}
            </Tag>
          </Card>
        </Col>
        <Col xs={12} sm={6}>
          <Card size="small" title={<Space size={4}><DashboardOutlined />{t('monitor.recordCount')}</Space>}>
            <span style={{ fontSize: 20, fontWeight: 500 }}>{capture.records}</span>
          </Card>
        </Col>
        <Col xs={12} sm={6}>
          <Card size="small" title={<Space size={4}><DashboardOutlined />{t('monitor.qualityTitle')}</Space>}>
            <Space size={8} wrap>
              <Tag color={stats.crcErrors > 0 ? 'error' : 'success'}>{`CRC ${stats.crcErrors}`}</Tag>
              <Tag color={stats.lostFrames > 0 ? 'error' : 'success'}>{`LOST ${stats.lostFrames}`}</Tag>
              <Tag>{`RESYNC ${stats.resyncs}`}</Tag>
            </Space>
          </Card>
        </Col>
      </Row>

      <Card
        size="small"
        title={t('monitor.openedFunctions')}
        style={{ flex: '0 0 auto' }}
        styles={{ body: { padding: 8 } }}
      >
        {openedFunctions.length > 0 ? (
          <Space size={[4, 4]} wrap>
            {openedFunctions.map((name) => (
              <Tag key={name} color="blue">
                {name}
              </Tag>
            ))}
          </Space>
        ) : (
          <Typography.Text type="secondary">{t('monitor.noOpenedFunctions')}</Typography.Text>
        )}
      </Card>

      <Card
        size="small"
        title={
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span>{t('monitor.dataTitle')}</span>
            <Space>
              <Tooltip title={t('monitor.displayDurationTooltip')}>
                <Space size={4}>
                  <ClockCircleOutlined />
                  {numericField(
                    displayDurationSeconds,
                    GH3220_SELECT_OPTIONS.displayDuration(lang),
                    setDisplayDurationSeconds,
                    customDuration,
                    setCustomDuration,
                    1,
                    60
                  )}
                </Space>
              </Tooltip>
              <Tooltip title={t('monitor.sampleRateTooltip')}>
                <Space size={4}>
                  <SettingOutlined />
                  {numericField(
                    sampleRate,
                    GH3220_SELECT_OPTIONS.sampleRate(lang),
                    setSampleRate,
                    customRate,
                    setCustomRate,
                    1,
                    1000
                  )}
                </Space>
              </Tooltip>
              <Select
                size="small"
                style={{ width: 90 }}
                value={dataMode}
                onChange={setDataMode}
                options={[
                  { value: 'raw', label: t('monitor.rawMode') },
                  { value: 'agc', label: t('monitor.agcMode') },
                ]}
              />
              <Select
                size="small"
                style={{ width: 200 }}
                value={activeFunctionId ?? undefined}
                onChange={(value) => setSelectedFunctionId(value)}
                options={functionOptions}
                placeholder={t('monitor.selectFunction')}
              />
              <Button size="small" icon={<ClearOutlined />} onClick={clearWaveformData}>
                {t('monitor.clearData')}
              </Button>
            </Space>
          </div>
        }
        style={{ flex: '0 0 auto' }}
        styles={{ body: { padding: 8, height: 510 } }}
      >
        {chartData.columns.length > 0 && chartData.rows.length > 0 ? (
          <MultiLineChart
            columns={chartData.columns}
            rows={chartData.rows}
            chartGroups={chartGroups}
            sampleRate={sampleRate}
            legendScope="gh3220"
            legendSelected={chartLegendSelected}
            onLegendSelectedChange={setChartLegendSelected}
          />
        ) : (
          <Empty description={t('monitor.noData')} style={{ marginTop: 200 }} />
        )}
      </Card>
    </div>
  );
};

export default MonitorTab;
