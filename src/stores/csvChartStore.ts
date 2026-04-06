import { create } from 'zustand';
import { CsvParseConfig, CsvParseResult, readCsvFile } from '../utils/csvParser';

interface CsvChartState {
  csvData: CsvParseResult | null;
  filePath: string | null;
  chart1Columns: string[];
  chart2Columns: string[];
  xAxisRange: [number, number];
  hiddenLines: string[];
  isLoading: boolean;
  error: string | null;
  parseConfig: CsvParseConfig;
}

interface CsvChartActions {
  loadCsvFile: (filePath: string) => Promise<void>;
  setChart1Columns: (columns: string[]) => void;
  setChart2Columns: (columns: string[]) => void;
  setXAxisRange: (range: [number, number]) => void;
  toggleLineVisibility: (columnName: string) => void;
  setParseConfig: (config: Partial<CsvParseConfig>) => void;
  clearData: () => void;
  clearError: () => void;
}

export type CsvChartStore = CsvChartState & CsvChartActions;

const DEFAULT_PARSE_CONFIG: CsvParseConfig = {
  skipFirstRow: false,
  useSecondRowAsHeader: false,
  splitColumn: false,
  splitColumnIndex: 0,
};

export const useCsvChartStore = create<CsvChartStore>((set, get) => ({
  csvData: null,
  filePath: null,
  chart1Columns: [],
  chart2Columns: [],
  xAxisRange: [0, 100],
  hiddenLines: [],
  isLoading: false,
  error: null,
  parseConfig: DEFAULT_PARSE_CONFIG,

  loadCsvFile: async (filePath: string) => {
    set({ isLoading: true, error: null });
    try {
      const config = get().parseConfig;
      const csvData = await readCsvFile(filePath, config);
      const xAxisRange: [number, number] = [0, csvData.rows.length > 0 ? csvData.rows.length - 1 : 0];
      set({
        csvData,
        filePath,
        xAxisRange,
        chart1Columns: [],
        chart2Columns: [],
        hiddenLines: [],
      });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    } finally {
      set({ isLoading: false });
    }
  },

  setChart1Columns: (columns: string[]) => {
    set({ chart1Columns: columns });
  },

  setChart2Columns: (columns: string[]) => {
    set({ chart2Columns: columns });
  },

  setXAxisRange: (range: [number, number]) => {
    set({ xAxisRange: range });
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

  clearData: () => {
    set({
      csvData: null,
      filePath: null,
      chart1Columns: [],
      chart2Columns: [],
      xAxisRange: [0, 100],
      hiddenLines: [],
      error: null,
    });
  },

  clearError: () => {
    set({ error: null });
  },
}));

export { DEFAULT_PARSE_CONFIG };
