import { describe, expect, it } from 'vitest';
import { makeUniqueColumnNames, parseCsv } from './csvParser';

describe('parseCsv column identity', () => {
  it('uniquifies duplicate headers while preserving row values', () => {
    const result = parseCsv('CH0,CH0,ACC_X\n1,2,3', { skipInfoRows: 0 });

    expect(result.columns).toEqual(['CH0', 'CH0 (2)', 'ACC_X']);
    expect(result.rows).toEqual([[1, 2, 3]]);
  });

  it('keeps short rows unchanged for the chart layer to fill', () => {
    const result = parseCsv('A,B,C\n1,2', { skipInfoRows: 0 });

    expect(result.rows).toEqual([[1, 2]]);
  });

  it('handles repeated blank and generated-looking headers deterministically', () => {
    expect(makeUniqueColumnNames(['', '', '未命名列', '未命名列', 'A', 'A', 'A (2)', 'A'])).toEqual([
      '未命名列',
      '未命名列 (2)',
      '未命名列 (3)',
      '未命名列 (4)',
      'A',
      'A (2)',
      'A (2) (2)',
      'A (3)',
    ]);
  });
});
