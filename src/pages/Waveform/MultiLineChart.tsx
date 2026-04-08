import React, { useMemo, useRef, useEffect, useCallback } from 'react';
import * as echarts from 'echarts/core';
import { LineChart } from 'echarts/charts';
import {
  LegendComponent,
  GridComponent,
  DataZoomComponent,
  TooltipComponent,
  ToolboxComponent,
} from 'echarts/components';
import { UniversalTransition } from 'echarts/features';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([
  LineChart,
  LegendComponent,
  GridComponent,
  DataZoomComponent,
  TooltipComponent,
  ToolboxComponent,
  CanvasRenderer,
  UniversalTransition,
]);

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

interface SingleChartProps {
  xAxisData: (string | number)[];
  series: { name: string; data: number[]; color: string }[];
  yAxisConfigs: YAxisConfig[];
  height: number;
  showDataZoom: boolean;
  visiblePoints: number;
  chartRefs: React.MutableRefObject<(echarts.ECharts | null)[]>;
  index: number;
  onDataZoomChange?: (start: number, end: number) => void;
}

const SingleChart: React.FC<SingleChartProps> = ({
  xAxisData,
  series,
  yAxisConfigs,
  height,
  showDataZoom,
  visiblePoints,
  chartRefs,
  index,
  onDataZoomChange,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);

  const option = useMemo(() => {
    const totalPoints = xAxisData.length;
    const start = 0;
    const end = totalPoints > 0 ? Math.min((visiblePoints / totalPoints) * 100, 100) : 100;

    const yAxisOption = yAxisConfigs.map((config, idx) => ({
      name: config.column,
      type: 'value',
      position: config.position,
      offset: config.offset,
      scale: true,
      axisLine: {
        show: true,
        lineStyle: {
          color: config.color,
        },
      },
      axisLabel: {
        color: config.color,
      },
      nameTextStyle: {
        color: config.color,
      },
      splitLine: {
        show: idx === 0,
      },
    }));

    const seriesOption = series.map((s, idx) => ({
      name: s.name,
      type: 'line',
      data: s.data,
      smooth: false,
      yAxisIndex: idx,
      lineStyle: {
        color: s.color,
        width: 1.5,
      },
      itemStyle: {
        color: s.color,
      },
      symbol: 'none',
      animation: false,
    }));

    const dataZoomOption = showDataZoom
      ? [
          {
            type: 'slider' as const,
            show: true,
            start,
            end,
            zoomLock: false,
            xAxisIndex: [0],
            height: 24,
            bottom: 8,
            handleStyle: {
              color: '#1890ff',
              borderColor: '#1890ff',
            },
            trackStyle: {
              backgroundColor: 'var(--bg-secondary)',
            },
            selectedDataBackground: {
              lineStyle: {
                color: '#1890ff',
              },
              areaStyle: {
                color: 'rgba(24, 144, 255, 0.2)',
              },
            },
            fillerColor: 'rgba(24, 144, 255, 0.15)',
            borderColor: 'var(--border-color)',
            textStyle: {
              color: 'var(--text-secondary)',
            },
            labelPrecision: 0,
          },
        ]
      : [];

    return {
      animationDuration: 0,
      progressive: 500,
      progressiveThreshold: 3000,
      tooltip: {
        trigger: 'axis',
        axisPointer: {
          type: 'cross',
          lineStyle: {
            color: 'var(--border-color)',
          },
          crossStyle: {
            color: 'var(--border-color)',
          },
        },
        backgroundColor: 'var(--bg-secondary)',
        borderColor: 'var(--border-color)',
        textStyle: {
          color: 'var(--text-primary)',
        },
      },
      legend: {
        top: 4,
        left: 'center',
        orient: 'horizontal',
        data: series.map((s) => s.name),
        textStyle: {
          color: 'var(--text-primary)',
        },
      },
      grid: {
        top: 40,
        left: yAxisConfigs.filter((c) => c.position === 'left').length > 0 ? 50 + (yAxisConfigs.filter((c) => c.position === 'left').length - 1) * 40 : 40,
        right: yAxisConfigs.filter((c) => c.position === 'right').length > 0 ? 50 + (yAxisConfigs.filter((c) => c.position === 'right').length - 1) * 40 : 20,
        bottom: showDataZoom ? 40 : 20,
      },
      xAxis: {
        type: 'category',
        data: xAxisData,
        boundaryGap: false,
        splitNumber: 10,
        axisLine: {
          lineStyle: {
            color: 'var(--border-color)',
          },
        },
        axisLabel: {
          color: 'var(--text-secondary)',
          formatter: (value: string | number) => {
            const num = typeof value === 'number' ? value : parseFloat(value);
            if (!isNaN(num)) {
              return num.toFixed(0);
            }
            return value;
          },
        },
      },
      yAxis: yAxisOption.length > 0 ? yAxisOption : [{ type: 'value', scale: true }],
      series: seriesOption,
      dataZoom: dataZoomOption,
    };
  }, [xAxisData, series, yAxisConfigs, showDataZoom, visiblePoints]);

  useEffect(() => {
    if (!containerRef.current) return;

    if (chartRefs.current[index]) {
      chartRefs.current[index].dispose();
    }

    const chart = echarts.init(containerRef.current);
    chartRefs.current[index] = chart;
    chart.setOption(option);

    if (showDataZoom && onDataZoomChange) {
      chart.on('datazoom', (params: unknown) => {
        const p = params as { start?: number; end?: number; batch?: Array<{ start: number; end: number }> };
        if (p.batch) {
          onDataZoomChange(p.batch[0].start, p.batch[0].end);
        } else if (p.start !== undefined && p.end !== undefined) {
          onDataZoomChange(p.start, p.end);
        }
      });
    }

    const handleResize = () => {
      chart.resize();
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.dispose();
      chartRefs.current[index] = null;
    };
  }, [option, chartRefs, index, showDataZoom, onDataZoomChange]);

  useEffect(() => {
    if (chartRefs.current[index]) {
      chartRefs.current[index].setOption(option, { notMerge: true });
    }
  }, [option, chartRefs, index]);

  return (
    <div
      ref={containerRef}
      style={{
        width: '100%',
        height,
        minHeight: 150,
      }}
    />
  );
};

const MultiLineChart: React.FC<MultiLineChartProps> = ({
  columns,
  rows,
  chartGroups,
  yAxisConfigs = {},
  visiblePoints = 1000,
}) => {
  const chartRefs = useRef<(echarts.ECharts | null)[]>([]);
  const zoomStateRef = useRef<{ start: number; end: number }>({ start: 0, end: 100 });

  const xAxisData = useMemo(() => {
    return rows.map((_, index) => index);
  }, [rows]);

  const colorMap = useMemo(() => {
    const map = new Map<string, string>();
    columns.forEach((col, index) => {
      map.set(col, COLORS[index % COLORS.length]);
    });
    return map;
  }, [columns]);

  const handleDataZoomChange = useCallback((start: number, end: number) => {
    zoomStateRef.current = { start, end };
    chartRefs.current.forEach((chart, idx) => {
      if (chart && idx !== chartGroups.length - 1) {
        chart.dispatchAction({
          type: 'dataZoom',
          start,
          end,
        });
      }
    });
  }, [chartGroups.length]);

  useEffect(() => {
    if (chartRefs.current.filter(Boolean).length > 1) {
      echarts.connect(chartRefs.current.filter(Boolean) as echarts.ECharts[]);
    }
  }, [chartGroups.length]);

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
          offset: Math.floor(colIndex / 2) * 50,
          color: colorMap.get(col) || COLORS[colIndex % COLORS.length],
        }));

        const series = group.columns.map((col) => ({
          name: col,
          data: rows.map((row) => {
            const colIndex = columns.indexOf(col);
            return colIndex >= 0 && colIndex < row.length ? row[colIndex] : 0;
          }),
          color: colorMap.get(col) || COLORS[columns.indexOf(col) % COLORS.length],
        }));

        return (
          <div
            key={group.name}
            style={{
              width: '100%',
              height: group.height || 300,
              minHeight: 150,
              flexShrink: 0,
              borderBottom: index < chartGroups.length - 1 ? '1px solid var(--border-color)' : 'none',
            }}
          >
            <SingleChart
              xAxisData={xAxisData}
              series={series}
              yAxisConfigs={groupYAxisConfigs}
              height={group.height || 300}
              showDataZoom={index === chartGroups.length - 1}
              visiblePoints={visiblePoints}
              chartRefs={chartRefs}
              index={index}
              onDataZoomChange={index === chartGroups.length - 1 ? handleDataZoomChange : undefined}
            />
          </div>
        );
      })}
    </div>
  );
};

export default React.memo(MultiLineChart);
