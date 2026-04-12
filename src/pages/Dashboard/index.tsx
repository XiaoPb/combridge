import React, { useEffect, useRef, useCallback } from 'react';
import { Layout } from 'antd';
import { useDashboardStore } from '../../stores/dashboardStore';
import { dashboardApi } from '../../api/dashboard';
import { onSerialData, onBleData } from '../../api/events';
import { useLogStore } from '../../stores/logStore';
import DashboardToolbar from './DashboardToolbar';
import DashboardCanvas from './DashboardCanvas';
import DashboardPanel from './DashboardPanel';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { SerialDataEvent, BleDataEvent } from '../../api/events';

const { Content, Sider } = Layout;

const DashboardPage: React.FC = () => {
  const {
    currentDashboard,
    createNewDashboard,
    setParserScripts,
    isRunning,
    dataSourceType,
    parserScript,
    parserType,
    addDataPoint,
    setLastError,
  } = useDashboardStore();

  const listenersRef = useRef<{
    serialData?: UnlistenFn;
    bleData?: UnlistenFn;
  }>({});

  const parseData = useCallback(async (rawData: number[]): Promise<Record<string, number> | null> => {
    try {
      const dataString = rawData.map((b) => String.fromCharCode(b)).join('');

      if (parserType === 'json') {
        try {
          const parsed = JSON.parse(dataString);
          const values: Record<string, number> = {};
          for (const [key, value] of Object.entries(parsed)) {
            if (typeof value === 'number') {
              values[key] = value;
            }
          }
          return Object.keys(values).length > 0 ? values : null;
        } catch {
          console.error('[Dashboard] JSON解析失败');
          return null;
        }
      }

      if (parserType === 'lua' && parserScript) {
        try {
          const result = await dashboardApi.executeParserScript(parserScript, dataString);
          return result;
        } catch (error) {
          console.error('[Dashboard] Lua脚本解析失败:', error);
          return null;
        }
      }

      if (parserType === 'delimiter') {
        const values: Record<string, number> = {};
        const parts = dataString.split(/[,\s\t]+/).filter((s) => s.length > 0);
        parts.forEach((part, index) => {
          const num = parseFloat(part);
          if (!isNaN(num)) {
            values[`field_${index}`] = num;
          }
        });
        return Object.keys(values).length > 0 ? values : null;
      }

      return null;
    } catch (error) {
      console.error('[Dashboard] 数据解析异常:', error);
      return null;
    }
  }, [parserType, parserScript]);

  const handleSerialData = useCallback(async (event: SerialDataEvent) => {
    if (!isRunning || dataSourceType !== 'serial') {
      return;
    }

    try {
      const values = await parseData(event.data);
      if (values) {
        addDataPoint({
          timestamp: event.timestamp ?? Date.now(),
          values,
        });
        useLogStore.getState().addLog('debug', 'Dashboard', `串口数据解析成功: ${JSON.stringify(values)}`);
      }
    } catch (error) {
      const errorMsg = `串口数据解析失败: ${error}`;
      console.error(`[Dashboard] ${errorMsg}`);
      setLastError(errorMsg);
      useLogStore.getState().addLog('error', 'Dashboard', errorMsg);
    }
  }, [isRunning, dataSourceType, parseData, addDataPoint, setLastError]);

  const handleBleData = useCallback(async (event: BleDataEvent) => {
    if (!isRunning || dataSourceType !== 'ble') {
      return;
    }

    try {
      const values = await parseData(event.data);
      if (values) {
        addDataPoint({
          timestamp: event.timestamp ?? Date.now(),
          values,
        });
        useLogStore.getState().addLog('debug', 'Dashboard', `蓝牙数据解析成功: ${JSON.stringify(values)}`);
      }
    } catch (error) {
      const errorMsg = `蓝牙数据解析失败: ${error}`;
      console.error(`[Dashboard] ${errorMsg}`);
      setLastError(errorMsg);
      useLogStore.getState().addLog('error', 'Dashboard', errorMsg);
    }
  }, [isRunning, dataSourceType, parseData, addDataPoint, setLastError]);

  useEffect(() => {
    const setupDataListeners = async () => {
      if (listenersRef.current.serialData) {
        listenersRef.current.serialData();
        listenersRef.current.serialData = undefined;
      }
      if (listenersRef.current.bleData) {
        listenersRef.current.bleData();
        listenersRef.current.bleData = undefined;
      }

      if (!isRunning) {
        useLogStore.getState().addLog('info', 'Dashboard', '数据流已停止');
        return;
      }

      if (dataSourceType === 'serial') {
        try {
          listenersRef.current.serialData = await onSerialData(handleSerialData);
          useLogStore.getState().addLog('info', 'Dashboard', '串口数据监听已启动');
        } catch (error) {
          const errorMsg = `启动串口监听失败: ${error}`;
          console.error(`[Dashboard] ${errorMsg}`);
          setLastError(errorMsg);
          useLogStore.getState().addLog('error', 'Dashboard', errorMsg);
        }
      } else if (dataSourceType === 'ble') {
        try {
          listenersRef.current.bleData = await onBleData(handleBleData);
          useLogStore.getState().addLog('info', 'Dashboard', '蓝牙数据监听已启动');
        } catch (error) {
          const errorMsg = `启动蓝牙监听失败: ${error}`;
          console.error(`[Dashboard] ${errorMsg}`);
          setLastError(errorMsg);
          useLogStore.getState().addLog('error', 'Dashboard', errorMsg);
        }
      }
    };

    setupDataListeners();

    return () => {
      if (listenersRef.current.serialData) {
        listenersRef.current.serialData();
        listenersRef.current.serialData = undefined;
      }
      if (listenersRef.current.bleData) {
        listenersRef.current.bleData();
        listenersRef.current.bleData = undefined;
      }
    };
  }, [isRunning, dataSourceType, handleSerialData, handleBleData, setLastError]);

  useEffect(() => {
    const init = async () => {
      try {
        await dashboardApi.initDefaultParserScripts();
        const scripts = await dashboardApi.getParserScripts();
        setParserScripts(scripts);
      } catch (error) {
        console.error('Failed to initialize parser scripts:', error);
      }
    };

    init();

    if (!currentDashboard) {
      createNewDashboard();
    }
  }, []);

  return (
    <Layout style={{ height: '100%', background: 'transparent' }}>
      <DashboardToolbar />
      <Layout>
        <Content style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
          <DashboardCanvas />
        </Content>
        <Sider
          width={320}
          theme="light"
          style={{ borderLeft: '1px solid #f0f0f0' }}
        >
          <DashboardPanel />
        </Sider>
      </Layout>
    </Layout>
  );
};

export default DashboardPage;
