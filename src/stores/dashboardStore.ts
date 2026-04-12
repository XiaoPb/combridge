import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type {
  DashboardConfig,
  DataPoint,
  DataSourceType,
  ParserType,
  WidgetConfig,
  ParserScriptInfo,
} from '../types/dashboard';

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

  setCurrentDashboard: (dashboard: DashboardConfig | null) => void;
  saveDashboard: (dashboard: DashboardConfig) => void;
  deleteDashboard: (id: string) => void;
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
        if (newBuffer.length > maxBufferSize) {
          newBuffer.shift();
        }
        set({ dataBuffer: newBuffer });
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
        const newDashboard: DashboardConfig = {
          id: generateId(),
          name: 'New Dashboard',
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
        set({ currentDashboard: newDashboard });
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
        });
      },
    }),
    {
      name: 'dashboard-storage',
      partialize: (state) => ({
        savedDashboards: state.savedDashboards,
        maxBufferSize: state.maxBufferSize,
      }),
    }
  )
);
