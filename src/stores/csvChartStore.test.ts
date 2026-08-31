import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { CsvParseResult } from '../utils/csvParser';
import { createChartGroup } from '../pages/Waveform/chartGroup';

const { readCsvFile } = vi.hoisted(() => ({
  readCsvFile: vi.fn<(filePath: string, config: unknown) => Promise<CsvParseResult>>(),
}));
vi.mock('../utils/csvParser', async () => {
  const actual = await vi.importActual<typeof import('../utils/csvParser')>('../utils/csvParser');
  return { ...actual, readCsvFile };
});

import { useCsvChartStore } from './csvChartStore';

const result = (columns: string[], rows: number[][] = [[1]]) => ({ columns, rows });

describe('CSV chart store isolation and load ordering', () => {
  beforeEach(() => {
    readCsvFile.mockReset();
    useCsvChartStore.setState(useCsvChartStore.getInitialState(), true);
  });

  it('updates and removes only the group matching the ID', () => {
    const first = createChartGroup('相同名称');
    const second = createChartGroup('相同名称');
    useCsvChartStore.getState().setChartGroups([first, second]);

    useCsvChartStore.getState().updateChartGroup(second.id, { columns: ['C'] });
    expect(useCsvChartStore.getState().chartGroups).toEqual([
      first,
      { ...second, columns: ['C'] },
    ]);

    useCsvChartStore.getState().removeChartGroup(second.id);
    expect(useCsvChartStore.getState().chartGroups).toEqual([first]);
  });

  it('fills the smallest unused chart group name and creates a unique ID', () => {
    const first = createChartGroup('图表1');
    const third = createChartGroup('图表3');
    useCsvChartStore.getState().setChartGroups([first, third]);

    useCsvChartStore.getState().addChartGroup();
    const groups = useCsvChartStore.getState().chartGroups;
    expect(groups.map(group => group.name)).toEqual(['图表1', '图表3', '图表2']);
    expect(new Set(groups.map(group => group.id)).size).toBe(3);
  });

  it('applies only the latest concurrent load', async () => {
    let resolveOld!: (value: CsvParseResult) => void;
    let resolveNew!: (value: CsvParseResult) => void;
    readCsvFile.mockImplementation((filePath) => new Promise(resolve => {
      if (filePath === 'old.csv') resolveOld = resolve;
      else resolveNew = resolve;
    }));

    const oldLoad = useCsvChartStore.getState().loadCsvFile('old.csv');
    const newLoad = useCsvChartStore.getState().loadCsvFile('new.csv');
    resolveNew(result(['NEW']));
    await newLoad;
    resolveOld(result(['OLD']));
    await oldLoad;

    expect(useCsvChartStore.getState().filePath).toBe('new.csv');
    expect(useCsvChartStore.getState().csvData).toEqual(result(['NEW']));
    expect(useCsvChartStore.getState().isLoading).toBe(false);
  });

  it('invalidates a pending load when data is cleared', async () => {
    let resolveLoad!: (value: CsvParseResult) => void;
    readCsvFile.mockImplementation(() => new Promise(resolve => { resolveLoad = resolve; }));
    const pending = useCsvChartStore.getState().loadCsvFile('pending.csv');

    useCsvChartStore.getState().clearData();
    resolveLoad(result(['LATE']));
    await pending;

    expect(useCsvChartStore.getState().csvData).toBeNull();
    expect(useCsvChartStore.getState().filePath).toBeNull();
    expect(useCsvChartStore.getState().isLoading).toBe(false);
  });

  it('preserves the previous successful data when the latest load fails', async () => {
    useCsvChartStore.setState({ csvData: result(['OLD']), filePath: 'old.csv', chartGroups: [createChartGroup('保留')] });
    readCsvFile.mockRejectedValueOnce(new Error('bad file'));

    await useCsvChartStore.getState().loadCsvFile('new.csv');

    expect(useCsvChartStore.getState().csvData).toEqual(result(['OLD']));
    expect(useCsvChartStore.getState().filePath).toBe('old.csv');
    expect(useCsvChartStore.getState().chartGroups.map(group => group.name)).toEqual(['保留']);
    expect(useCsvChartStore.getState().error).toBeTruthy();
    expect(useCsvChartStore.getState().isLoading).toBe(false);
  });
});
