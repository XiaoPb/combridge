import { describe, expect, it } from 'vitest';
import {
  calculateVisibleLineStats,
  getVisibleRowRange,
} from './multiLineChartStats';

describe('getVisibleRowRange', () => {
  it('converts percentages to an inclusive row range', () => {
    expect(getVisibleRowRange(5, { start: 25, end: 75 })).toEqual({
      startIndex: 1,
      endIndex: 3,
    });
  });

  it('clamps invalid percentages and uses all rows when zoom is invalid', () => {
    expect(getVisibleRowRange(3, { start: -10, end: 150 })).toEqual({
      startIndex: 0,
      endIndex: 2,
    });
    expect(getVisibleRowRange(3, { start: Number.NaN, end: 50 })).toEqual({
      startIndex: 0,
      endIndex: 2,
    });
    expect(getVisibleRowRange(5, { start: 75, end: 25 })).toEqual({
      startIndex: 1,
      endIndex: 3,
    });
    expect(getVisibleRowRange(3, { start: 0, end: Number.POSITIVE_INFINITY })).toEqual({
      startIndex: 0,
      endIndex: 2,
    });
  });

  it('returns null for empty rows', () => {
    expect(getVisibleRowRange(0, { start: 0, end: 100 })).toBeNull();
  });
});

describe('calculateVisibleLineStats', () => {
  const columns = ['A', 'B', 'C', 'D', 'E'];
  const rows = [
    [1, 10, Number.NaN, 100, 5],
    [3, 20, 4, 80, 7],
    [5, Number.POSITIVE_INFINITY, 8, 60, 9],
    [7, 40, 12, 40, 11],
  ];

  it('calculates max, min, average, and diff only in the visible range', () => {
    expect(calculateVisibleLineStats(columns, rows, ['A', 'B'], { start: 25, end: 75 }))
      .toEqual([
        { name: 'A', color: '#165DFF', max: 5, min: 3, avg: 4, diff: 2 },
        { name: 'B', color: '#F53F3F', max: 20, min: 20, avg: 20, diff: 0 },
      ]);
  });

  it('filters non-finite values and limits each chart to four unique lines', () => {
    const result = calculateVisibleLineStats(
      columns,
      rows,
      ['A', 'B', 'C', 'D', 'E', 'A'],
      { start: 0, end: 100 },
    );
    expect(result).toHaveLength(4);
    expect(result[1]).toMatchObject({ name: 'B', max: 40, min: 10, avg: 70 / 3, diff: 30 });
    expect(result.map((item) => item.name)).toEqual(['A', 'B', 'C', 'D']);
  });

  it('deduplicates before applying the four-line limit', () => {
    const result = calculateVisibleLineStats(
      columns,
      rows,
      ['A', 'A', 'B', 'C', 'D'],
      { start: 0, end: 100 },
    );

    expect(result.map((item) => item.name)).toEqual(['A', 'B', 'C', 'D']);
  });

  it('keeps the hard four-line limit when maxLines is larger', () => {
    const result = calculateVisibleLineStats(
      columns,
      rows,
      ['A', 'B', 'C', 'D', 'E'],
      { start: 0, end: 100 },
      5,
    );

    expect(result).toHaveLength(4);
  });

  it('returns null values when a line has no finite values', () => {
    expect(calculateVisibleLineStats(['A'], [[Number.NaN], [Number.POSITIVE_INFINITY]], ['A'], { start: 0, end: 100 }))
      .toEqual([{ name: 'A', color: '#165DFF', max: null, min: null, avg: null, diff: null }]);
  });

  it('returns null values for a missing column', () => {
    expect(calculateVisibleLineStats(['A'], [[1], [2]], ['B'], { start: 0, end: 100 }))
      .toEqual([{ name: 'B', color: '#165DFF', max: null, min: null, avg: null, diff: null }]);
  });
});
