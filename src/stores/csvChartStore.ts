import { create } from 'zustand';
import { CsvParseConfig, CsvParseResult, readCsvFile } from '../utils/csvParser';
import type { ChartGroupConfig, YAxisConfig } from '../pages/Waveform/MultiLineChart';

interface CsvChartState {
  csvData: CsvParseResult | null;
  filePath: string | null;
  chartGroups: ChartGroupConfig[];
  yAxisConfigs: Record<string, YAxisConfig[]>;
  hiddenLines: string[];
  isLoading: boolean;
  error: string | null;
  parseConfig: CsvParseConfig;
  visiblePoints: number;
}

interface CsvChartActions {
  loadCsvFile: (filePath: string) => Promise<void>;
  setChartGroups: (groups: ChartGroupConfig[]) => void;
  addChartGroup: (group: ChartGroupConfig) => void;
  removeChartGroup: (name: string) => void;
  updateChartGroup: (name: string, group: Partial<ChartGroupConfig>) => void;
  setYAxisConfigs: (groupName: string, configs: YAxisConfig[]) => void;
  toggleLineVisibility: (columnName: string) => void;
  setParseConfig: (config: Partial<CsvParseConfig>) => void;
  setVisiblePoints: (points: number) => void;
  clearData: () => void;
  clearError: () => void;
}

export type CsvChartStore = CsvChartState & CsvChartActions;

const DEFAULT_PARSE_CONFIG: CsvParseConfig = {
  skipInfoRows: 0,
  noHeader: false,
  splitColumn: false,
  splitColumnIndex: 0,
};

const DEFAULT_CHART_GROUPS: ChartGroupConfig[] = [
  { name: '图表1', columns: [], height: 300 },
  { name: '图表2', columns: [], height: 300 },
];

export const useCsvChartStore = create<CsvChartStore>((set, get) => ({
  csvData: null,
  filePath: null,
  chartGroups: DEFAULT_CHART_GROUPS,
  yAxisConfigs: {},
  hiddenLines: [],
  isLoading: false,
  error: null,
  parseConfig: DEFAULT_PARSE_CONFIG,
  visiblePoints: 1000,

  loadCsvFile: async (filePath: string) => {
    set({ isLoading: true, error: null });
    try {
      const config = get().parseConfig;
      const csvData = await readCsvFile(filePath, config);
      set({
        csvData,
        filePath,
        chartGroups: DEFAULT_CHART_GROUPS,
        hiddenLines: [],
      });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    } finally {
      set({ isLoading: false });
    }
  },

  setChartGroups: (groups: ChartGroupConfig[]) => {
    set({ chartGroups: groups });
  },

  addChartGroup: (group: ChartGroupConfig) => {
    set({ chartGroups: [...get().chartGroups, group] });
  },

  removeChartGroup: (name: string) => {
    const newGroups = get().chartGroups.filter(g => g.name !== name);
    const newYAxisConfigs = { ...get().yAxisConfigs };
    delete newYAxisConfigs[name];
    set({ chartGroups: newGroups, yAxisConfigs: newYAxisConfigs });
  },

  updateChartGroup: (name: string, group: Partial<ChartGroupConfig>) => {
    set({
      chartGroups: get().chartGroups.map(g => 
        g.name === name ? { ...g, ...group } : g
      ),
    });
  },

  setYAxisConfigs: (groupName: string, configs: YAxisConfig[]) => {
    set({
      yAxisConfigs: { ...get().yAxisConfigs, [groupName]: configs },
    });
  },

  toggleLineVisibility: (columnName: string) => {
    const hiddenLines = get().hiddenLines;
    if (hiddenLines.includes(columnName)) {
      set({ hiddenLines: hiddenLines.filter(name => name !== columnName) });
    } else {
      set({ hiddenLines: [...hiddenLines, columnName] });
    }
  },

  setParseConfig: (config: Partial<CsvParseConfig>) => {
    set({ parseConfig: { ...get().parseConfig, ...config } });
  },

  setVisiblePoints: (points: number) => {
    set({ visiblePoints: points });
  },

  clearData: () => {
    set({
      csvData: null,
      filePath: null,
      chartGroups: DEFAULT_CHART_GROUPS,
      yAxisConfigs: {},
      hiddenLines: [],
      error: null,
    });
  },

  clearError: () => {
    set({ error: null });
  },
}));

export { DEFAULT_PARSE_CONFIG, DEFAULT_CHART_GROUPS };
