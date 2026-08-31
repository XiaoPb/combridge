import { describe, expect, it } from 'vitest';
import type { ChartGroupConfig } from '../Waveform/chartGroup';
import {
  appendGh3036ChartGroup,
  normalizeGh3036ChartGroups,
} from './gh3036ChartGroup';

describe('GH3036 chart group identity', () => {
  it('keeps a surviving ID distinct from a newly added group after deletion', () => {
    const initial: ChartGroupConfig[] = [
      { id: 'id0', name: '图表 1', columns: ['CH0'] },
      { id: 'id1', name: '图表 2', columns: ['CH1'] },
    ];

    const afterDelete = initial.filter((group) => group.id !== 'id0');
    const afterAdd = appendGh3036ChartGroup(afterDelete, '图表 2');

    expect(afterAdd[0].id).toBe('id1');
    expect(afterAdd[1].id).not.toBe(afterAdd[0].id);
  });

  it('does not reuse an allocated ID after its group is deleted', () => {
    const initial = normalizeGh3036ChartGroups([
      { name: '图表 1', columns: ['CH0'] },
      { name: '图表 2', columns: ['CH1'] },
    ]);

    const removedId = initial[0].id;
    const afterDelete = initial.filter((group) => group.id !== removedId);
    const afterAdd = appendGh3036ChartGroup(afterDelete, '图表 3');

    expect(afterAdd[0].id).toBe(initial[1].id);
    expect(afterAdd[1].id).not.toBe(removedId);
    expect(afterAdd[1].id).not.toBe(initial[1].id);
  });

  it('normalizes missing and existing IDs without collisions', () => {
    const groups: ChartGroupConfig[] = [
      { id: 'gh3036-data-0', name: '已有', columns: [] },
      { name: '缺失', columns: [] },
      { id: 'gh3036-data-1', name: '已有 2', columns: [] },
    ];

    const normalized = normalizeGh3036ChartGroups(groups);
    const ids = normalized.map((group) => group.id);

    expect(ids[0]).toBe('gh3036-data-0');
    expect(ids[2]).toBe('gh3036-data-1');
    expect(ids[1]).not.toBe(ids[0]);
    expect(ids[1]).not.toBe(ids[2]);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
