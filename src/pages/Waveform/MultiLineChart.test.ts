import { describe, expect, it, vi } from 'vitest';

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
