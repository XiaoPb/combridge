import React, { createRef } from 'react';
import { describe, expect, it, vi } from 'vitest';
import type { MultiLineChartHandle, MultiLineChartProps } from './MultiLineChart';
import type { ChartGroupConfig } from './chartGroup';
import { getChartGroupKey } from './chartGroup';

const columns = ['heart_rate'];
const rows = [[72]];
const groups: ChartGroupConfig[] = [{ name: 'Heart rate', columns, height: 300 }];

describe('MultiLineChart data zoom synchronization', () => {
  it('dispatches sibling zoom updates silently', async () => {
    const chartModule = await import('./MultiLineChart');
    const dispatchDataZoomSilently = (chartModule as Record<string, unknown>)
      .dispatchDataZoomSilently as (
        chart: { dispatchAction: (...args: unknown[]) => void },
        state: { start: number; end: number },
      ) => void;
    const chart = { dispatchAction: vi.fn() };

    expect(typeof dispatchDataZoomSilently).toBe('function');
    dispatchDataZoomSilently(chart, { start: 12, end: 68 });

    expect(chart.dispatchAction).toHaveBeenCalledWith(
      { type: 'dataZoom', start: 12, end: 68 },
      { silent: true },
    );
  });

  it('processes consecutive user zoom events without synchronizing them back as a loop', async () => {
    const chartModule = await import('./MultiLineChart');
    const handleDataZoomEvent = (chartModule as Record<string, unknown>)
      .handleDataZoomEvent as (
      chartKey: string,
      params: unknown,
      chartInstances: ReadonlyMap<
        string,
        { dispatchAction: (...args: unknown[]) => void }
      >,
      setLocalDataZoom: (state: { start: number; end: number }) => void,
      onDataZoomChange?: (state: { start: number; end: number }) => void,
    ) => void;
    const source = { dispatchAction: vi.fn() };
    const sibling = { dispatchAction: vi.fn() };
    const chartInstances = new Map([
      ['source', source],
      ['sibling', sibling],
    ]);
    const setLocalDataZoom = vi.fn();
    const onDataZoomChange = vi.fn();

    expect(typeof handleDataZoomEvent).toBe('function');

    handleDataZoomEvent(
      'source',
      { start: 10, end: 90 },
      chartInstances,
      setLocalDataZoom,
      onDataZoomChange,
    );
    handleDataZoomEvent(
      'source',
      { batch: [{ start: 20, end: 80 }] },
      chartInstances,
      setLocalDataZoom,
      onDataZoomChange,
    );

    expect(setLocalDataZoom).toHaveBeenNthCalledWith(1, { start: 10, end: 90 });
    expect(setLocalDataZoom).toHaveBeenNthCalledWith(2, { start: 20, end: 80 });
    expect(onDataZoomChange).toHaveBeenNthCalledWith(1, {
      start: 10,
      end: 90,
    });
    expect(onDataZoomChange).toHaveBeenNthCalledWith(2, {
      start: 20,
      end: 80,
    });
    expect(sibling.dispatchAction).toHaveBeenCalledTimes(2);
    expect(sibling.dispatchAction).toHaveBeenNthCalledWith(
      1,
      { type: 'dataZoom', start: 10, end: 90 },
      { silent: true },
    );
    expect(sibling.dispatchAction).toHaveBeenNthCalledWith(
      2,
      { type: 'dataZoom', start: 20, end: 80 },
      { silent: true },
    );
    expect(source.dispatchAction).not.toHaveBeenCalled();
  });
});

describe('MultiLineChart export API', () => {
  it('exposes an exportAllPng method through its ref handle', async () => {
    const chartModule = await import('./MultiLineChart');
    const component = chartModule.default as unknown as {
      $$typeof: symbol;
      type?: { $$typeof?: symbol };
    };
    const ref = createRef<MultiLineChartHandle>();
    const onExportError = vi.fn();
    const props = {
      columns,
      rows,
      chartGroups: groups,
      onExportError,
    } satisfies MultiLineChartProps;
    const element = React.createElement(chartModule.default, { ...props, ref });

    expect(component.$$typeof).toBe(Symbol.for('react.memo'));
    expect(component.type?.$$typeof).toBe(Symbol.for('react.forward_ref'));
    expect(element.props.ref).toBe(ref);
  });

  it('reports single-chart export failures to the error callback', async () => {
    const chartModule = await import('./MultiLineChart');
    const exportChart = (chartModule as Record<string, unknown>)
      .exportChart as (
      chart: { getDataURL: () => string },
      type: 'png' | 'svg',
      filename: string,
      onExportError?: (error: Error) => void,
    ) => Promise<void>;
    const onExportError = vi.fn();
    const chart = {
      getDataURL: vi.fn(() => {
        throw new Error('chart export failed');
      }),
    };

    await expect(
      exportChart(chart, 'png', 'chart.png', onExportError),
    ).rejects.toThrow('chart export failed');
    expect(onExportError).toHaveBeenCalledWith(expect.any(Error));
  });

  it.each([
    ['missing chart', undefined, groups[0]],
    ['missing group', { getDataURL: vi.fn() }, undefined],
  ])('reports %s instead of silently returning', async (_label, chart, group) => {
    const chartModule = await import('./MultiLineChart');
    const exportSingleChart = (chartModule as Record<string, unknown>)
      .exportSingleChart as (
      chart: { getDataURL: () => string } | undefined,
      group: typeof groups[number] | undefined,
      type: 'png' | 'svg',
      filename: string,
      onExportError?: (error: Error) => void,
    ) => Promise<void>;
    const onExportError = vi.fn();

    await expect(
      exportSingleChart(chart, group, 'png', 'chart.png', onExportError),
    ).rejects.toThrow('Cannot export chart: chart or group is unavailable');
    expect(onExportError).toHaveBeenCalledWith(
      expect.objectContaining({
        message: 'Cannot export chart: chart or group is unavailable',
      }),
    );
  });

  it('exports all charts in chartGroups order with fixed PNG options and filename', async () => {
    const chartModule = await import('./MultiLineChart');
    const exportAllChartsPng = (chartModule as Record<string, unknown>)
      .exportAllChartsPng as (
      chartGroups: readonly ChartGroupConfig[],
      chartInstances: ReadonlyMap<string, {
        resize: () => void;
        getWidth: () => number;
        getHeight: () => number;
        getDataURL: (options: {
          type: 'png';
          pixelRatio: number;
          backgroundColor: string;
        }) => string;
      }>,
      dependencies: {
        composeChartPng: (dataUrls: readonly string[], options: {
          gap: number;
        }) => Promise<{ blob: Blob }>;
        downloadBlob: (blob: Blob, filename: string) => void;
        waitForRender: () => Promise<void>;
        now: () => number;
        onExportError?: (error: Error) => void;
      },
    ) => Promise<void>;
    const calls: string[] = [];
    const first = {
      resize: vi.fn(() => calls.push('first.resize')),
      getWidth: vi.fn(() => 320),
      getHeight: vi.fn(() => 180),
      getDataURL: vi.fn(() => {
        calls.push('first.getDataURL');
        return 'first-png';
      }),
    };
    const second = {
      resize: vi.fn(() => calls.push('second.resize')),
      getWidth: vi.fn(() => 320),
      getHeight: vi.fn(() => 240),
      getDataURL: vi.fn(() => {
        calls.push('second.getDataURL');
        return 'second-png';
      }),
    };
    const chartGroups: ChartGroupConfig[] = [
      { name: 'First', columns: ['a'], id: 'first' },
      { name: 'Second', columns: ['b'], id: 'second' },
    ];
    const chartInstances = new Map([
      [getChartGroupKey(chartGroups[0], 0), first],
      [getChartGroupKey(chartGroups[1], 1), second],
    ]);
    const composeChartPng = vi.fn(async (dataUrls, options) => {
      expect(dataUrls).toEqual(['first-png', 'second-png']);
      expect(options).toEqual({ gap: 8 });
      return { blob: new Blob(['all'], { type: 'image/png' }) };
    });
    const downloadBlob = vi.fn();
    const timestamp = 1700000000000;

    await exportAllChartsPng(chartGroups, chartInstances, {
      composeChartPng,
      downloadBlob,
      waitForRender: vi.fn(async () => undefined),
      now: () => timestamp,
    });

    expect(calls).toEqual([
      'first.resize',
      'first.getDataURL',
      'second.resize',
      'second.getDataURL',
    ]);
    expect(first.getDataURL).toHaveBeenCalledWith({
      type: 'png',
      pixelRatio: 2,
      backgroundColor: '#fff',
    });
    expect(second.getDataURL).toHaveBeenCalledWith({
      type: 'png',
      pixelRatio: 2,
      backgroundColor: '#fff',
    });
    expect(composeChartPng).toHaveBeenCalledWith(
      ['first-png', 'second-png'],
      { gap: 8 },
    );
    expect(downloadBlob).toHaveBeenCalledWith(
      expect.any(Blob),
      `waveform_all_${timestamp}.png`,
    );
  });

  it('rejects all-chart export when there are no valid instances or dimensions', async () => {
    const chartModule = await import('./MultiLineChart');
    const exportAllChartsPng = (chartModule as Record<string, unknown>)
      .exportAllChartsPng as (
      groups: readonly ChartGroupConfig[],
      instances: ReadonlyMap<string, unknown>,
      dependencies?: { waitForRender?: () => Promise<void>; onExportError?: (error: Error) => void },
    ) => Promise<void>;
    const emptyGroups: ChartGroupConfig[] = [{ name: 'Only', columns: ['a'], id: 'only' }];
    const onExportError = vi.fn();
    const dependencies = {
      waitForRender: vi.fn(async () => undefined),
      onExportError,
    };

    await expect(exportAllChartsPng(emptyGroups, new Map(), dependencies)).rejects.toThrow(
      'Cannot export chart PNG: missing chart instance for group "Only"',
    );
    expect(onExportError).toHaveBeenCalledWith(
      expect.objectContaining({
        message: 'Cannot export chart PNG: missing chart instance for group "Only"',
      }),
    );

    const first = {
      resize: vi.fn(),
      getWidth: () => 320,
      getHeight: () => 180,
      getDataURL: vi.fn(),
    };
    const partialGroups: ChartGroupConfig[] = [
      { name: 'First', columns: ['a'], id: 'first' },
      { name: 'Second', columns: ['b'], id: 'second' },
    ];
    const partialError = vi.fn();
    await expect(
      exportAllChartsPng(
        partialGroups,
        new Map([
          [getChartGroupKey(partialGroups[0], 0), first],
        ]),
        { waitForRender: vi.fn(async () => undefined), onExportError: partialError },
      ),
    ).rejects.toThrow('Cannot export chart PNG: missing chart instance for group "Second"');
    expect(partialError).toHaveBeenCalledWith(
      expect.objectContaining({
        message: 'Cannot export chart PNG: missing chart instance for group "Second"',
      }),
    );
    expect(first.resize).not.toHaveBeenCalled();

    const invalidError = vi.fn();
    await expect(
      exportAllChartsPng(
        emptyGroups,
        new Map([
          [getChartGroupKey(emptyGroups[0], 0), {
            resize: vi.fn(),
            getWidth: () => 0,
            getHeight: () => 100,
            getDataURL: vi.fn(),
          }],
        ]),
        { waitForRender: vi.fn(async () => undefined), onExportError: invalidError },
      ),
    ).rejects.toThrow('Cannot export chart PNG: chart dimensions are invalid');
    expect(invalidError).toHaveBeenCalledWith(
      expect.objectContaining({
        message: 'Cannot export chart PNG: chart dimensions are invalid for group "Only"',
      }),
    );
  });
});
