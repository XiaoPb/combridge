import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MenuVisibilityPreferences } from '../api/tauri';

vi.mock('../api/tauri', () => ({
  preferencesApi: {
    get: vi.fn(),
    updateMenuVisibility: vi.fn(async () => undefined),
  },
}));

import { preferencesApi } from '../api/tauri';
import { DEFAULT_MENU_VISIBILITY, useMenuVisibilityStore } from './menuVisibilityStore';

/** 构造 V1 时代带 capture/control/events 旧页签 key 的持久化配置 */
function persistedWith(tabs: Record<string, boolean>): MenuVisibilityPreferences {
  const base = structuredClone(DEFAULT_MENU_VISIBILITY);
  return {
    ...base,
    home: {
      ...base.home,
      gh3220: { visible: true, tabs },
    },
  } as unknown as MenuVisibilityPreferences;
}

describe('menuVisibilityStore · GH3220 旧页签兼容', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const loadWith = async (tabs: Record<string, boolean>) => {
    // preferencesApi.get() 返回完整 Preferences，菜单配置挂在 menuVisibility 字段下
    vi.mocked(preferencesApi.get).mockResolvedValue({
      menuVisibility: persistedWith(tabs),
    } as never);
    await useMenuVisibilityStore.getState().loadMenuVisibility();
    return useMenuVisibilityStore.getState().menuVisibility.home.gh3220.tabs;
  };

  it('旧 key 中任一为 true 时，收敛出可见的 debug 并丢弃旧 key', async () => {
    const tabs = await loadWith({
      config: true,
      monitor: true,
      version: true,
      capture: true,
      control: false,
      events: false,
    });
    expect(Object.keys(tabs).sort()).toEqual(['config', 'debug', 'monitor', 'version']);
    expect(tabs.debug).toBe(true);
  });

  it('旧 key 全为 false 时，debug 保持隐藏', async () => {
    const tabs = await loadWith({
      config: true,
      monitor: true,
      version: true,
      capture: false,
      control: false,
      events: false,
    });
    expect(tabs.debug).toBe(false);
    expect(Object.keys(tabs)).not.toContain('capture');
  });

  it('已有 debug 配置时优先沿用，不被旧 key 覆盖', async () => {
    const tabs = await loadWith({
      config: true,
      monitor: true,
      version: true,
      debug: false,
      capture: true,
      events: true,
    });
    expect(tabs.debug).toBe(false);
    expect(Object.keys(tabs).sort()).toEqual(['config', 'debug', 'monitor', 'version']);
  });

  it('无旧 key 且缺 debug 时回落到默认值（可见）', async () => {
    const tabs = await loadWith({ config: true, monitor: true, version: true });
    expect(tabs.debug).toBe(true);
  });
});
