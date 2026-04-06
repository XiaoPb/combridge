import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';

interface WaveformChartProps {
  columns: string[];
  rows: number[][];
  displayRows?: number;
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

const WaveformChart: React.FC<WaveformChartProps> = ({
  columns,
  rows,
  displayRows = 500,
}) => {
  const displayData = rows.slice(-displayRows).map((row, index) => {
    const point: Record<string, number | string> = { index };
    columns.forEach((col, colIndex) => {
      if (colIndex < row.length) {
        point[col] = row[colIndex];
      }
    });
    return point;
  });

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
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={displayData}
        margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="var(--border-color)" />
        <XAxis
          dataKey="index"
          stroke="var(--text-secondary)"
          tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
        />
        <YAxis
          stroke="var(--text-secondary)"
          tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
        />
        <Tooltip
          contentStyle={{
            backgroundColor: 'var(--bg-secondary)',
            border: '1px solid var(--border-color)',
            borderRadius: 4,
          }}
          labelStyle={{ color: 'var(--text-primary)' }}
        />
        <Legend />
        {columns.map((col, index) => (
          <Line
            key={col}
            type="monotone"
            dataKey={col}
            stroke={COLORS[index % COLORS.length]}
            dot={false}
            strokeWidth={1.5}
            isAnimationActive={false}
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  );
};

export default WaveformChart;
