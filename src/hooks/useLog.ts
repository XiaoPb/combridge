import { useLogStore, type LogLevel } from '../stores/logStore';

export const useLog = () => {
  const addLog = useLogStore((state) => state.addLog);

  const logInfo = (source: string, message: string) => {
    addLog('info', source, message);
  };

  const logWarn = (source: string, message: string) => {
    addLog('warn', source, message);
  };

  const logError = (source: string, message: string) => {
    addLog('error', source, message);
  };

  const logDebug = (source: string, message: string) => {
    addLog('debug', source, message);
  };

  return {
    logInfo,
    logWarn,
    logError,
    logDebug,
  };
};

export type { LogLevel };
