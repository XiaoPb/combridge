import React, { useMemo, useRef, useEffect, useCallback, useState } from 'react';
import * as echarts from 'echarts/core';
import { LineChart } from 'echarts/charts';
import {
  LegendComponent,
  GridComponent,
  DataZoomComponent,
  TooltipComponent,
} from 'echarts/components';
import { UniversalTransition } from 'echarts/features';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([
  LineChart,
  LegendComponent,
  GridComponent,
  DataZoomComponent,
  TooltipComponent,
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

const formatScientific = (value: number): string => {
  if (value === 0) return '0';
  const absValue = Math.abs(value);
  if (absValue >= 10000 || absValue < 0.001) {
    return value.toExponential(1);
  }
  if (absValue >= 100) {
    return value.toFixed(0);
  }
  if (absValue >= 1) {
    return value.toFixed(2);
  }
  return value.toFixed(4);
};

const MultiLineChart: React.FC<MultiLineChartProps> = ({
  columns,
  rows,
  chartGroups,
  yAxisConfigs = {},
}) => {
  const containerRefs = useRef<(HTMLDivElement | null)[]>([]);
  const chartInstances = useRef<echarts.ECharts[]>([]);
  const [initialized, setInitialized] = useState(false);
  const zoomStateRef = useRef<{ start: number; end: number }>({ start: 0, end: 100 });
  const isZoomingRef = useRef(false);

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

  const unifiedGridConfig = useMemo(() => {
    let maxLeftAxisCount = 1;
    let maxRightAxisCount = 0;

    chartGroups.forEach((group) => {
      const groupYAxisConfigs = yAxisConfigs[group.name] || group.columns.map((col, colIndex) => ({
        column: col,
        position: (colIndex % 2 === 0 ? 'left' : 'right') as 'left' | 'right',
        offset: Math.floor(colIndex / 2) * 50,
        color: colorMap.get(col) || COLORS[colIndex % COLORS.length],
      }));

      const leftCount = groupYAxisConfigs.filter((c) => c.position === 'left').length;
      const rightCount = groupYAxisConfigs.filter((c) => c.position === 'right').length;

      maxLeftAxisCount = Math.max(maxLeftAxisCount, leftCount);
      maxRightAxisCount = Math.max(maxRightAxisCount, rightCount);
    });

    const leftWidth = 50 + (maxLeftAxisCount - 1) * 40;
    const rightWidth = maxRightAxisCount > 0 ? 50 + (maxRightAxisCount - 1) * 40 : 20;

    return {
      top: 40,
      left: leftWidth,
      right: rightWidth,
      bottom: 50,
    };
  }, [chartGroups, yAxisConfigs, colorMap]);

  const getChartOption = useCallback((
    group: ChartGroupConfig,
    groupColorMap: Map<string, string>
  ) => {
    const groupYAxisConfigs = yAxisConfigs[group.name] || group.columns.map((col, colIndex) => ({
      column: col,
      position: (colIndex % 2 === 0 ? 'left' : 'right') as 'left' | 'right',
      offset: Math.floor(colIndex / 2) * 50,
      color: groupColorMap.get(col) || COLORS[colIndex % COLORS.length],
    }));

    const yAxisOption = groupYAxisConfigs.map((config, idx) => ({
      name: config.column,
      type: 'value' as const,
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
        formatter: (value: number) => formatScientific(value),
      },
      nameTextStyle: {
        color: config.color,
      },
      splitLine: {
        show: idx === 0,
      },
    }));

    const seriesData = group.columns.map((col) => ({
      name: col,
      data: rows.map((row) => {
        const colIndex = columns.indexOf(col);
        return colIndex >= 0 && colIndex < row.length ? row[colIndex] : 0;
      }),
      color: groupColorMap.get(col) || COLORS[columns.indexOf(col) % COLORS.length],
    }));

    const seriesOption = seriesData.map((s, idx) => ({
      name: s.name,
      type: 'line' as const,
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

    const dataZoomOption = [
      {
        type: 'slider' as const,
        show: true,
        start: zoomStateRef.current.start,
        end: zoomStateRef.current.end,
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
    ];

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
        formatter: (params: unknown) => {
          const items = params as Array<{ seriesName: string; value: number | number[]; color: string }>;
          if (!Array.isArray(items)) return '';
          const firstValue = items[0]?.value;
          const index = Array.isArray(firstValue) ? firstValue[0] : firstValue;
          let html = `<div style="font-weight: bold; margin-bottom: 4px;">Index: ${index}</div>`;
          items.forEach((item) => {
            const val = Array.isArray(item.value) ? item.value[1] : item.value;
            html += `<div style="display: flex; align-items: center; gap: 8px;">
              <span style="display: inline-block; width: 10px; height: 10px; background: ${item.color}; border-radius: 50%;"></span>
              <span>${item.seriesName}: ${formatScientific(val)}</span>
            </div>`;
          });
          return html;
        },
      },
      legend: {
        top: 4,
        left: 'center',
        orient: 'horizontal',
        data: seriesData.map((s) => s.name),
        textStyle: {
          color: 'var(--text-primary)',
        },
      },
      grid: unifiedGridConfig,
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
      yAxis: yAxisOption.length > 0 ? yAxisOption : [{ type: 'value' as const, scale: true, axisLabel: { formatter: (value: number) => formatScientific(value) } }],
      series: seriesOption,
      dataZoom: dataZoomOption,
    };
  }, [xAxisData, rows, columns, yAxisConfigs, unifiedGridConfig]);

  useEffect(() => {
    chartInstances.current.forEach((chart) => {
      chart?.dispose();
    });
    chartInstances.current = [];

    containerRefs.current.forEach((container, index) => {
      if (!container) return;

      const chart = echarts.init(container);
      chartInstances.current[index] = chart;

      const group = chartGroups[index];
      if (group) {
        const option = getChartOption(group, colorMap);
        chart.setOption(option);
      }
    });

    if (chartInstances.current.filter(Boolean).length > 1) {
      echarts.connect(chartInstances.current.filter(Boolean));
    }

    setInitialized(true);

    return () => {
      chartInstances.current.forEach((chart) => {
        chart?.dispose();
      });
      chartInstances.current = [];
      setInitialized(false);
    };
  }, [chartGroups, getChartOption, colorMap]);

  useEffect(() => {
    if (!initialized) return;

    chartInstances.current.forEach((chart, index) => {
      if (!chart) return;

      const group = chartGroups[index];
      if (group) {
        const option = getChartOption(group, colorMap);
        chart.setOption(option, { notMerge: false });
      }
    });
  }, [rows, columns, initialized, chartGroups, getChartOption, colorMap]);

  useEffect(() => {
    if (!initialized || chartInstances.current.length === 0) return;

    const handleDataZoom = (chartIndex: number) => (params: unknown) => {
      if (isZoomingRef.current) return;
      isZoomingRef.current = true;

      const p = params as { start?: number; end?: number; batch?: Array<{ start: number; end: number }> };
      let start: number, end: number;

      if (p.batch) {
        start = p.batch[0].start;
        end = p.batch[0].end;
      } else if (p.start !== undefined && p.end !== undefined) {
        start = p.start;
        end = p.end;
      } else {
        isZoomingRef.current = false;
        return;
      }

      zoomStateRef.current = { start, end };

      chartInstances.current.forEach((chart, idx) => {
        if (chart && idx !== chartIndex) {
          chart.dispatchAction({
            type: 'dataZoom',
            start,
            end,
          });
        }
      });

      setTimeout(() => {
        isZoomingRef.current = false;
      }, 50);
    };

    const disposers: Array<() => void> = [];

    chartInstances.current.forEach((chart, index) => {
      if (!chart) return;
      const handler = handleDataZoom(index);
      chart.on('datazoom', handler);
      disposers.push(() => chart.off('datazoom', handler));
    });

    return () => {
      disposers.forEach((dispose) => dispose());
    };
  }, [initialized]);

  useEffect(() => {
    const handleResize = () => {
      chartInstances.current.forEach((chart) => {
        chart?.resize();
      });
    };

    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('resize', handleResize);
    };
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
      {chartGroups.map((group, index) => (
        <div
          key={group.name}
          ref={(el) => { containerRefs.current[index] = el; }}
          style={{
            width: '100%',
            height: group.height || 300,
            minHeight: 150,
            flexShrink: 0,
            borderBottom: index < chartGroups.length - 1 ? '1px solid var(--border-color)' : 'none',
          }}
        />
      ))}
    </div>
  );
};

export default React.memo(MultiLineChart);
