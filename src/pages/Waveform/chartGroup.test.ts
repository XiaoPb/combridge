import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  createChartGroup,
  getChartGroupKey,
  getNextChartGroupName,
} from './chartGroup';

describe('chart group identity', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('uses empty columns and 300 height by default', () => {
    const group = createChartGroup('图表1');

    expect(group.columns).toEqual([]);
    expect(group.height).toBe(300);
  });

  it('clones the input columns', () => {
    const columns = ['CH0'];
    const group = createChartGroup('图表1', columns);

    columns.push('CH1');

    expect(group.columns).toEqual(['CH0']);
  });

  it('uses distinct timestamp-counter IDs when randomUUID is unavailable', () => {
    vi.stubGlobal('crypto', {});

    const first = createChartGroup('图表1');
    const second = createChartGroup('图表1');

    expect(first.id).toMatch(/^chart-\d+-\d+$/);
    expect(second.id).toMatch(/^chart-\d+-\d+$/);
    expect(first.id).not.toBe(second.id);
  });

  it('creates distinct stable IDs for chart groups with the same name', () => {
    const first = createChartGroup('图表1');
    const second = createChartGroup('图表1');

    expect(first.id).not.toBe(second.id);
    expect(getChartGroupKey(first, 0)).not.toBe(getChartGroupKey(second, 0));
  });

  it('returns the smallest unused chart group name', () => {
    expect(getNextChartGroupName([createChartGroup('图表1'), createChartGroup('图表3')])).toBe('图表2');
  });

  it('uses the legacy key format for groups without an ID', () => {
    expect(getChartGroupKey({ name: 'PPG', columns: ['CH0'] }, 2)).toBe('legacy:2:PPG');
  });
});
