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
import { useCsvChartStore } from '../../stores/csvChartStore';
import { useGh3036Store } from '../../stores/gh3036Store';

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
  sampleRate?: number;
  initialDataZoom?: { start: number; end: number };
  onDataZoomChange?: (state: { start: number; end: number }) => void;
}

const COLORS = [
  '#165DFF',
  '#F53F3F',
  '#00B42A',
  '#FF7D00',
  '#722ED1',
  '#14C9C9',
  '#EB0AA4',
  '#FFC107',
  '#40A9FF'
];

const colorCache = new Map<string, string>();

const getStableColor = (col: string, index: number): string => {
  if (colorCache.has(col)) {
    return colorCache.get(col)!;
  }
  const color = COLORS[index % COLORS.length];
  colorCache.set(col, color);
  return color;
};

const Y_AXIS_WIDTH = 50;
const MAX_LINES_PER_CHART = 4;

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

const formatTime = (seconds: number | undefined): string => {
  if (seconds === undefined || isNaN(seconds)) return '0ms';
  if (seconds < 1) {
    return `${(seconds * 1000).toFixed(0)}ms`;
  }
  if (seconds < 60) {
    return `${seconds.toFixed(2)}s`;
  }
  const minutes = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${minutes}m ${secs.toFixed(1)}s`;
};

interface ContextMenuPosition {
  x: number;
  y: number;
  chartIndex: number;
}

const MultiLineChart: React.FC<MultiLineChartProps> = ({
  columns,
  rows,
  chartGroups,
  sampleRate = 25,
  initialDataZoom,
  onDataZoomChange,
}) => {
  const containerRefs = useRef<(HTMLDivElement | null)[]>([]);
  const chartInstances = useRef<echarts.ECharts[]>([]);
  const [initialized, setInitialized] = useState(false);
  const isZoomingRef = useRef(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuPosition | null>(null);

  const { dataZoomState, setDataZoomState } = useCsvChartStore();
  const { chartLegendSelected, setChartLegendSelected } = useGh3036Store();

  const xAxisData = useMemo(() => {
    const interval = 1 / sampleRate;
    return rows.map((_, index) => index * interval);
  }, [rows, sampleRate]);

  const colorMap = useMemo(() => {
    const map = new Map<string, string>();
    columns.forEach((col, index) => {
      map.set(col, getStableColor(col, index));
    });
    return map;
  }, [columns]);

  const unifiedGridConfig = useMemo(() => {
    const leftWidth = Y_AXIS_WIDTH * 2;
    const rightWidth = Y_AXIS_WIDTH * 2;

    return {
      top: 40,
      left: leftWidth,
      right: rightWidth,
      bottom: 50,
    };
  }, []);

  const getChartOption = useCallback((
    group: ChartGroupConfig,
    groupColorMap: Map<string, string>
  ) => {
    const limitedColumns = group.columns.slice(0, MAX_LINES_PER_CHART);

    const yAxisPositions: Array<{ position: 'left' | 'right'; offset: number }> = [
      { position: 'left', offset: 0 },
      { position: 'right', offset: 0 },
      { position: 'left', offset: Y_AXIS_WIDTH },
      { position: 'right', offset: Y_AXIS_WIDTH },
    ];

    const yAxisOption = yAxisPositions.map((pos, idx) => {
      const col = limitedColumns[idx];
      const color = col ? (groupColorMap.get(col) || COLORS[idx]) : 'transparent';
      const hasData = !!col;

      return {
        name: col || '',
        type: 'value' as const,
        position: pos.position,
        offset: pos.offset,
        scale: true,
        axisLine: {
          show: hasData,
          lineStyle: {
            color: color,
          },
        },
        axisLabel: {
          color: hasData ? color : 'transparent',
          formatter: (value: number) => formatScientific(value),
        },
        nameTextStyle: {
          color: hasData ? color : 'transparent',
        },
        splitLine: {
          show: idx === 0,
        },
      };
    });

    const seriesData = limitedColumns.map((col) => ({
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

    const effectiveDataZoom = initialDataZoom || dataZoomState;

    const dataZoomOption = [
      {
        type: 'slider' as const,
        show: true,
        start: effectiveDataZoom.start,
        end: effectiveDataZoom.end,
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
        labelFormatter: (value: number) => {
          if (rows.length === 0) return '0ms';
          const interval = 1 / sampleRate;
          const totalPoints = rows.length;
          const index = Math.round(value * (totalPoints - 1) / 100);
          const clampedIndex = Math.max(0, Math.min(index, totalPoints - 1));
          const timeValue = clampedIndex * interval;
          return formatTime(timeValue);
        },
      },
    ];

    return {
      animation: false,  // 全局禁用动画
      animationDuration: 0,
      progressive: 500,
      progressiveThreshold: 3000,
      lazyUpdate: true,  // 全局启用懒更新
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
          const items = params as Array<{ seriesName: string; value: number | number[]; color: string; dataIndex?: number }>;
          if (!Array.isArray(items)) return '';
          const dataIndex = items[0]?.dataIndex ?? 0;
          const timeValue = xAxisData[dataIndex];
          let html = `<div style="font-weight: bold; margin-bottom: 4px;">${t('chart.time')}: ${formatTime(timeValue)}</div>`;
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
              return formatTime(num);
            }
            return value;
          },
        },
      },
      yAxis: yAxisOption,
      series: seriesOption,
      dataZoom: dataZoomOption,
    };
  }, [xAxisData, rows, columns, unifiedGridConfig, dataZoomState, initialDataZoom, sampleRate]);

  const handleContextMenu = useCallback((chartIndex: number) => (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, chartIndex });
  }, []);

  const handleSaveAsPNG = useCallback(() => {
    if (!contextMenu) return;
    const chart = chartInstances.current[contextMenu.chartIndex];
    if (!chart) return;

    const url = chart.getDataURL({
      type: 'png',
      pixelRatio: 2,
      backgroundColor: '#fff',
    });

    const link = document.createElement('a');
    link.href = url;
    link.download = `waveform_${chartGroups[contextMenu.chartIndex]?.name || 'chart'}_${Date.now()}.png`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);

    setContextMenu(null);
  }, [contextMenu, chartGroups]);

  const handleSaveAsSVG = useCallback(() => {
    if (!contextMenu) return;
    const chart = chartInstances.current[contextMenu.chartIndex];
    if (!chart) return;

    const url = chart.getDataURL({
      type: 'svg',
      backgroundColor: '#fff',
    });

    const link = document.createElement('a');
    link.href = url;
    link.download = `waveform_${chartGroups[contextMenu.chartIndex]?.name || 'chart'}_${Date.now()}.svg`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);

    setContextMenu(null);
  }, [contextMenu, chartGroups]);

  useEffect(() => {
    const handleClickOutside = () => {
      setContextMenu(null);
    };
    if (contextMenu) {
      document.addEventListener('click', handleClickOutside);
      return () => {
        document.removeEventListener('click', handleClickOutside);
      };
    }
  }, [contextMenu]);

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

    // 使用防抖优化高频更新
    const updateTimer = setTimeout(() => {
      chartInstances.current.forEach((chart, index) => {
        if (!chart) return;

        const group = chartGroups[index];
        if (group) {
          const option = getChartOption(group, colorMap);
          chart.setOption(option, {
            notMerge: false,
            lazyUpdate: true  // 启用懒更新，减少渲染次数
          });
        }
      });
    }, 16);  // 约60fps的更新频率

    return () => clearTimeout(updateTimer);
  }, [rows, columns, initialized, chartGroups, getChartOption, colorMap, sampleRate]);

  useEffect(() => {
    if (!initialized || chartInstances.current.length === 0) return;

    chartInstances.current.forEach((chart, chartIndex) => {
      if (!chart) return;
      const group = chartGroups[chartIndex];
      if (!group) return;

      const limitedColumns = group.columns.slice(0, MAX_LINES_PER_CHART);
      limitedColumns.forEach((col) => {
        const key = `${group.name}_${col}`;
        const selectedState = chartLegendSelected || {};
        const isSelected = selectedState[key];
        
        if (isSelected === false) {
          chart.dispatchAction({
            type: 'legendUnSelect',
            name: col,
          });
        } else if (isSelected === true) {
          chart.dispatchAction({
            type: 'legendSelect',
            name: col,
          });
        }
      });
    });
  }, [initialized, chartGroups, chartLegendSelected]);

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

      setDataZoomState({ start, end });

      if (onDataZoomChange) {
        onDataZoomChange({ start, end });
      }

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
  }, [initialized, setDataZoomState, onDataZoomChange]);

  useEffect(() => {
    if (!initialized || chartInstances.current.length === 0) return;

    const handleLegendSelectChanged = (chartIndex: number) => (params: unknown) => {
      const p = params as { name: string; selected: Record<string, boolean> };
      const group = chartGroups[chartIndex];
      if (!group) return;

      const newSelected = { ...chartLegendSelected };
      Object.entries(p.selected).forEach(([name, selected]) => {
        const key = `${group.name}_${name}`;
        newSelected[key] = selected;
      });
      setChartLegendSelected(newSelected);
    };

    const disposers: Array<() => void> = [];

    chartInstances.current.forEach((chart, index) => {
      if (!chart) return;
      const handler = handleLegendSelectChanged(index);
      chart.on('legendselectchanged', handler);
      disposers.push(() => chart.off('legendselectchanged', handler));
    });

    return () => {
      disposers.forEach((dispose) => dispose());
    };
  }, [initialized, chartGroups, chartLegendSelected, setChartLegendSelected]);

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
          className="chart-container"
          style={{
            width: '100%',
            height: group.height || 300,
            minHeight: 150,
            flexShrink: 0,
            borderBottom: index < chartGroups.length - 1 ? '1px solid var(--border-color)' : 'none',
          }}
          onContextMenu={handleContextMenu(index)}
        />
      ))}

      {contextMenu && (
        <div
          className="chart-context-menu"
          style={{
            left: contextMenu.x,
            top: contextMenu.y,
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <div
            className="chart-context-menu-item"
            onClick={(e) => {
              e.stopPropagation();
              handleSaveAsPNG();
            }}
          >
            保存为 PNG
          </div>
          <div
            className="chart-context-menu-item"
            onClick={(e) => {
              e.stopPropagation();
              handleSaveAsSVG();
            }}
          >
            保存为 SVG
          </div>
        </div>
      )}
    </div>
  );
};

function t(key: string): string {
  const translations: Record<string, string> = {
    'chart.time': '时间',
  };
  return translations[key] || key;
}

export default React.memo(MultiLineChart);
