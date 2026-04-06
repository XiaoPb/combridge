import React, { useMemo } from 'react';
import { Card, Slider, Select, Checkbox, Space, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { useCsvChartStore } from '../../stores/csvChartStore';

const { Text } = Typography;

interface ChartSidebarProps {
  columns: string[];
  totalRows: number;
}

const ChartSidebar: React.FC<ChartSidebarProps> = ({ columns, totalRows }) => {
  const { t } = useTranslation('waveform');

  const {
    chart1Columns,
    chart2Columns,
    xAxisRange,
    hiddenLines,
    setChart1Columns,
    setChart2Columns,
    setXAxisRange,
    toggleLineVisibility,
  } = useCsvChartStore();

  const maxRange = Math.max(0, totalRows - 1);

  const visibleColumns = useMemo(() => {
    return columns.filter(col => !hiddenLines.includes(col));
  }, [columns, hiddenLines]);

  const handleLineVisibilityChange = (checkedValues: string[]) => {
    const newHidden = columns.filter(col => !checkedValues.includes(col));
    newHidden.forEach(col => {
      if (!hiddenLines.includes(col)) {
        toggleLineVisibility(col);
      }
    });
    hiddenLines.forEach(col => {
      if (!newHidden.includes(col)) {
        toggleLineVisibility(col);
      }
    });
  };

  return (
    <div style={{ width: 280, height: '100%', overflow: 'auto' }}>
      <Space orientation="vertical" style={{ width: '100%' }} size="middle">
        <Card size="small" title={t('sidebar.xAxisRange')} styles={{ body: { padding: 12 } }}>
          <Space orientation="vertical" style={{ width: '100%' }}>
            <Slider
              range
              min={0}
              max={maxRange}
              value={xAxisRange}
              onChange={(value) => setXAxisRange(value as [number, number])}
              disabled={totalRows === 0}
            />
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t('sidebar.rangeDisplay', { start: xAxisRange[0], end: xAxisRange[1] })}
            </Text>
          </Space>
        </Card>

        <Card size="small" title={t('sidebar.chart1Columns')} styles={{ body: { padding: 12 } }}>
          <Select
            mode="multiple"
            allowClear
            style={{ width: '100%' }}
            placeholder={t('sidebar.selectColumns')}
            value={chart1Columns}
            onChange={setChart1Columns}
            options={columns.map(col => ({ label: col, value: col }))}
            maxTagCount="responsive"
          />
        </Card>

        <Card size="small" title={t('sidebar.chart2Columns')} styles={{ body: { padding: 12 } }}>
          <Select
            mode="multiple"
            allowClear
            style={{ width: '100%' }}
            placeholder={t('sidebar.selectColumns')}
            value={chart2Columns}
            onChange={setChart2Columns}
            options={columns.map(col => ({ label: col, value: col }))}
            maxTagCount="responsive"
          />
        </Card>

        <Card size="small" title={t('sidebar.lineVisibility')} styles={{ body: { padding: 12 } }}>
          <Checkbox.Group
            value={visibleColumns}
            onChange={handleLineVisibilityChange as (checkedValue: (string | number | boolean)[]) => void}
            style={{ width: '100%' }}
          >
            <Space orientation="vertical" style={{ width: '100%' }}>
              {columns.map(col => (
                <Checkbox key={col} value={col}>
                  {col}
                </Checkbox>
              ))}
            </Space>
          </Checkbox.Group>
          {columns.length === 0 && (
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t('sidebar.noColumns')}
            </Text>
          )}
        </Card>
      </Space>
    </div>
  );
};

export default ChartSidebar;
