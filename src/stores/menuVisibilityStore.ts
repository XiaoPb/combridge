import { create } from 'zustand';
import { preferencesApi, type MenuVisibilityPreferences } from '../api/tauri';

export type HomeMenuKey = keyof MenuVisibilityPreferences['home'];
export type SidebarMenuKey = keyof MenuVisibilityPreferences['sidebar'];

export const DEFAULT_MENU_VISIBILITY: MenuVisibilityPreferences = {
  home: {
    connection: {
      visible: true,
      tabs: {
        serial: true,
        ble: true,
      },
    },
    dashboard: {
      visible: false,
      tabs: {
        dashboard: false,
        console: false,
        settings: false,
        jsonEditor: false,
      },
    },
    gh3036: {
      visible: true,
      tabs: {
        config: true,
        monitor: true,
        version: true,
        factory: true,
        threshold: true,
      },
    },
    gh3220: {
      visible: true,
      tabs: {
        config: true,
        monitor: true,
        version: true,
        debug: true,
      },
    },
    ota: {
      visible: true,
      tabs: {
        upgrade: true,
        history: true,
      },
    },
    protocol: {
      visible: false,
      tabs: {
        editor: false,
        bind: false,
      },
    },
    waveform: {
      visible: true,
      tabs: {
        realtime: true,
        csvLoader: true,
      },
    },
    system: {
      visible: true,
      tabs: {
        info: false,
        logs: false,
        settings: true,
        integration: true,
      },
    },
  },
  sidebar: {
    home: true,
    serial: true,
    ble: true,
    dashboard: false,
    gh3036: true,
    gh3220: true,
    ota: true,
    protocol: false,
    waveform: true,
    system: true,
  },
};

/** V1 的调试类页签 key，旧持久化配置加载时统一收敛到 debug */
const LEGACY_GH3220_TAB_KEYS = ['capture', 'control', 'events'];

/**
 * 旧配置没有 debug 键：把 capture/control/events 的可见性收敛到 debug，
 * 同时丢弃旧 key，避免残留配置导致调试页消失或菜单里出现无效项。
 */
const mergeGh3220Tabs = (savedTabs?: Record<string, boolean>): Record<string, boolean> => {
  const defaults = DEFAULT_MENU_VISIBILITY.home.gh3220.tabs;
  if (!savedTabs) return { ...defaults };
  const hasLegacy = LEGACY_GH3220_TAB_KEYS.some((key) => key in savedTabs);
  const legacyVisible = LEGACY_GH3220_TAB_KEYS.some((key) => savedTabs[key] === true);
  return {
    config: savedTabs.config ?? defaults.config,
    monitor: savedTabs.monitor ?? defaults.monitor,
    version: savedTabs.version ?? defaults.version,
    debug: savedTabs.debug ?? (hasLegacy ? legacyVisible : defaults.debug),
  };
};

interface MenuVisibilityState {
  menuVisibility: MenuVisibilityPreferences;
  isLoaded: boolean;
  isSaving: boolean;
  loadMenuVisibility: () => Promise<void>;
  saveMenuVisibility: (prefs: MenuVisibilityPreferences) => Promise<void>;
  resetMenuVisibility: () => Promise<void>;
}

const mergeMenuVisibility = (
  prefs?: MenuVisibilityPreferences
): MenuVisibilityPreferences => {
  if (!prefs) {
    return structuredClone(DEFAULT_MENU_VISIBILITY);
  }

  const mergedHome = Object.entries(DEFAULT_MENU_VISIBILITY.home).reduce(
    (acc, [key, defaultGroup]) => {
      const groupKey = key as HomeMenuKey;
      const savedGroup = prefs.home?.[groupKey];
      acc[groupKey] = {
        visible: savedGroup?.visible ?? defaultGroup.visible,
        tabs:
          groupKey === 'gh3220'
            ? mergeGh3220Tabs(savedGroup?.tabs)
            : {
                ...defaultGroup.tabs,
                ...(savedGroup?.tabs ?? {}),
              },
      };
      return acc;
    },
    {} as MenuVisibilityPreferences['home']
  );

  return {
    home: mergedHome,
    sidebar: {
      ...DEFAULT_MENU_VISIBILITY.sidebar,
      ...(prefs.sidebar ?? {}),
    },
  };
};

export const cloneDefaultMenuVisibility = (): MenuVisibilityPreferences =>
  structuredClone(DEFAULT_MENU_VISIBILITY);

export const useMenuVisibilityStore = create<MenuVisibilityState>((set) => ({
  menuVisibility: cloneDefaultMenuVisibility(),
  isLoaded: false,
  isSaving: false,

  loadMenuVisibility: async () => {
    try {
      const prefs = await preferencesApi.get();
      set({
        menuVisibility: mergeMenuVisibility(prefs.menuVisibility),
        isLoaded: true,
      });
    } catch (err) {
      console.error('加载菜单显示偏好设置失败:', err);
      set({
        menuVisibility: cloneDefaultMenuVisibility(),
        isLoaded: true,
      });
    }
  },

  saveMenuVisibility: async (prefs: MenuVisibilityPreferences) => {
    const mergedPrefs = mergeMenuVisibility(prefs);
    set({ isSaving: true, menuVisibility: mergedPrefs });
    try {
      await preferencesApi.updateMenuVisibility(mergedPrefs);
    } catch (err) {
      console.error('保存菜单显示偏好设置失败:', err);
    } finally {
      set({ isSaving: false });
    }
  },

  resetMenuVisibility: async () => {
    const defaults = cloneDefaultMenuVisibility();
    set({ isSaving: true, menuVisibility: defaults });
    try {
      await preferencesApi.updateMenuVisibility(defaults);
    } catch (err) {
      console.error('恢复菜单显示默认设置失败:', err);
    } finally {
      set({ isSaving: false });
    }
  },
}));
