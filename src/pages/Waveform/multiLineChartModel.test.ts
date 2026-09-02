import { describe, expect, it } from 'vitest';
import {
  LINE_COLORS,
  MAX_LINES_PER_CHART,
  buildChartSeries,
} from './multiLineChartModel';

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

  it('uses zero for an in-range sparse cell', () => {
    const sparseRows = [[5, undefined] as unknown as number[]];
    const series = buildChartSeries(['A', 'B'], sparseRows, ['A', 'B']);

    expect(series.map(({ data }) => data)).toEqual([[5], [0]]);
  });

  it('limits to four lines by default while preserving group order', () => {
    const series = buildChartSeries(['A', 'B', 'C', 'D', 'E'], [[1, 2, 3, 4, 5]], ['D', 'B', 'E', 'A', 'C']);

    expect(series).toHaveLength(MAX_LINES_PER_CHART);
    expect(series.map(({ name, data }) => [name, data])).toEqual([
      ['D', [4]],
      ['B', [2]],
      ['E', [5]],
      ['A', [1]],
    ]);
  });

  it('normalizes maxLines before applying the existing slice and deduplication', () => {
    const columns = ['A', 'B', 'C', 'D', 'E'];
    const rows = [[1, 2, 3, 4, 5]];
    const groupColumns = ['A', 'B', 'C', 'D', 'E'];
    const buildWithMaxLines = (maxLines: number) =>
      buildChartSeries(columns, rows, groupColumns, maxLines);

    expect(buildWithMaxLines(0)).toHaveLength(0);
    expect(buildWithMaxLines(-1)).toHaveLength(0);
    expect(buildWithMaxLines(2.9)).toHaveLength(2);
    expect(buildWithMaxLines(Number.NaN)).toHaveLength(MAX_LINES_PER_CHART);
    expect(buildWithMaxLines(Number.POSITIVE_INFINITY)).toHaveLength(MAX_LINES_PER_CHART);
    expect(buildWithMaxLines(10)).toHaveLength(MAX_LINES_PER_CHART);
  });

  it('applies the line limit before deduplicating group columns', () => {
    const series = buildChartSeries(
      ['A', 'B', 'C', 'D'],
      [[1, 2, 3, 4]],
      ['A', 'A', 'B', 'C', 'D'],
    );

    expect(series.map(({ name }) => name)).toEqual(['A', 'B', 'C']);
  });

  it('assigns colors by local line index rather than global column index', () => {
    const series = buildChartSeries(['A', 'B', 'C', 'D', 'E', 'F'], [[1, 2, 3, 4, 5, 6]], ['E', 'F']);

    expect(series.map(({ color }) => color)).toEqual([LINE_COLORS[0], LINE_COLORS[1]]);
  });
});
