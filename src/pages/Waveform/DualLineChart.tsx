import React, { useMemo, useCallback, memo } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Legend,
  ResponsiveContainer,
} from 'recharts';

interface DualLineChartProps {
  columns: string[];
  rows: number[][];
  chart1Columns: string[];
  chart2Columns: string[];
  xAxisRange: [number, number];
  hiddenLines: string[];
}

const COLORS = [
  '#1890ff',
  '#52c41a',
  '#faad14',
  '#f5222d',
];

const MAX_LINES_PER_CHART = 4;
const SAMPLING_THRESHOLD = 2000;

const getColorIndex = (columns: string[], col: string): number => {
  const index = columns.indexOf(col);
  return index >= 0 ? index : 0;
};

const sampleData = <T,>(data: T[], maxPoints: number): T[] => {
  if (data.length <= maxPoints) return data;
  const step = Math.ceil(data.length / maxPoints);
  const sampled: T[] = [];
  for (let i = 0; i < data.length; i += step) {
    sampled.push(data[i]);
  }
  return sampled;
};

interface ChartRendererProps {
  displayData: Record<string, number | string>[];
  chartColumns: string[];
  columns: string[];
  showXAxis: boolean;
  chartId: number;
}

const ChartRenderer: React.FC<ChartRendererProps> = memo(({
  displayData,
  chartColumns,
  columns,
  showXAxis,
  chartId,
}) => {
  const limitedColumns = chartColumns.slice(0, MAX_LINES_PER_CHART);
  
  const yAxisConfigs = useMemo(() => {
    return limitedColumns.map((col, index) => ({
      col,
      yAxisId: `yAxis-${chartId}-${index}`,
      orientation: (index % 2 === 0 ? 'left' : 'right') as 'left' | 'right',
    }));
  }, [limitedColumns, chartId]);

  const leftYAxes = yAxisConfigs.filter(c => c.orientation === 'left');
  const rightYAxes = yAxisConfigs.filter(c => c.orientation === 'right');

  const sampledData = useMemo(() => 
    sampleData(displayData, SAMPLING_THRESHOLD),
    [displayData]
  );

  const renderLegend = useCallback((value: string) => (
    <span style={{ color: 'var(--text-primary)', cursor: 'default' }}>
      {value}
    </span>
  ), []);

  if (limitedColumns.length === 0) {
    return (
      <div
        style={{
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--text-secondary)',
        }}
      >
        暂无曲线
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={sampledData}
        margin={{ top: 5, right: 30 + rightYAxes.length * 40, left: 20 + leftYAxes.length * 40, bottom: 5 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="var(--border-color)" />
        {showXAxis && (
          <XAxis
            dataKey="index"
            stroke="var(--text-secondary)"
            tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
          />
        )}
        {leftYAxes.map(({ yAxisId }) => (
          <YAxis
            key={yAxisId}
            yAxisId={yAxisId}
            orientation="left"
            domain={['auto', 'auto']}
            stroke="var(--text-secondary)"
            tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
            width={60}
          />
        ))}
        {rightYAxes.map(({ yAxisId }) => (
          <YAxis
            key={yAxisId}
            yAxisId={yAxisId}
            orientation="right"
            domain={['auto', 'auto']}
            stroke="var(--text-secondary)"
            tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
            width={60}
          />
        ))}
        <Legend
          onClick={() => {}}
          formatter={renderLegend}
        />
        {limitedColumns.map((col) => {
          const config = yAxisConfigs.find((c) => c.col === col);
          const colorIndex = getColorIndex(columns, col);
          return (
            <Line
              key={col}
              type="monotone"
              dataKey={col}
              stroke={COLORS[colorIndex % COLORS.length]}
              strokeWidth={1.5}
              dot={false}
              isAnimationActive={false}
              yAxisId={config?.yAxisId || `yAxis-${chartId}-0`}
            />
          );
        })}
      </LineChart>
    </ResponsiveContainer>
  );
});

ChartRenderer.displayName = 'ChartRenderer';

const DualLineChart: React.FC<DualLineChartProps> = ({
  columns,
  rows,
  chart1Columns,
  chart2Columns,
  xAxisRange,
  hiddenLines,
}) => {
  const [minX, maxX] = xAxisRange;
  
  const displayData = useMemo(() => {
    const slicedRows = rows.slice(minX, maxX + 1);
    return slicedRows.map((row, index) => {
      const point: Record<string, number | string> = { index: minX + index };
      columns.forEach((col, colIndex) => {
        if (colIndex < row.length) {
          point[col] = row[colIndex];
        }
      });
      return point;
    });
  }, [rows, minX, maxX, columns]);

  const visibleChart1Columns = useMemo(
    () => chart1Columns.filter((col) => !hiddenLines.includes(col)),
    [chart1Columns, hiddenLines]
  );
  
  const visibleChart2Columns = useMemo(
    () => chart2Columns.filter((col) => !hiddenLines.includes(col)),
    [chart2Columns, hiddenLines]
  );

  if (columns.length === 0 || rows.length === 0) {
    return (
      <div
        style={{
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--text-secondary)',
        }}
      >
        暂无数据
      </div>
    );
  }

  return (
    <div style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ width: '100%', height: '50%' }}>
        <ChartRenderer
          displayData={displayData}
          chartColumns={visibleChart1Columns}
          columns={columns}
          showXAxis={false}
          chartId={1}
        />
      </div>
      <div style={{ width: '100%', height: '50%' }}>
        <ChartRenderer
          displayData={displayData}
          chartColumns={visibleChart2Columns}
          columns={columns}
          showXAxis={true}
          chartId={2}
        />
      </div>
    </div>
  );
};

export default memo(DualLineChart);
