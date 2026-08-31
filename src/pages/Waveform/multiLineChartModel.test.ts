import { describe, expect, it } from 'vitest';
import { LINE_COLORS, buildChartSeries } from './multiLineChartModel';

describe('buildChartSeries', () => {
  it('builds one independent series and color per selected column', () => {
    const series = buildChartSeries(['A', 'B', 'C', 'D'], [[0, 0, 0, 0]], ['A', 'B', 'C', 'D']);

    expect(series.map(({ data }) => data)).toEqual([[0], [0], [0], [0]]);
    expect(new Set(series.map(({ color }) => color)).size).toBe(4);
  });

  it('returns independent mutable data and objects for separate charts', () => {
    const first = buildChartSeries(['A'], [[1]], ['A']);
    const second = buildChartSeries(['A'], [[1]], ['A']);

    expect(first[0]).not.toBe(second[0]);
    expect(first[0].data).not.toBe(second[0].data);
    first[0].data[0] = 99;
    expect(second[0].data).toEqual([1]);
  });

  it('uses zero when a selected column has no cell in a row', () => {
    const series = buildChartSeries(['A', 'B'], [[5]], ['A', 'B']);

    expect(series.map(({ data }) => data)).toEqual([[5], [0]]);
  });

  it('limits to four lines by default while preserving group order', () => {
    const series = buildChartSeries(['A', 'B', 'C', 'D', 'E'], [[1, 2, 3, 4, 5]], ['D', 'B', 'E', 'A', 'C']);

    expect(series.map(({ name, data }) => [name, data])).toEqual([
      ['D', [4]],
      ['B', [2]],
      ['E', [5]],
      ['A', [1]],
    ]);
  });

  it('assigns colors by local line index rather than global column index', () => {
    const series = buildChartSeries(['A', 'B', 'C', 'D', 'E', 'F'], [[1, 2, 3, 4, 5, 6]], ['E', 'F']);

    expect(series.map(({ color }) => color)).toEqual([LINE_COLORS[0], LINE_COLORS[1]]);
  });
});
