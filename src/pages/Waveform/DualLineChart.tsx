import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Legend,
  ResponsiveContainer,
  YAxisProps,
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
  '#722ed1',
  '#13c2c2',
  '#eb2f96',
  '#fa8c16',
];

const getColorIndex = (columns: string[], col: string): number => {
  const index = columns.indexOf(col);
  return index >= 0 ? index : 0;
};

const DualLineChart: React.FC<DualLineChartProps> = ({
  columns,
  rows,
  chart1Columns,
  chart2Columns,
  xAxisRange,
  hiddenLines,
}) => {
  const [minX, maxX] = xAxisRange;
  const displayData = rows.slice(minX, maxX + 1).map((row, index) => {
    const point: Record<string, number | string> = { index: minX + index };
    columns.forEach((col, colIndex) => {
      if (colIndex < row.length) {
        point[col] = row[colIndex];
      }
    });
    return point;
  });

  const visibleChart1Columns = chart1Columns.filter(
    (col) => !hiddenLines.includes(col)
  );
  const visibleChart2Columns = chart2Columns.filter(
    (col) => !hiddenLines.includes(col)
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

  const renderChart = (
    chartColumns: string[],
    showXAxis: boolean,
    chartId: number
  ) => {
    const yAxisConfigs: { col: string; yAxisId: string; orientation: YAxisProps['orientation'] }[] = 
      chartColumns.map((col, index) => ({
        col,
        yAxisId: `yAxis-${chartId}-${index}`,
        orientation: index % 2 === 0 ? 'left' : 'right',
      }));

    return (
      <ResponsiveContainer width="100%" height="100%">
        <LineChart
          data={displayData}
          margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
        >
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border-color)" />
          {showXAxis && (
            <XAxis
              dataKey="index"
              stroke="var(--text-secondary)"
              tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
            />
          )}
          {yAxisConfigs.map(({ yAxisId, orientation }) => (
            <YAxis
              key={yAxisId}
              yAxisId={yAxisId}
              orientation={orientation}
              domain={['auto', 'auto']}
              stroke="var(--text-secondary)"
              tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
            />
          ))}
          <Legend
            onClick={() => {}}
            formatter={(value) => (
              <span style={{ color: 'var(--text-primary)', cursor: 'default' }}>
                {value}
              </span>
            )}
          />
          {chartColumns.map((col) => {
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
                yAxisId={config?.yAxisId || 'left'}
              />
            );
          })}
        </LineChart>
      </ResponsiveContainer>
    );
  };

  return (
    <div style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ width: '100%', height: '50%' }}>
        {renderChart(visibleChart1Columns, false, 1)}
      </div>
      <div style={{ width: '100%', height: '50%' }}>
        {renderChart(visibleChart2Columns, true, 2)}
      </div>
    </div>
  );
};

export default DualLineChart;
