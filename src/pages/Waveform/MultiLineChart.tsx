import React, {
  forwardRef,
  useMemo,
  useRef,
  useEffect,
  useCallback,
  useState,
  useImperativeHandle,
} from 'react';
import { useTranslation } from 'react-i18next';
import * as echarts from 'echarts/core';
import { LineChart } from 'echarts/charts';
import {
  LegendComponent,
  GridComponent,
  DataZoomComponent,
  TooltipComponent,
} from 'echarts/components';
import { UniversalTransition } from 'echarts/features';
import { CanvasRenderer, SVGRenderer } from 'echarts/renderers';
import type { ChartGroupConfig } from './chartGroup';
import {
  getChartGroupKey,
  getChartLegendKey,
  getLegendAction,
  migrateLegacyChartLegendSelections,
  resolveChartLegendSelection,
} from './chartGroup';
import {
  buildChartSeries,
  MAX_LINES_PER_CHART,
} from './multiLineChartModel';
import {
  composeChartPng,
  dataUrlToBlob,
  downloadBlob,
  ensureWhiteSvgBackground,
  resolveCssVariablesInValue,
} from './multiLineChartExport';
export type { ChartGroupConfig } from './chartGroup';

echarts.use([
  LineChart,
  LegendComponent,
  GridComponent,
  DataZoomComponent,
  TooltipComponent,
  CanvasRenderer,
  SVGRenderer,
  UniversalTransition,
]);

export interface YAxisConfig {
  column: string;
  position: 'left' | 'right';
  offset: number;
  color: string;
}

export interface MultiLineChartProps {
  columns: string[];
  rows: number[][];
  chartGroups: ChartGroupConfig[];
  sampleRate?: number;
  initialDataZoom?: { start: number; end: number };
  onDataZoomChange?: (state: { start: number; end: number }) => void;
  legendScope?: string;
  legendSelected?: Record<string, boolean>;
  onLegendSelectedChange?: (selected: Record<string, boolean>) => void;
  onExportError?: (error: Error) => void;
}

export interface MultiLineChartHandle {
  exportAllPng: () => Promise<void>;
}

interface ChartExportInstance {
  resize: () => void;
  getWidth: () => number;
  getHeight: () => number;
  getDataURL: (options: {
    type: 'png';
    pixelRatio: number;
    backgroundColor: string;
  }) => string;
}

interface ChartExportDependencies {
  composeChartPng: (
    dataUrls: readonly string[],
    options: { gap: number },
  ) => Promise<{ blob: Blob }>;
  downloadBlob: (blob: Blob, filename: string) => void;
  waitForRender: () => Promise<void>;
  now: () => number;
  onExportError?: (error: Error) => void;
}

const Y_AXIS_WIDTH = 50;
type DataZoomState = { start: number; end: number };

export function dispatchDataZoomSilently(
  chart: Pick<echarts.ECharts, 'dispatchAction'>,
  state: DataZoomState,
): void {
  chart.dispatchAction(
    {
      type: 'dataZoom',
      start: state.start,
      end: state.end,
    },
    { silent: true },
  );
}

export function handleDataZoomEvent(
  chartKey: string,
  params: unknown,
  chartInstances: ReadonlyMap<
    string,
    Pick<echarts.ECharts, 'dispatchAction'>
  >,
  setLocalDataZoom: (state: DataZoomState) => void,
  onDataZoomChange?: (state: DataZoomState) => void,
): void {
  const p = params as {
    start?: number;
    end?: number;
    batch?: Array<{ start: number; end: number }>;
  };
  const zoom = p.batch?.[0] ?? p;
  if (zoom.start === undefined || zoom.end === undefined) return;

  const state = { start: zoom.start, end: zoom.end };
  setLocalDataZoom(state);
  onDataZoomChange?.(state);
  chartInstances.forEach((sibling, siblingKey) => {
    if (siblingKey !== chartKey) dispatchDataZoomSilently(sibling, state);
  });
}

const formatScientific = (value: number): string => {
  if (value === 0) return '0';
  const absValue = Math.abs(value);
  if (absValue >= 10000 || absValue < 0.001) return value.toExponential(1);
  if (absValue >= 100) return value.toFixed(0);
  if (absValue >= 1) return value.toFixed(2);
  return value.toFixed(4);
};

export function getYAxisSplitNumber(height: number): number {
  if (!Number.isFinite(height) || height < 220) return 3;
  if (height < 300) return 4;
  return 5;
}

export function getChartYAxisSplitNumber(height?: number): number {
  return getYAxisSplitNumber(height ?? 300);
}

export function normalizeChartHeight(height?: number): number {
  return height !== undefined && Number.isFinite(height) ? height : 300;
}

export function formatActualValue(value: number): string {
  if (!Number.isFinite(value)) return String(value);

  const valueString = value.toString();
  const exponentIndex = valueString.search(/[eE]/);
  if (exponentIndex === -1) return valueString;

  const [coefficient, exponentString] = valueString.split(/[eE]/);
  const exponent = Number(exponentString);
  const sign = coefficient.startsWith('-') ? '-' : '';
  const unsignedCoefficient = coefficient.replace(/^[+-]/, '');
  const digits = unsignedCoefficient.replace('.', '');
  const coefficientDecimalPosition = unsignedCoefficient.indexOf('.');
  const decimalPosition =
    (coefficientDecimalPosition === -1
      ? unsignedCoefficient.length
      : coefficientDecimalPosition) + exponent;

  if (decimalPosition <= 0) {
    return `${sign}0.${'0'.repeat(-decimalPosition)}${digits}`;
  }
  if (decimalPosition >= digits.length) {
    return `${sign}${digits}${'0'.repeat(decimalPosition - digits.length)}`;
  }
  return `${sign}${digits.slice(0, decimalPosition)}.${digits.slice(decimalPosition)}`;
}

const formatTime = (seconds: number | undefined): string => {
  if (seconds === undefined || isNaN(seconds)) return '0ms';
  if (seconds < 1) return `${(seconds * 1000).toFixed(0)}ms`;
  if (seconds < 60) return `${seconds.toFixed(2)}s`;
  return `${Math.floor(seconds / 60)}m ${(seconds % 60).toFixed(1)}s`;
};

interface ContextMenuPosition {
  x: number;
  y: number;
  groupKey: string;
}

const MultiLineChart = forwardRef<MultiLineChartHandle, MultiLineChartProps>(({
  columns,
  rows,
  chartGroups,
  sampleRate = 25,
  initialDataZoom,
  onDataZoomChange,
  legendScope = 'chart',
  legendSelected,
  onLegendSelectedChange,
  onExportError,
}, ref) => {
  const { t: translate } = useTranslation('waveform');
  const containerRefs = useRef(new Map<string, HTMLDivElement>());
  const chartInstances = useRef(new Map<string, echarts.ECharts>());
  const structureSignatures = useRef(new Map<string, string>());
  const [initialized, setInitialized] = useState(false);
  const [instanceRevision, setInstanceRevision] = useState(0);
  const [localLegendSelected, setLocalLegendSelected] = useState<
    Record<string, boolean>
  >({});
  const [localDataZoom, setLocalDataZoom] = useState<DataZoomState>(
    initialDataZoom ?? { start: 0, end: 100 },
  );
  const updateTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const legendSelectedRef = useRef(legendSelected);
  const localLegendSelectedRef = useRef(localLegendSelected);
  const onLegendSelectedChangeRef = useRef(onLegendSelectedChange);
  const onDataZoomChangeRef = useRef(onDataZoomChange);
  const onExportErrorRef = useRef(onExportError);
  const [contextMenu, setContextMenu] = useState<ContextMenuPosition | null>(
    null,
  );

  legendSelectedRef.current = legendSelected;
  localLegendSelectedRef.current = localLegendSelected;
  onLegendSelectedChangeRef.current = onLegendSelectedChange;
  onDataZoomChangeRef.current = onDataZoomChange;
  onExportErrorRef.current = onExportError;
  const effectiveLegendSelected = legendSelected ?? localLegendSelected;

  useEffect(() => {
    setLocalDataZoom(initialDataZoom ?? { start: 0, end: 100 });
  }, [initialDataZoom?.start, initialDataZoom?.end]);

  useEffect(() => {
    if (legendSelected === undefined) return;
    const migrated = migrateLegacyChartLegendSelections(
      legendScope,
      legendSelected,
      chartGroups,
    );
    if (migrated !== legendSelected)
      onLegendSelectedChangeRef.current?.(migrated);
  }, [legendSelected, chartGroups, legendScope]);

  const xAxisData = useMemo(
    () => rows.map((_, index) => index / sampleRate),
    [rows, sampleRate],
  );
  const unifiedGridConfig = useMemo(
    () => ({
      top: 40,
      left: Y_AXIS_WIDTH * 2,
      right: Y_AXIS_WIDTH * 2,
      bottom: 50,
    }),
    [],
  );
  const chartGeometrySignature = useMemo(
    () =>
      chartGroups
        .map(
          (group, index) =>
            `${getChartGroupKey(group, index)}:${normalizeChartHeight(group.height)}`,
        )
        .join('|'),
    [chartGroups],
  );
  const chartGroupKeySignature = useMemo(
    () =>
      chartGroups
        .map((group, index) => getChartGroupKey(group, index))
        .join('\u0001'),
    [chartGroups],
  );

  const getChartOption = useCallback(
    (group: ChartGroupConfig) => {
      const seriesData = buildChartSeries(
        columns,
        rows,
        group.columns,
        MAX_LINES_PER_CHART,
      );
      const yAxisPositions: Array<{
        position: 'left' | 'right';
        offset: number;
      }> = [
        { position: 'left', offset: 0 },
        { position: 'right', offset: 0 },
        { position: 'left', offset: Y_AXIS_WIDTH },
        { position: 'right', offset: Y_AXIS_WIDTH },
      ];
      const yAxis = yAxisPositions.map((pos, idx) => {
        const series = seriesData[idx];
        const col = series?.name;
        const color = series?.color || 'transparent';
        const hasData = !!col;
        return {
          name: col || '',
          type: 'value' as const,
          position: pos.position,
          offset: pos.offset,
          splitNumber: getChartYAxisSplitNumber(group.height),
          scale: true,
          axisLine: { show: hasData, lineStyle: { color } },
          axisLabel: {
            color: hasData ? color : 'transparent',
            formatter: (value: number) => formatScientific(value),
          },
          nameTextStyle: { color: hasData ? color : 'transparent' },
          splitLine: { show: idx === 0 },
        };
      });
      const series = seriesData.map((s, idx) => ({
        name: s.name,
        type: 'line' as const,
        data: s.data,
        smooth: false,
        yAxisIndex: idx,
        lineStyle: { color: s.color, width: 1.5 },
        itemStyle: { color: s.color },
        symbol: 'none',
        animation: false,
      }));
      return {
        animation: false,
        animationDuration: 0,
        progressive: 500,
        progressiveThreshold: 3000,
        lazyUpdate: true,
        tooltip: {
          trigger: 'axis',
          axisPointer: {
            type: 'cross',
            lineStyle: { color: 'var(--border-color)' },
            crossStyle: { color: 'var(--border-color)' },
          },
          backgroundColor: 'var(--bg-secondary)',
          borderColor: 'var(--border-color)',
          textStyle: { color: 'var(--text-primary)' },
          formatter: (params: unknown) => {
            const items = params as Array<{
              seriesName: string;
              value: number | number[];
              color: string;
              dataIndex?: number;
            }>;
            if (!Array.isArray(items)) return '';
            const timeValue = xAxisData[items[0]?.dataIndex ?? 0];
            let html = `<div style="font-weight: bold; margin-bottom: 4px;">${t('chart.time')}: ${formatTime(timeValue)}</div>`;
            items.forEach((item) => {
              const val = Array.isArray(item.value)
                ? item.value[1]
                : item.value;
              html += `<div style="display: flex; align-items: center; gap: 8px;"><span style="display: inline-block; width: 10px; height: 10px; background: ${item.color}; border-radius: 50%;"></span><span>${item.seriesName}: ${formatActualValue(val)}</span></div>`;
            });
            return html;
          },
        },
        legend: {
          top: 4,
          left: 'center',
          orient: 'horizontal',
          data: seriesData.map((s) => s.name),
          textStyle: { color: 'var(--text-primary)' },
        },
        grid: unifiedGridConfig,
        xAxis: {
          type: 'category',
          data: xAxisData,
          boundaryGap: false,
          splitNumber: 10,
          axisLine: { lineStyle: { color: 'var(--border-color)' } },
          axisLabel: {
            color: 'var(--text-secondary)',
            formatter: (value: string | number) => {
              const num = typeof value === 'number' ? value : parseFloat(value);
              return !isNaN(num) ? formatTime(num) : value;
            },
          },
        },
        yAxis,
        series,
        dataZoom: [
          {
            type: 'slider' as const,
            show: true,
            start: localDataZoom.start,
            end: localDataZoom.end,
            zoomLock: false,
            xAxisIndex: [0],
            height: 24,
            bottom: 8,
            handleStyle: { color: '#1890ff', borderColor: '#1890ff' },
            trackStyle: { backgroundColor: 'var(--bg-secondary)' },
            selectedDataBackground: {
              lineStyle: { color: '#1890ff' },
              areaStyle: { color: 'rgba(24, 144, 255, 0.2)' },
            },
            fillerColor: 'rgba(24, 144, 255, 0.15)',
            borderColor: 'var(--border-color)',
            textStyle: { color: 'var(--text-secondary)' },
            labelFormatter: (value: number) =>
              rows.length === 0
                ? '0ms'
                : formatTime(
                    Math.max(
                      0,
                      Math.min(
                        Math.round((value * (rows.length - 1)) / 100),
                        rows.length - 1,
                      ),
                    ) / sampleRate,
                  ),
          },
        ],
      };
    },
    [columns, rows, sampleRate, xAxisData, unifiedGridConfig, localDataZoom],
  );

  const replayLegendSelection = useCallback(
    (group: ChartGroupConfig, groupIndex: number, chart: echarts.ECharts) => {
      group.columns.slice(0, MAX_LINES_PER_CHART).forEach((column) => {
        const selected = resolveChartLegendSelection(
          legendScope,
          effectiveLegendSelected,
          group,
          groupIndex,
          column,
        );
        chart.dispatchAction(
          {
            type: getLegendAction(selected),
            name: column,
          },
          { silent: true },
        );
      });
    },
    [effectiveLegendSelected, legendScope],
  );

  useEffect(() => {
    const liveKeys = new Set(
      chartGroups.map((group, index) => getChartGroupKey(group, index)),
    );
    let membershipChanged = false;
    chartInstances.current.forEach((chart, key) => {
      const container = containerRefs.current.get(key);
      if (!liveKeys.has(key) || !container || chart.getDom() !== container) {
        chart.dispose();
        chartInstances.current.delete(key);
        structureSignatures.current.delete(key);
        membershipChanged = true;
      }
    });
    chartGroups.forEach((group, index) => {
      const key = getChartGroupKey(group, index);
      const container = containerRefs.current.get(key);
      if (container && !chartInstances.current.has(key)) {
        chartInstances.current.set(key, echarts.init(container));
        membershipChanged = true;
      }
    });
    if (membershipChanged) setInstanceRevision((revision) => revision + 1);
    setInitialized(true);
  }, [chartGroups, columns.length, rows.length]);

  useEffect(() => {
    if (!initialized) return;
    if (updateTimer.current) clearTimeout(updateTimer.current);
    updateTimer.current = setTimeout(() => {
      chartGroups.forEach((group, index) => {
        const groupKey = getChartGroupKey(group, index);
        const chart = chartInstances.current.get(groupKey);
        if (chart) {
          const selectedColumns = group.columns.slice(0, MAX_LINES_PER_CHART);
          const seriesStructure = buildChartSeries(
            columns,
            [],
            selectedColumns,
            MAX_LINES_PER_CHART,
          ).map((series) => series.name);
          const structureSignature = `${groupKey}:${selectedColumns.join('\u0000')}:${seriesStructure.join('\u0000')}`;
          const structureChanged =
            structureSignatures.current.get(groupKey) !== structureSignature;
          chart.setOption(
            getChartOption(group),
            structureChanged
              ? { notMerge: true, lazyUpdate: true }
              : { replaceMerge: ['series'], lazyUpdate: true },
          );
          structureSignatures.current.set(groupKey, structureSignature);
          replayLegendSelection(group, index, chart);
        }
      });
      updateTimer.current = null;
    }, 16);
    return () => {
      if (updateTimer.current) clearTimeout(updateTimer.current);
      updateTimer.current = null;
    };
  }, [
    initialized,
    chartGroups,
    getChartOption,
    replayLegendSelection,
    instanceRevision,
  ]);

  useEffect(() => {
    if (!initialized) return;
    chartGroups.forEach((group, index) => {
      const chart = chartInstances.current.get(getChartGroupKey(group, index));
      if (!chart) return;
      replayLegendSelection(group, index, chart);
    });
  }, [
    initialized,
    chartGroupKeySignature,
    replayLegendSelection,
    instanceRevision,
  ]);

  useEffect(() => {
    if (!initialized) return;
    const disposers: Array<() => void> = [];
    chartGroups.forEach((group, index) => {
      const key = getChartGroupKey(group, index);
      const chart = chartInstances.current.get(key);
      if (!chart) return;
      const handler = (params: unknown) =>
        handleDataZoomEvent(
          key,
          params,
          chartInstances.current,
          setLocalDataZoom,
          onDataZoomChangeRef.current,
        );
      chart.on('datazoom', handler);
      disposers.push(() => chart.off('datazoom', handler));
    });
    return () => disposers.forEach((dispose) => dispose());
  }, [initialized, chartGroupKeySignature, instanceRevision]);

  useEffect(() => {
    if (!initialized) return;
    const disposers: Array<() => void> = [];
    chartGroups.forEach((group, index) => {
      const key = getChartGroupKey(group, index);
      const chart = chartInstances.current.get(key);
      if (!chart) return;
      const handler = (params: unknown) => {
        const selected = (params as { selected?: Record<string, boolean> })
          .selected;
        if (!selected) return;
        const next = {
          ...(legendSelectedRef.current ?? localLegendSelectedRef.current),
        };
        Object.entries(selected).forEach(([name, value]) => {
          next[getChartLegendKey(legendScope, group, index, name)] = value;
        });
        if (legendSelectedRef.current === undefined)
          setLocalLegendSelected(next);
        onLegendSelectedChangeRef.current?.(next);
      };
      chart.on('legendselectchanged', handler);
      disposers.push(() => chart.off('legendselectchanged', handler));
    });
    return () => disposers.forEach((dispose) => dispose());
  }, [initialized, chartGroupKeySignature, legendScope, instanceRevision]);

  useEffect(() => {
    const handleResize = () =>
      chartInstances.current.forEach((chart) => chart.resize());
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  useEffect(() => {
    if (!initialized || typeof ResizeObserver === 'undefined') return;

    const containerKeys = new Map<Element, string>();
    chartGroups.forEach((group, index) => {
      const key = getChartGroupKey(group, index);
      const container = containerRefs.current.get(key);
      if (container) containerKeys.set(container, key);
    });

    const observer = new ResizeObserver((entries) => {
      entries.forEach(({ target }) => {
        const key = containerKeys.get(target);
        if (!key) return;
        chartInstances.current.get(key)?.resize();
      });
    });
    containerKeys.forEach((_, container) => observer.observe(container));

    return () => observer.disconnect();
  }, [
    initialized,
    instanceRevision,
    chartGroupKeySignature,
    chartGeometrySignature,
  ]);

  useEffect(() => {
    if (!initialized) return;
    chartInstances.current.forEach((chart) => chart.resize());
  }, [initialized, instanceRevision, chartGeometrySignature]);

  useEffect(
    () => () => {
      if (updateTimer.current) clearTimeout(updateTimer.current);
      chartInstances.current.forEach((chart) => chart.dispose());
      chartInstances.current.clear();
      structureSignatures.current.clear();
      containerRefs.current.clear();
    },
    [],
  );

  const handleContextMenu = useCallback(
    (groupKey: string) => (event: React.MouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
      setContextMenu({ x: event.clientX, y: event.clientY, groupKey });
    },
    [],
  );
  const saveChart = useCallback(
    (type: 'png' | 'svg') => {
      if (!contextMenu) return;
      const chart = chartInstances.current.get(contextMenu.groupKey);
      const group = chartGroups.find(
        (item, index) => getChartGroupKey(item, index) === contextMenu.groupKey,
      );
      const filename = `waveform_${group?.name || 'chart'}_${Date.now()}.${type}`;
      void exportSingleChart(
        chart,
        group,
        type,
        filename,
        onExportErrorRef.current,
      )
        .catch((error: unknown) => {
          console.error('Chart export failed', error);
        })
        .finally(() => setContextMenu(null));
    },
    [contextMenu, chartGroups],
  );

  const exportAllPng = useCallback(async (): Promise<void> => {
    try {
      await exportAllChartsPng(chartGroups, chartInstances.current, {
        onExportError: onExportErrorRef.current,
      });
    } catch (error) {
      throw toExportError(error);
    }
  }, [chartGroups]);

  useImperativeHandle(ref, () => ({ exportAllPng }), [exportAllPng]);
  useEffect(() => {
    if (!contextMenu) return;
    const groupIsLive = chartGroups.some(
      (group, index) => getChartGroupKey(group, index) === contextMenu.groupKey,
    );
    if (!groupIsLive || !chartInstances.current.has(contextMenu.groupKey))
      setContextMenu(null);
  }, [
    contextMenu,
    chartGroupKeySignature,
    initialized,
    columns.length,
    rows.length,
    instanceRevision,
  ]);
  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    document.addEventListener('click', close);
    return () => document.removeEventListener('click', close);
  }, [contextMenu]);

  if (columns.length === 0 || rows.length === 0)
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
  if (chartGroups.length === 0)
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
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'auto',
      }}
    >
      {chartGroups.map((group, index) => {
        const key = getChartGroupKey(group, index);
        return (
          <div
            key={key}
            ref={(element) => {
              if (element) containerRefs.current.set(key, element);
              else containerRefs.current.delete(key);
            }}
            className="chart-container"
            style={{
              width: '100%',
              height: normalizeChartHeight(group.height),
              minHeight: 150,
              flexShrink: 0,
              borderBottom:
                index < chartGroups.length - 1
                  ? '1px solid var(--border-color)'
                  : 'none',
            }}
            onContextMenu={handleContextMenu(key)}
          />
        );
      })}
      {contextMenu && (
        <div
          className="chart-context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onClick={(event) => event.stopPropagation()}
        >
          <div
            className="chart-context-menu-item"
            onClick={(event) => {
              event.stopPropagation();
              saveChart('png');
            }}
          >
            {getChartExportMenuLabel('png', translate)}
          </div>
          <div
            className="chart-context-menu-item"
            onClick={(event) => {
              event.stopPropagation();
              saveChart('svg');
            }}
          >
            {getChartExportMenuLabel('svg', translate)}
          </div>
        </div>
      )}
    </div>
  );
});

function t(key: string): string {
  return ({ 'chart.time': '时间' } as Record<string, string>)[key] || key;
}

export function getChartExportMenuLabel(
  type: 'png' | 'svg',
  translate: (key: string) => string,
): string {
  return translate(type === 'png' ? 'chart.saveAsPng' : 'chart.saveAsSvg');
}

export async function exportChart(
  chart: echarts.ECharts,
  type: 'png' | 'svg',
  filename: string,
  onExportError?: (error: Error) => void,
): Promise<void> {
  try {
    await waitForChartRender();
    const url = type === 'svg'
      ? exportSvgChart(chart)
      : chart.getDataURL({
          type,
          pixelRatio: 2,
          backgroundColor: '#fff',
        });
    downloadBlob(dataUrlToBlob(url), filename);
  } catch (error) {
    const exportError = toExportError(error);
    onExportError?.(exportError);
    throw exportError;
  }
}

export async function exportSingleChart(
  chart: echarts.ECharts | undefined,
  group: ChartGroupConfig | undefined,
  type: 'png' | 'svg',
  filename: string,
  onExportError?: (error: Error) => void,
): Promise<void> {
  try {
    if (!chart || !group)
      throw new Error('Cannot export chart: chart or group is unavailable');
    await exportChart(chart, type, filename);
  } catch (error) {
    const exportError = toExportError(error);
    onExportError?.(exportError);
    throw exportError;
  }
}

export async function exportAllChartsPng(
  chartGroups: readonly ChartGroupConfig[],
  chartInstances: ReadonlyMap<string, ChartExportInstance>,
  dependencies: Partial<ChartExportDependencies> = {},
): Promise<void> {
  const {
    composeChartPng: compose = composeChartPng,
    downloadBlob: download = downloadBlob,
    waitForRender = waitForChartRender,
    now = Date.now,
    onExportError,
  } = dependencies;

  try {
    await waitForRender();
    if (chartGroups.length === 0)
      throw new Error('Cannot export all charts: no valid chart instances');

    const charts = chartGroups.map((group, index) => {
      const chart = chartInstances.get(getChartGroupKey(group, index));
      if (!chart)
        throw new Error(
          `Cannot export chart PNG: missing chart instance for group "${group.name || index + 1}"`,
        );
      return { group, chart };
    });
    const validatedCharts = charts.map(({ group, chart }) => {
      const width = chart.getWidth();
      const height = chart.getHeight();
      if (
        !Number.isFinite(width) ||
        !Number.isFinite(height) ||
        width <= 0 ||
        height <= 0
      )
        throw new Error(
          `Cannot export chart PNG: chart dimensions are invalid for group "${group.name || 'chart'}"`,
        );
      return { chart, width, height };
    });
    const dataUrls = validatedCharts.map(({ chart }) => {
      chart.resize();
      return chart.getDataURL({
        type: 'png',
        pixelRatio: 2,
        backgroundColor: '#fff',
      });
    });

    const output = await compose(dataUrls, { gap: 8 });
    download(output.blob, `waveform_all_${now()}.png`);
  } catch (error) {
    const exportError = toExportError(error);
    onExportError?.(exportError);
    throw exportError;
  }
}

function exportSvgChart(chart: echarts.ECharts): string {
  const liveDom = chart.getDom();
  const width = Math.max(1, chart.getWidth(), liveDom.clientWidth);
  const height = Math.max(1, chart.getHeight(), liveDom.clientHeight);
  const liveStyle = window.getComputedStyle(liveDom);
  const rootStyle = document.documentElement
    ? window.getComputedStyle(document.documentElement)
    : null;
  const resolveCssVariable = (name: string): string | undefined => {
    const liveValue = liveStyle.getPropertyValue(name).trim();
    if (liveValue) return liveValue;
    const rootValue = rootStyle?.getPropertyValue(name).trim();
    return rootValue || undefined;
  };
  const container = document.createElement('div');
  container.style.position = 'absolute';
  container.style.left = '-10000px';
  container.style.top = '-10000px';
  container.style.width = `${width}px`;
  container.style.height = `${height}px`;

  let temporaryChart: echarts.ECharts | null = null;
  try {
    document.body.appendChild(container);
    temporaryChart = echarts.init(container, undefined, {
      renderer: 'svg',
      width,
      height,
    });
    const resolvedOption = resolveCssVariablesInValue(
      chart.getOption(),
      resolveCssVariable,
    ) as Record<string, unknown>;
    const exportOption = { ...resolvedOption, backgroundColor: '#fff' };
    temporaryChart.setOption(exportOption);
    return ensureWhiteSvgBackground(
      temporaryChart.getDataURL({ type: 'svg' }),
      width,
      height,
    );
  } finally {
    temporaryChart?.dispose();
    container.remove();
  }
}

export default React.memo(MultiLineChart);

function toExportError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

function waitForChartRender(): Promise<void> {
  return new Promise((resolve) => {
    const finish = () => setTimeout(resolve, 0);
    if (typeof requestAnimationFrame === 'function') requestAnimationFrame(finish);
    else finish();
  });
}
