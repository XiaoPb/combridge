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
        realtime: false,
        csvLoader: true,
      },
    },
    system: {
      visible: true,
      tabs: {
        info: false,
        logs: false,
        settings: true,
      },
    },
  },
  sidebar: {
    home: true,
    serial: true,
    ble: true,
    dashboard: false,
    gh3036: true,
    protocol: false,
    waveform: true,
    system: true,
  },
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
        tabs: {
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
