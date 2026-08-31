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
});
