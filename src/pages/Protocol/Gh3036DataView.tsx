import React, { useMemo, useCallback, useEffect } from 'react';
import { Button, Empty, Card, Select, Space, Row, Col, message } from 'antd';
import { ClearOutlined, FileAddOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { gh3036ForceNewCsvFile } from '../../api/gh3036';
import MultiLineChart from '../Waveform/MultiLineChart';
import { getChartGroupKey } from '../Waveform/chartGroup';
import {
  appendGh3036ChartGroup,
  normalizeGh3036ChartGroups,
} from './gh3036ChartGroup';

const Gh3036DataView: React.FC = () => {
  const { t } = useTranslation('protocol');
  const {
    framesData,
    selectedFunctionId,
    chartGroups,
    clearWaveformData,
    setSelectedFunctionId,
    setChartGroups,
    chartLegendSelected,
    setChartLegendSelected,
  } = useGh3036Store();

  const chartGroupsWithStableIds = useMemo(
    () => normalizeGh3036ChartGroups(chartGroups),
    [chartGroups],
  );

  useEffect(() => {
    const needsNormalization = chartGroups.some(
      (group, index) => group.id !== chartGroupsWithStableIds[index]?.id,
    );
    if (needsNormalization) {
      setChartGroups(chartGroupsWithStableIds);
    }
  }, [chartGroups, chartGroupsWithStableIds, setChartGroups]);

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

  const availableColumns = useMemo(() => {
    if (!currentFrames) return [];
    return Array.from(
      { length: currentFrames.channel_count },
      (_, i) => `CH${i}`,
    );
  }, [currentFrames]);

  const handleColumnSelect = useCallback(
    (chartIndex: number, columnIndex: number, column: string | null) => {
      let newGroups = [...chartGroupsWithStableIds];
      while (newGroups.length <= chartIndex) {
        newGroups = appendGh3036ChartGroup(
          newGroups,
          `图表 ${newGroups.length + 1}`,
        );
      }

      const group = { ...newGroups[chartIndex] };
      const newColumns = [...group.columns];

      if (column === null) {
        if (newColumns.length > columnIndex) {
          newColumns.splice(columnIndex, 1);
        }
      } else {
        while (newColumns.length <= columnIndex) {
          newColumns.push('');
        }
        newColumns[columnIndex] = column;
      }

      group.columns = newColumns.filter((c) => c);
      newGroups[chartIndex] = group;

      setChartGroups(newGroups.filter((g) => g.columns.length > 0));
    },
    [chartGroupsWithStableIds, setChartGroups],
  );

  const handleForceNewCsvFile = useCallback(async () => {
    try {
      await gh3036ForceNewCsvFile();
      message.success(t('gh3036.newCsvFileCreated'));
    } catch (error) {
      message.error(t('gh3036.newCsvFileFailed'));
      console.error('[GH3036] 手动创建新CSV文件失败:', error);
    }
  }, [t]);

  const chartData = useMemo(() => {
    if (!currentFrames || chartGroupsWithStableIds.length === 0) {
      return [];
    }

    return chartGroupsWithStableIds.map((group) => {
      const columns = group.columns.slice(0, 4);
      const rows: number[][] = [];

      for (let frameIdx = 0; frameIdx < currentFrames.frame_count; frameIdx++) {
        const row: number[] = [];
        for (const col of columns) {
          const chIdx = parseInt(col.replace('CH', ''), 10);
          row.push(currentFrames.ipd_pa[chIdx]?.[frameIdx] ?? 0);
        }
        rows.push(row);
      }

      return { group, columns, rows };
    });
  }, [currentFrames, chartGroupsWithStableIds]);

  if (framesData.size === 0) {
    return (
      <Card
        size="small"
        title={t('gh3036.dataView')}
        style={{ height: '100%' }}
        styles={{
          body: {
            padding: 8,
            height: 'calc(100% - 40px)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          },
        }}
      >
        <Empty description={t('gh3036.noData')} />
      </Card>
    );
  }

  return (
    <Card
      size="small"
      title={t('gh3036.dataView')}
      extra={
        <Space>
          <Select
            size="small"
            style={{ width: 150 }}
            value={selectedFunctionId}
            onChange={setSelectedFunctionId}
            options={functionOptions}
            placeholder={t('gh3036.selectFunction')}
          />
          <Button
            size="small"
            icon={<FileAddOutlined />}
            onClick={handleForceNewCsvFile}
          >
            {t('gh3036.newCsvFile')}
          </Button>
          <Button
            size="small"
            icon={<ClearOutlined />}
            onClick={clearWaveformData}
          >
            {t('gh3036.clearData')}
          </Button>
        </Space>
      }
      style={{ height: '100%' }}
      styles={{
        body: { padding: 8, height: 'calc(100% - 40px)', overflow: 'auto' },
      }}
    >
      <Space orientation="vertical" style={{ width: '100%' }} size="small">
        <Row gutter={8}>
          {[0, 1].map((chartIndex) => {
            const controlGroup = chartGroupsWithStableIds[chartIndex];
            const controlKey = controlGroup
              ? getChartGroupKey(controlGroup, chartIndex)
              : `gh3036-data-slot-${chartIndex}`;
            return (
              <Col span={12} key={controlKey}>
                <Card
                  size="small"
                  title={`图表 ${chartIndex + 1}`}
                  style={{ marginBottom: 8 }}
                >
                  <Space size="small" wrap>
                    {[0, 1, 2, 3].map((lineIndex) => (
                      <Select
                        key={lineIndex}
                        size="small"
                        style={{ width: 80 }}
                        value={
                          chartGroupsWithStableIds[chartIndex]?.columns[
                            lineIndex
                          ] ?? null
                        }
                        onChange={(value) =>
                          handleColumnSelect(chartIndex, lineIndex, value)
                        }
                        options={availableColumns.map((col) => ({
                          value: col,
                          label: col,
                        }))}
                        placeholder={`线${lineIndex + 1}`}
                        allowClear
                      />
                    ))}
                  </Space>
                </Card>
              </Col>
            );
          })}
        </Row>

        {chartData.map((data, index) => (
          <div
            key={getChartGroupKey(data.group, index)}
            style={{
              height: 200,
              border: '1px solid #d9d9d9',
              borderRadius: 4,
              marginBottom: 8,
            }}
          >
            {data.columns.length > 0 ? (
              <MultiLineChart
                columns={data.columns}
                rows={data.rows}
                chartGroups={[{ ...data.group, columns: data.columns }]}
                legendScope="gh3036-data"
                legendSelected={chartLegendSelected}
                onLegendSelectedChange={setChartLegendSelected}
              />
            ) : (
              <Empty
                description={`请选择图表 ${index + 1} 的通道`}
                style={{ marginTop: 60 }}
              />
            )}
          </div>
        ))}
      </Space>
    </Card>
  );
};

export default Gh3036DataView;
