import React, { useMemo, useCallback, memo, useRef, useEffect, useState } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Legend,
  ResponsiveContainer,
  Brush,
} from 'recharts';

export interface ChartGroupConfig {
  name: string;
  columns: string[];
  height?: number;
}

export interface YAxisConfig {
  column: string;
  position: 'left' | 'right';
  offset: number;
  color: string;
}

interface MultiLineChartProps {
  columns: string[];
  rows: number[][];
  chartGroups: ChartGroupConfig[];
  yAxisConfigs?: Record<string, YAxisConfig[]>;
  visiblePoints?: number;
}

const COLORS = [
  '#165DFF',
  '#F53F3F',
  '#00B42A',
  '#FF7D00',
  '#722ed1',
  '#13c2c2',
  '#eb2f96',
  '#fa8c16',
];

const SAMPLING_THRESHOLD = 2000;

const sampleData = <T,>(data: T[], maxPoints: number): T[] => {
  if (data.length <= maxPoints) return data;
  const step = Math.ceil(data.length / maxPoints);
  const sampled: T[] = [];
  for (let i = 0; i < data.length; i += step) {
    sampled.push(data[i]);
  }
  return sampled;
};

interface SingleChartProps {
  displayData: Record<string, number | string>[];
  chartConfig: ChartGroupConfig;
  columns: string[];
  yAxisConfigs: YAxisConfig[];
  showXAxis: boolean;
  showBrush: boolean;
  brushRange: [number, number];
  onBrushChange: (range: [number, number]) => void;
  colorMap: Map<string, string>;
}

const SingleChart: React.FC<SingleChartProps> = memo(({
  displayData,
  chartConfig,
  columns,
  yAxisConfigs,
  showXAxis,
  showBrush,
  brushRange,
  onBrushChange,
  colorMap,
}) => {
  const visibleColumns = chartConfig.columns.filter(col => columns.includes(col));
  
  const yAxisConfigMap = useMemo(() => {
    const map = new Map<string, YAxisConfig>();
    yAxisConfigs.forEach(config => {
      map.set(config.column, config);
    });
    return map;
  }, [yAxisConfigs]);

  const leftYAxes = useMemo(() => {
    return yAxisConfigs
      .filter(c => c.position === 'left' && visibleColumns.includes(c.column))
      .sort((a, b) => a.offset - b.offset);
  }, [yAxisConfigs, visibleColumns]);

  const rightYAxes = useMemo(() => {
    return yAxisConfigs
      .filter(c => c.position === 'right' && visibleColumns.includes(c.column))
      .sort((a, b) => a.offset - b.offset);
  }, [yAxisConfigs, visibleColumns]);

  const sampledData = useMemo(() => 
    sampleData(displayData, SAMPLING_THRESHOLD),
    [displayData]
  );

  const renderLegend = useCallback((value: string) => (
    <span style={{ color: 'var(--text-primary)', cursor: 'default' }}>
      {value}
    </span>
  ), []);

  if (visibleColumns.length === 0) {
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

  const leftOffset = 20 + leftYAxes.length * 50;
  const rightOffset = 30 + rightYAxes.length * 50;

  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart
        data={sampledData}
        margin={{ top: 5, right: rightOffset, left: leftOffset, bottom: 5 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="var(--border-color)" />
        {showXAxis && (
          <XAxis
            dataKey="index"
            stroke="var(--text-secondary)"
            tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
          />
        )}
        {leftYAxes.map((config, index) => (
          <YAxis
            key={`left-${config.column}`}
            yAxisId={`y-${config.column}`}
            orientation="left"
            domain={['auto', 'auto']}
            stroke={config.color}
            tick={{ fill: config.color, fontSize: 11 }}
            width={50}
            label={{
              value: config.column,
              angle: -90,
              position: 'insideLeft',
              offset: 10 + index * 50,
              fill: config.color,
              fontSize: 11,
            }}
          />
        ))}
        {rightYAxes.map((config, index) => (
          <YAxis
            key={`right-${config.column}`}
            yAxisId={`y-${config.column}`}
            orientation="right"
            domain={['auto', 'auto']}
            stroke={config.color}
            tick={{ fill: config.color, fontSize: 11 }}
            width={50}
            label={{
              value: config.column,
              angle: 90,
              position: 'insideRight',
              offset: 10 + index * 50,
              fill: config.color,
              fontSize: 11,
            }}
          />
        ))}
        <Legend
          onClick={() => {}}
          formatter={renderLegend}
        />
        {showBrush && (
          <Brush
            dataKey="index"
            height={30}
            stroke="var(--primary-color)"
            fill="var(--bg-secondary)"
            startIndex={brushRange[0]}
            endIndex={brushRange[1]}
            onChange={(e) => {
              if (e && typeof e.startIndex === 'number' && typeof e.endIndex === 'number') {
                onBrushChange([e.startIndex, e.endIndex]);
              }
            }}
          />
        )}
        {visibleColumns.map((col) => {
          const config = yAxisConfigMap.get(col);
          const color = colorMap.get(col) || COLORS[columns.indexOf(col) % COLORS.length];
          return (
            <Line
              key={col}
              type="monotone"
              dataKey={col}
              stroke={color}
              strokeWidth={1.5}
              dot={false}
              isAnimationActive={false}
              yAxisId={config ? `y-${config.column}` : 'y-default'}
            />
          );
        })}
      </LineChart>
    </ResponsiveContainer>
  );
});

SingleChart.displayName = 'SingleChart';

const MultiLineChart: React.FC<MultiLineChartProps> = ({
  columns,
  rows,
  chartGroups,
  yAxisConfigs = {},
  visiblePoints = 1000,
}) => {
  const [brushRange, setBrushRange] = useState<[number, number]>([0, Math.min(visiblePoints - 1, rows.length - 1)]);
  const prevRowsLengthRef = useRef(rows.length);

  useEffect(() => {
    if (rows.length !== prevRowsLengthRef.current) {
      prevRowsLengthRef.current = rows.length;
      setBrushRange([0, Math.min(visiblePoints - 1, rows.length - 1)]);
    }
  }, [rows.length, visiblePoints]);

  const displayData = useMemo(() => {
    const [start, end] = brushRange;
    const slicedRows = rows.slice(start, end + 1);
    return slicedRows.map((row, index) => {
      const point: Record<string, number | string> = { index: start + index };
      columns.forEach((col, colIndex) => {
        if (colIndex < row.length) {
          point[col] = row[colIndex];
        }
      });
      return point;
    });
  }, [rows, brushRange, columns]);

  const colorMap = useMemo(() => {
    const map = new Map<string, string>();
    columns.forEach((col, index) => {
      map.set(col, COLORS[index % COLORS.length]);
    });
    return map;
  }, [columns]);

  const handleBrushChange = useCallback((range: [number, number]) => {
    setBrushRange(range);
  }, []);

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

  if (chartGroups.length === 0) {
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
        请配置图表分组
      </div>
    );
  }

  return (
    <div style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column', overflow: 'auto' }}>
      {chartGroups.map((group, index) => {
        const groupYAxisConfigs = yAxisConfigs[group.name] || group.columns.map((col, colIndex) => ({
          column: col,
          position: (colIndex % 2 === 0 ? 'left' : 'right') as 'left' | 'right',
          offset: Math.floor(colIndex / 2) * 60,
          color: colorMap.get(col) || COLORS[colIndex % COLORS.length],
        }));

        return (
          <div
            key={group.name}
            style={{
              width: '100%',
              height: group.height || 300,
              minHeight: 200,
              flexShrink: 0,
              borderBottom: index < chartGroups.length - 1 ? '1px solid var(--border-color)' : 'none',
            }}
          >
            <SingleChart
              displayData={displayData}
              chartConfig={group}
              columns={columns}
              yAxisConfigs={groupYAxisConfigs}
              showXAxis={index === chartGroups.length - 1}
              showBrush={index === chartGroups.length - 1}
              brushRange={brushRange}
              onBrushChange={handleBrushChange}
              colorMap={colorMap}
            />
          </div>
        );
      })}
    </div>
  );
};

export default memo(MultiLineChart);
