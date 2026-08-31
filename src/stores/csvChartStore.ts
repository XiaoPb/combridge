import { create } from 'zustand';
import { CsvParseConfig, CsvParseResult, readCsvFile } from '../utils/csvParser';
import type { IdentifiedChartGroupConfig } from '../pages/Waveform/chartGroup';
import { createChartGroup, getNextChartGroupName } from '../pages/Waveform/chartGroup';
import i18n from '../i18n';
import { formatErrorMessage } from '../utils/errorMessage';

interface DataZoomState { start: number; end: number; }
interface CsvChartState {
  csvData: CsvParseResult | null;
  filePath: string | null;
  chartGroups: IdentifiedChartGroupConfig[];
  isLoading: boolean;
  error: string | null;
  parseConfig: CsvParseConfig;
  visiblePoints: number;
  sampleRate: number;
  dataZoomState: DataZoomState;
}
interface CsvChartActions {
  loadCsvFile: (filePath: string) => Promise<void>;
  setChartGroups: (groups: IdentifiedChartGroupConfig[]) => void;
  addChartGroup: () => void;
  removeChartGroup: (id: string) => void;
  updateChartGroup: (id: string, patch: Partial<IdentifiedChartGroupConfig>) => void;
  setParseConfig: (config: Partial<CsvParseConfig>) => void;
  setVisiblePoints: (points: number) => void;
  setSampleRate: (rate: number) => void;
  setDataZoomState: (state: DataZoomState) => void;
  clearData: () => void;
  clearError: () => void;
}
export type CsvChartStore = CsvChartState & CsvChartActions;

const DEFAULT_PARSE_CONFIG: CsvParseConfig = { skipInfoRows: 1, noHeader: false };
const DEFAULT_DATA_ZOOM_STATE: DataZoomState = { start: 0, end: 100 };

export function createDefaultChartGroups(): IdentifiedChartGroupConfig[] {
  return [createChartGroup('图表1'), createChartGroup('图表2')];
}

function autoAssignChartGroups(columns: string[]): IdentifiedChartGroupConfig[] {
  const accColumns: string[] = [];
  const chColumns: string[] = [];
  columns.forEach((col) => {
    const baseColumn = col.replace(/ \(\d+\)$/, '');
    if (/^ACC_?[XYZ]$/i.test(baseColumn)) accColumns.push(col);
    else if (/^(?:CH|Ipd)[0-3]$/i.test(baseColumn)) chColumns.push(col);
  });
  return [createChartGroup('图表1', chColumns), createChartGroup('图表2', accColumns)];
}

let loadGeneration = 0;

export const useCsvChartStore = create<CsvChartStore>((set, get) => ({
  csvData: null,
  filePath: null,
  chartGroups: createDefaultChartGroups(),
  isLoading: false,
  error: null,
  parseConfig: { ...DEFAULT_PARSE_CONFIG },
  visiblePoints: 1000,
  sampleRate: 25,
  dataZoomState: { ...DEFAULT_DATA_ZOOM_STATE },
  loadCsvFile: async (filePath) => {
    const generation = ++loadGeneration;
    set({ isLoading: true, error: null });
    try {
      const csvData = await readCsvFile(filePath, get().parseConfig);
      if (generation !== loadGeneration) return;
      const dataLength = csvData.rows.length;
      const tenSecondsPoints = get().sampleRate * 10;
      const dataZoomState = dataLength > tenSecondsPoints
        ? { start: 0, end: (tenSecondsPoints / dataLength) * 100 }
        : { ...DEFAULT_DATA_ZOOM_STATE };
      set({ csvData, filePath, chartGroups: autoAssignChartGroups(csvData.columns), dataZoomState, isLoading: false });
    } catch (err) {
      if (generation !== loadGeneration) return;
      set({ error: formatErrorMessage(err, i18n.t('waveform:errors.loadCsv')), isLoading: false });
    }
  },
  setChartGroups: (groups) => set({ chartGroups: groups }),
  addChartGroup: () => {
    const groups = get().chartGroups;
    set({ chartGroups: [...groups, createChartGroup(getNextChartGroupName(groups))] });
  },
  removeChartGroup: (id) => set({ chartGroups: get().chartGroups.filter(group => group.id !== id) }),
  updateChartGroup: (id, patch) => {
    const { id: _ignoredId, ...safePatch } = patch;
    set({ chartGroups: get().chartGroups.map(group => group.id === id ? { ...group, ...safePatch } : group) });
  },
  setParseConfig: (config) => set({ parseConfig: { ...get().parseConfig, ...config } }),
  setVisiblePoints: (points) => set({ visiblePoints: points }),
  setSampleRate: (rate) => set({ sampleRate: rate }),
  setDataZoomState: (state) => set({ dataZoomState: state }),
  clearData: () => {
    ++loadGeneration;
    set({ csvData: null, filePath: null, chartGroups: createDefaultChartGroups(), isLoading: false, error: null, dataZoomState: { ...DEFAULT_DATA_ZOOM_STATE } });
  },
  clearError: () => set({ error: null }),
}));

export { DEFAULT_PARSE_CONFIG, DEFAULT_DATA_ZOOM_STATE };
