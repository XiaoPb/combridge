import { useEffect, useCallback } from 'react';
import { useConfigStore, type AppConfig } from '../stores/configStore';

type ThemeMode = AppConfig['theme'];

function getSystemTheme(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

function resolveActualTheme(theme: ThemeMode): 'light' | 'dark' {
  if (theme === 'system') {
    return getSystemTheme() ? 'dark' : 'light';
  }
  return theme;
}

function applyThemeToDom(isDark: boolean): void {
  const root = document.documentElement;
  if (isDark) {
    root.setAttribute('data-theme', 'dark');
  } else {
    root.removeAttribute('data-theme');
  }
}

export function useTheme() {
  const settings = useConfigStore((s) => s.settings);
  const updateConfig = useConfigStore((s) => s.updateConfig);
  const theme = settings.theme;
  const isDark = resolveActualTheme(theme) === 'dark';

  useEffect(() => {
    applyThemeToDom(isDark);
  }, [isDark]);

  useEffect(() => {
    if (theme !== 'system') return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = (e: MediaQueryListEvent) => {
      applyThemeToDom(e.matches);
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [theme]);

  const setThemeMode = useCallback((newTheme: ThemeMode) => {
    updateConfig({ theme: newTheme });
  }, [updateConfig]);

  const toggleTheme = useCallback(() => {
    const newTheme: ThemeMode = isDark ? 'light' : 'dark';
    setThemeMode(newTheme);
  }, [isDark, setThemeMode]);

  return {
    theme,
    isDark,
    isSystem: theme === 'system',
    setTheme: setThemeMode,
    toggleTheme,
  };
}

export default useTheme;
