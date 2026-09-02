import { describe, expect, it } from 'vitest';
import {
  calculateVisibleLineStats,
  getVisibleRowRange,
} from './multiLineChartStats';
import { LINE_COLORS } from './multiLineChartModel';

describe('getVisibleRowRange', () => {
  it('converts percentages to an inclusive row range', () => {
    expect(getVisibleRowRange(10, { start: 0, end: 5 })).toEqual({
      startIndex: 0,
      endIndex: 1,
    });
    expect(getVisibleRowRange(10, { start: 11, end: 22 })).toEqual({
      startIndex: 0,
      endIndex: 2,
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
    expect(
      getVisibleRowRange(3, { start: 0, end: Number.POSITIVE_INFINITY }),
    ).toEqual({
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
    const visibleRows = [
      [100, 1000, 0, 0, 0],
      [3, 20, 0, 0, 0],
      [5, 40, 0, 0, 0],
      [7, 80, 0, 0, 0],
      [-100, -1000, 0, 0, 0],
    ];

    expect(
      calculateVisibleLineStats(
        columns,
        visibleRows,
        ['A', 'B'],
        { start: 25, end: 50 },
      ),
    )
      .toEqual([
        { name: 'A', color: LINE_COLORS[0], max: 5, min: 3, avg: 4, diff: 2 },
        { name: 'B', color: LINE_COLORS[1], max: 40, min: 20, avg: 30, diff: 20 },
      ]);
  });

  it('calculates statistics from the single middle row when zoom is collapsed', () => {
    const visibleRows = [
      [100, 1000],
      [3, 20],
      [5, 40],
      [7, 80],
      [-100, -1000],
    ];

    expect(
      calculateVisibleLineStats(
        ['A', 'B'],
        visibleRows,
        ['A', 'B'],
        { start: 50, end: 50 },
      ),
    )
      .toEqual([
        { name: 'A', color: LINE_COLORS[0], max: 5, min: 5, avg: 5, diff: 0 },
        { name: 'B', color: LINE_COLORS[1], max: 40, min: 40, avg: 40, diff: 0 },
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
    expect(result[1]).toMatchObject({
      name: 'B',
      max: 40,
      min: 10,
      diff: 30,
    });
    expect(result[1].avg).toBeCloseTo(70 / 3, 12);
    expect(result.map((item) => item.name)).toEqual(['A', 'B', 'C', 'D']);
  });

  it('applies the four-line limit before deduplicating', () => {
    const result = calculateVisibleLineStats(
      columns,
      rows,
      ['A', 'A', 'B', 'C', 'D'],
      { start: 0, end: 100 },
    );

    expect(result.map((item) => item.name)).toEqual(['A', 'B', 'C']);
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

  it('keeps the average finite for very large values', () => {
    const [stat] = calculateVisibleLineStats(
      ['A'],
      [[1e308], [1e308]],
      ['A'],
      { start: 0, end: 100 },
    );

    expect(stat.avg).toBe(1e308);
    expect(Number.isFinite(stat.avg)).toBe(true);
  });

  it('normalizes maxLines to a finite integer in the range zero through four', () => {
    const getStats = (maxLines: number) => calculateVisibleLineStats(
      ['A', 'B', 'C', 'D', 'E'],
      [[1, 2, 3, 4, 5]],
      ['A', 'B', 'C', 'D', 'E'],
      { start: 0, end: 100 },
      maxLines,
    );

    expect(getStats(Number.NaN)).toHaveLength(4);
    expect(getStats(Number.POSITIVE_INFINITY)).toHaveLength(4);
    expect(getStats(2.9)).toHaveLength(2);
    expect(getStats(-1)).toHaveLength(0);
  });

  it('returns null values when a line has no finite values', () => {
    expect(
      calculateVisibleLineStats(
        ['A'],
        [[Number.NaN], [Number.POSITIVE_INFINITY]],
        ['A'],
        { start: 0, end: 100 },
      ),
    ).toEqual([
      {
        name: 'A',
        color: LINE_COLORS[0],
        max: null,
        min: null,
        avg: null,
        diff: null,
      },
    ]);
  });

  it('returns null values for a missing column', () => {
    expect(
      calculateVisibleLineStats(
        ['A'],
        [[1], [2]],
        ['B'],
        { start: 0, end: 100 },
      ),
    ).toEqual([
      {
        name: 'B',
        color: LINE_COLORS[0],
        max: null,
        min: null,
        avg: null,
        diff: null,
      },
    ]);
  });
});
