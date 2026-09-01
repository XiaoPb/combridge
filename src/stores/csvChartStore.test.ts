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

  it('auto-assigns generated duplicate headers without matching unrelated columns', async () => {
    readCsvFile.mockResolvedValue(result([
      'CH0',
      'CH0 (2)',
      'ACC_X',
      'ACC_X (2)',
      'CH0 (other)',
      'ACC_X (other)',
      'OTHER (2)',
    ]));

    await useCsvChartStore.getState().loadCsvFile('headers.csv');

    const [channelGroup, accelerometerGroup] =
      useCsvChartStore.getState().chartGroups;
    expect(channelGroup.columns).toEqual(['CH0', 'CH0 (2)']);
    expect(accelerometerGroup.columns).toEqual(['ACC_X', 'ACC_X (2)']);
  });

  it('preserves zoom when loading another CSV without an explicit reload', async () => {
    useCsvChartStore.setState({
      csvData: result(['OLD'], [[1], [2]]),
      filePath: 'old.csv',
      dataZoomState: { start: 24, end: 76 },
    });
    readCsvFile.mockResolvedValue(result(['NEW'], [[3], [4], [5]]));

    await useCsvChartStore.getState().loadCsvFile('new.csv');

    expect(useCsvChartStore.getState().dataZoomState).toEqual({ start: 24, end: 76 });
  });

  it('resets zoom when explicitly reloading the CSV', async () => {
    useCsvChartStore.setState({
      csvData: result(['OLD'], Array.from({ length: 200 }, (_, index) => [index])),
      filePath: 'old.csv',
      sampleRate: 10,
      dataZoomState: { start: 24, end: 76 },
    });
    readCsvFile.mockResolvedValue(result(['NEW'], Array.from({ length: 200 }, (_, index) => [index])));

    await useCsvChartStore.getState().loadCsvFile('old.csv', { resetZoom: true });

    expect(useCsvChartStore.getState().dataZoomState).toEqual({ start: 0, end: 50 });
  });
});
