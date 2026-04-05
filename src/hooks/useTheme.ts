import { useState, useEffect, useCallback } from 'react';
import { configService, AppConfig } from '../services/configService';

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
  const [theme, setTheme] = useState<ThemeMode>(() => configService.getConfig().theme);
  const [isDark, setIsDark] = useState<boolean>(() => {
    const config = configService.getConfig();
    return resolveActualTheme(config.theme) === 'dark';
  });

  useEffect(() => {
    const unsubscribe = configService.subscribe((config) => {
      setTheme(config.theme);
      const actualTheme = resolveActualTheme(config.theme);
      setIsDark(actualTheme === 'dark');
    });

    return unsubscribe;
  }, []);

  useEffect(() => {
    applyThemeToDom(isDark);
  }, [isDark]);

  useEffect(() => {
    if (theme !== 'system') return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = (e: MediaQueryListEvent) => {
      setIsDark(e.matches);
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [theme]);

  const setThemeMode = useCallback((newTheme: ThemeMode) => {
    configService.updateConfig({ theme: newTheme });
  }, []);

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
