import { useState, useEffect, useRef, useCallback } from 'react';

type DebouncedFunction<T extends unknown[]> = (...args: T) => void;

export const useDebounce = <T>(
  value: T,
  delay: number
): [T, () => void] => {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);
  const timerRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
    }

    timerRef.current = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [value, delay]);

  const flush = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
    }
    setDebouncedValue(value);
  }, [value]);

  return [debouncedValue, flush];
};

export const useDebouncedCallback = <T extends unknown[]>(
  callback: DebouncedFunction<T>,
  delay: number
): DebouncedFunction<T> => {
  const timerRef = useRef<NodeJS.Timeout | null>(null);
  const callbackRef = useRef(callback);

  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  useEffect(() => {
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, []);

  return useCallback((...args: T) => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
    }

    timerRef.current = setTimeout(() => {
      callbackRef.current(...args);
    }, delay);
  }, [delay]) as DebouncedFunction<T>;
};

export const useThrottle = <T>(
  value: T,
  interval: number
): T => {
  const [throttledValue, setThrottledValue] = useState<T>(value);
  const lastExecutedRef = useRef<number>(Date.now());

  useEffect(() => {
    const now = Date.now();
    const timeSinceLastExecution = now - lastExecutedRef.current;

    if (timeSinceLastExecution >= interval) {
      lastExecutedRef.current = now;
      setThrottledValue(value);
    } else {
      const timerId = setTimeout(() => {
        lastExecutedRef.current = Date.now();
        setThrottledValue(value);
      }, interval - timeSinceLastExecution);

      return () => clearTimeout(timerId);
    }
  }, [value, interval]);

  return throttledValue;
};

export const useThrottledCallback = <T extends unknown[]>(
  callback: DebouncedFunction<T>,
  interval: number
): DebouncedFunction<T> => {
  const lastExecutedRef = useRef<number>(0);
  const callbackRef = useRef(callback);

  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  return useCallback((...args: T) => {
    const now = Date.now();
    const timeSinceLastExecution = now - lastExecutedRef.current;

    if (timeSinceLastExecution >= interval) {
      lastExecutedRef.current = now;
      callbackRef.current(...args);
    }
  }, [interval]) as DebouncedFunction<T>;
};

export default useDebounce;
