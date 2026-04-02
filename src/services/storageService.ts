type StorageKey = string;

interface StorageItem<T> {
  value: T;
  timestamp: number;
  expires?: number;
}

class StorageService {
  private prefix: string;

  constructor(prefix = 'combridge') {
    this.prefix = prefix;
  }

  private getKey(key: StorageKey): string {
    return `${this.prefix}-${key}`;
  }

  set<T>(key: StorageKey, value: T, expires?: number): void {
    const item: StorageItem<T> = {
      value,
      timestamp: Date.now(),
      expires,
    };

    try {
      localStorage.setItem(this.getKey(key), JSON.stringify(item));
    } catch (err) {
      console.error('Failed to save to storage:', err);
    }
  }

  get<T>(key: StorageKey, defaultValue?: T): T | undefined {
    try {
      const stored = localStorage.getItem(this.getKey(key));
      if (!stored) return defaultValue;

      const item: StorageItem<T> = JSON.parse(stored);

      if (item.expires && Date.now() > item.timestamp + item.expires) {
        this.remove(key);
        return defaultValue;
      }

      return item.value;
    } catch (err) {
      console.error('Failed to read from storage:', err);
      return defaultValue;
    }
  }

  remove(key: StorageKey): void {
    try {
      localStorage.removeItem(this.getKey(key));
    } catch (err) {
      console.error('Failed to remove from storage:', err);
    }
  }

  clear(): void {
    try {
      const keysToRemove: string[] = [];
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key && key.startsWith(this.prefix)) {
          keysToRemove.push(key);
        }
      }
      keysToRemove.forEach((key) => localStorage.removeItem(key));
    } catch (err) {
      console.error('Failed to clear storage:', err);
    }
  }

  has(key: StorageKey): boolean {
    return localStorage.getItem(this.getKey(key)) !== null;
  }

  keys(): string[] {
    const keys: string[] = [];
    const prefix = `${this.prefix}-`;

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key && key.startsWith(prefix)) {
        keys.push(key.substring(prefix.length));
      }
    }

    return keys;
  }

  setSession<T>(key: StorageKey, value: T): void {
    const item: StorageItem<T> = {
      value,
      timestamp: Date.now(),
    };

    try {
      sessionStorage.setItem(this.getKey(key), JSON.stringify(item));
    } catch (err) {
      console.error('Failed to save to session storage:', err);
    }
  }

  getSession<T>(key: StorageKey, defaultValue?: T): T | undefined {
    try {
      const stored = sessionStorage.getItem(this.getKey(key));
      if (!stored) return defaultValue;

      const item: StorageItem<T> = JSON.parse(stored);
      return item.value;
    } catch (err) {
      console.error('Failed to read from session storage:', err);
      return defaultValue;
    }
  }

  removeSession(key: StorageKey): void {
    try {
      sessionStorage.removeItem(this.getKey(key));
    } catch (err) {
      console.error('Failed to remove from session storage:', err);
    }
  }

  pushToList<T>(key: StorageKey, item: T, maxLength = 100): T[] {
    const list = this.get<T[]>(key, []) || [];
    list.push(item);

    if (list.length > maxLength) {
      list.shift();
    }

    this.set(key, list);
    return list;
  }

  getFromList<T>(key: StorageKey): T[] {
    return this.get<T[]>(key, []) || [];
  }

  removeFromList<T>(key: StorageKey, predicate: (item: T) => boolean): T[] {
    const list = this.get<T[]>(key, []) || [];
    const filtered = list.filter((item) => !predicate(item));
    this.set(key, filtered);
    return filtered;
  }

  updateInList<T>(key: StorageKey, predicate: (item: T) => boolean, updates: Partial<T>): T[] {
    const list = this.get<T[]>(key, []) || [];
    const updated = list.map((item) =>
      predicate(item) ? { ...item, ...updates } : item
    );
    this.set(key, updated);
    return updated;
  }

  getStorageInfo(): { used: number; available: number; keys: number } {
    let used = 0;
    let keys = 0;

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key) {
        const value = localStorage.getItem(key);
        if (value) {
          used += key.length + value.length;
        }
        keys++;
      }
    }

    return {
      used: used * 2,
      available: 5 * 1024 * 1024 - used * 2,
      keys,
    };
  }
}

export const storageService = new StorageService();
export default storageService;
