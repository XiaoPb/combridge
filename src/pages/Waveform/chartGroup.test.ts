import { describe, expect, it } from 'vitest';

import {
  createChartGroup,
  getChartGroupKey,
  getNextChartGroupName,
} from './chartGroup';

describe('chart group identity', () => {
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
