import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type {
  DashboardConfig,
  DataPoint,
  DataSourceType,
  ParserType,
  WidgetConfig,
  ParserScriptInfo,
  TabType,
  DashboardJsonConfig,
  RawDataPoint,
  SerialConfig,
  BleConnectionConfig,
} from '../types/dashboard';
import { DEFAULT_JSON_CONFIG, DEFAULT_SERIAL_CONFIG } from '../types/dashboard';

interface DashboardState {
  currentDashboard: DashboardConfig | null;
  savedDashboards: DashboardConfig[];
  dataSourceType: DataSourceType;
  connectedDevice: string | null;
  parserType: ParserType;
  parserScript: string | null;
  parserConfig: Record<string, unknown>;
  isRunning: boolean;
  dataBuffer: DataPoint[];
  maxBufferSize: number;
  isEditMode: boolean;
  selectedWidget: string | null;
  parserScripts: ParserScriptInfo[];
  lastError: string | null;

  activeTabs: TabType[];
  jsonConfig: DashboardJsonConfig;
  jsonFiles: string[];
  selectedJsonFile: string | null;
  rawDataBuffer: RawDataPoint[];
  parsedDataBuffer: DataPoint[];
  serialConfig: SerialConfig;
  serialPort: string;
  bleConfig: BleConnectionConfig | null;

  setCurrentDashboard: (dashboard: DashboardConfig | null) => void;
  saveDashboard: (dashboard: DashboardConfig) => void;
  deleteDashboard: (id: string) => void;
  renameDashboard: (id: string, name: string) => void;
  setDataSourceType: (type: DataSourceType) => void;
  setConnectedDevice: (deviceId: string | null) => void;
  setParserType: (type: ParserType) => void;
  setParserScript: (scriptName: string | null) => void;
  setParserConfig: (config: Record<string, unknown>) => void;
  setIsRunning: (running: boolean) => void;
  addDataPoint: (point: DataPoint) => void;
  clearDataBuffer: () => void;
  setIsEditMode: (edit: boolean) => void;
  setSelectedWidget: (widgetId: string | null) => void;
  addWidget: (widget: WidgetConfig) => void;
  updateWidget: (id: string, updates: Partial<WidgetConfig>) => void;
  removeWidget: (id: string) => void;
  setParserScripts: (scripts: ParserScriptInfo[]) => void;
  createNewDashboard: () => void;
  getSelectedWidget: () => WidgetConfig | null;
  setLastError: (error: string | null) => void;
  resetDashboard: () => void;

  setActiveTabs: (tabs: TabType[]) => void;
  toggleTab: (tab: TabType) => void;
  setJsonConfig: (config: DashboardJsonConfig) => void;
  setJsonFiles: (files: string[]) => void;
  setSelectedJsonFile: (file: string | null) => void;
  addRawDataPoint: (point: RawDataPoint) => void;
  clearRawDataBuffer: () => void;
  addParsedDataPoint: (point: DataPoint) => void;
  clearParsedDataBuffer: () => void;
  setSerialConfig: (config: SerialConfig) => void;
  setSerialPort: (port: string) => void;
  setBleConfig: (config: BleConnectionConfig | null) => void;
  exportToCsv: () => string;
}

const generateId = () => Math.random().toString(36).substring(2, 11);

export const useDashboardStore = create<DashboardState>()(
  persist(
    (set, get) => ({
      currentDashboard: null,
      savedDashboards: [],
      dataSourceType: 'serial',
      connectedDevice: null,
      parserType: 'json',
      parserScript: null,
      parserConfig: {},
      isRunning: false,
      dataBuffer: [],
      maxBufferSize: 1000,
      isEditMode: false,
      selectedWidget: null,
      parserScripts: [],
      lastError: null,

      activeTabs: ['dashboard'],
      jsonConfig: DEFAULT_JSON_CONFIG,
      jsonFiles: [],
      selectedJsonFile: null,
      rawDataBuffer: [],
      parsedDataBuffer: [],
      serialConfig: DEFAULT_SERIAL_CONFIG,
      serialPort: '',
      bleConfig: null,

      setCurrentDashboard: (dashboard) => set({ currentDashboard: dashboard }),

      saveDashboard: (dashboard) => {
        const { savedDashboards } = get();
        const existingIndex = savedDashboards.findIndex((d) => d.id === dashboard.id);
        if (existingIndex >= 0) {
          const updated = [...savedDashboards];
          updated[existingIndex] = dashboard;
          set({ savedDashboards: updated, currentDashboard: dashboard });
        } else {
          set({
            savedDashboards: [...savedDashboards, dashboard],
            currentDashboard: dashboard,
          });
        }
      },

      deleteDashboard: (id) => {
        const { savedDashboards, currentDashboard } = get();
        set({
          savedDashboards: savedDashboards.filter((d) => d.id !== id),
          currentDashboard: currentDashboard?.id === id ? null : currentDashboard,
        });
      },

      setDataSourceType: (type) => set({ dataSourceType: type }),

      setConnectedDevice: (deviceId) => set({ connectedDevice: deviceId }),

      setParserType: (type) => set({ parserType: type }),

      setParserScript: (scriptName) => set({ parserScript: scriptName }),

      setParserConfig: (config) => set({ parserConfig: config }),

      setIsRunning: (running) => set({ isRunning: running }),

      addDataPoint: (point) => {
        const { dataBuffer, maxBufferSize } = get();
        const newBuffer = [...dataBuffer, point];
        set({ dataBuffer: newBuffer.length > maxBufferSize ? newBuffer.slice(newBuffer.length - maxBufferSize) : newBuffer });
      },

      clearDataBuffer: () => set({ dataBuffer: [] }),

      setIsEditMode: (edit) => set({ isEditMode: edit }),

      setSelectedWidget: (widgetId) => set({ selectedWidget: widgetId }),

      addWidget: (widget) => {
        const { currentDashboard } = get();
        if (!currentDashboard) return;
        set({
          currentDashboard: {
            ...currentDashboard,
            widgets: [...currentDashboard.widgets, widget],
          },
        });
      },

      updateWidget: (id, updates) => {
        const { currentDashboard } = get();
        if (!currentDashboard) return;
        set({
          currentDashboard: {
            ...currentDashboard,
            widgets: currentDashboard.widgets.map((w) =>
              w.id === id ? { ...w, ...updates } : w
            ),
          },
        });
      },

      removeWidget: (id) => {
        const { currentDashboard, selectedWidget } = get();
        if (!currentDashboard) return;
        set({
          currentDashboard: {
            ...currentDashboard,
            widgets: currentDashboard.widgets.filter((w) => w.id !== id),
          },
          selectedWidget: selectedWidget === id ? null : selectedWidget,
        });
      },

      setParserScripts: (scripts) => set({ parserScripts: scripts }),

      createNewDashboard: () => {
        const { savedDashboards } = get();
        const newDashboard: DashboardConfig = {
          id: generateId(),
          name: `Dashboard ${savedDashboards.length + 1}`,
          dataSource: {
            type: 'serial',
          },
          parser: {
            type: 'json',
            config: {},
          },
          widgets: [],
          refreshRate: 100,
        };
        set({
          currentDashboard: newDashboard,
          savedDashboards: [...savedDashboards, newDashboard],
        });
      },

      renameDashboard: (id: string, name: string) => {
        const { savedDashboards, currentDashboard } = get();
        const updatedDashboards = savedDashboards.map((d) =>
          d.id === id ? { ...d, name } : d
        );
        set({
          savedDashboards: updatedDashboards,
          currentDashboard:
            currentDashboard?.id === id
              ? { ...currentDashboard, name }
              : currentDashboard,
        });
      },

      getSelectedWidget: () => {
        const { currentDashboard, selectedWidget } = get();
        if (!currentDashboard || !selectedWidget) return null;
        return currentDashboard.widgets.find((w) => w.id === selectedWidget) || null;
      },

      setLastError: (error) => set({ lastError: error }),

      resetDashboard: () => {
        set({
          currentDashboard: null,
          dataSourceType: 'serial',
          connectedDevice: null,
          parserType: 'json',
          parserScript: null,
          parserConfig: {},
          isRunning: false,
          dataBuffer: [],
          isEditMode: false,
          selectedWidget: null,
          lastError: null,
          activeTabs: ['dashboard'],
          rawDataBuffer: [],
          parsedDataBuffer: [],
          serialConfig: DEFAULT_SERIAL_CONFIG,
          serialPort: '',
        });
      },

      setActiveTabs: (tabs) => set({ activeTabs: tabs }),

      toggleTab: (tab) => {
        const { activeTabs } = get();
        if (tab === 'jsonEditor') {
          if (activeTabs.includes('jsonEditor')) {
            set({ activeTabs: activeTabs.filter((t) => t !== 'jsonEditor') });
          } else {
            set({ activeTabs: ['jsonEditor'] });
          }
        } else {
          let newTabs = activeTabs.filter((t) => t !== 'jsonEditor');
          if (newTabs.includes(tab)) {
            newTabs = newTabs.filter((t) => t !== tab);
            if (newTabs.length === 0) {
              newTabs = ['dashboard'];
            }
          } else {
            newTabs = [...newTabs, tab];
          }
          set({ activeTabs: newTabs });
        }
      },

      setJsonConfig: (config) => set({ jsonConfig: config }),

      setJsonFiles: (files) => set({ jsonFiles: files }),

      setSelectedJsonFile: (file) => set({ selectedJsonFile: file }),

      addRawDataPoint: (point) => {
        const { rawDataBuffer, maxBufferSize } = get();
        const newBuffer = [...rawDataBuffer, point];
        set({ rawDataBuffer: newBuffer.length > maxBufferSize ? newBuffer.slice(newBuffer.length - maxBufferSize) : newBuffer });
      },

      clearRawDataBuffer: () => set({ rawDataBuffer: [] }),

      addParsedDataPoint: (point) => {
        const { parsedDataBuffer, maxBufferSize } = get();
        const newBuffer = [...parsedDataBuffer, point];
        set({ parsedDataBuffer: newBuffer.length > maxBufferSize ? newBuffer.slice(newBuffer.length - maxBufferSize) : newBuffer });
      },

      clearParsedDataBuffer: () => set({ parsedDataBuffer: [] }),

      setSerialConfig: (config) => set({ serialConfig: config }),

      setSerialPort: (port) => set({ serialPort: port }),

      setBleConfig: (config) => set({ bleConfig: config }),

      exportToCsv: () => {
        const { parsedDataBuffer } = get();
        if (parsedDataBuffer.length === 0) {
          return '';
        }

        const allKeys = new Set<string>();
        parsedDataBuffer.forEach((point) => {
          Object.keys(point.values).forEach((key) => allKeys.add(key));
        });
        const keys = ['timestamp', ...Array.from(allKeys)];

        const rows = parsedDataBuffer.map((point) => {
          const row = [point.timestamp.toString()];
          keys.slice(1).forEach((key) => {
            row.push(point.values[key]?.toString() ?? '');
          });
          return row.join(',');
        });

        return [keys.join(','), ...rows].join('\n');
      },
    }),
    {
      name: 'dashboard-storage',
      partialize: (state) => ({
        savedDashboards: state.savedDashboards,
        maxBufferSize: state.maxBufferSize,
        serialConfig: state.serialConfig,
        serialPort: state.serialPort,
        activeTabs: state.activeTabs,
        connectedDevice: state.connectedDevice,
      }),
    }
  )
);
