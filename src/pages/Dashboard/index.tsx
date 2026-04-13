import React, { useEffect, useRef } from 'react';
import { Layout, theme } from 'antd';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import { useTheme } from '../../hooks';
import { dashboardApi } from '../../api/dashboard';
import { onSerialData, onBleData, onParsedData } from '../../api/events';
import { useLogStore } from '../../stores/logStore';
import DashboardTabs from './DashboardTabs';
import DashboardCanvas from './DashboardCanvas';
import ConsolePanel from './ConsolePanel';
import SettingsPanel from './SettingsPanel';
import JsonEditor from './JsonEditor';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { SerialDataEvent, BleDataEvent, ParsedDataEvent } from '../../api/events';

const { Content, Sider } = Layout;

const DashboardPage: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { token } = theme.useToken();
  const { isDark } = useTheme();
  const {
    activeTabs,
    setJsonFiles,
    isRunning,
    dataSourceType,
    addRawDataPoint,
    addParsedDataPoint,
    setLastError,
  } = useDashboardStore();

  const listenersRef = useRef<{
    serialData?: UnlistenFn;
    bleData?: UnlistenFn;
    parsedData?: UnlistenFn;
  }>({});

  useEffect(() => {
    const init = async () => {
      try {
        await dashboardApi.initDefaultParserScripts();
        const scripts = await dashboardApi.getParserScripts();
        useDashboardStore.getState().setParserScripts(scripts);

        const jsonFiles = await dashboardApi.getJsonFiles();
        setJsonFiles(jsonFiles);
      } catch (error) {
        console.error('Failed to initialize dashboard:', error);
      }
    };

    init();
  }, [setJsonFiles]);

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
      if (listenersRef.current.parsedData) {
        listenersRef.current.parsedData();
        listenersRef.current.parsedData = undefined;
      }

      if (!isRunning) {
        useLogStore.getState().addLog('info', 'Dashboard', '数据流已停止');
        return;
      }

      if (dataSourceType === 'serial') {
        try {
          listenersRef.current.serialData = await onSerialData((event: SerialDataEvent) => {
            addRawDataPoint({
              timestamp: event.timestamp ?? Date.now(),
              data: event.data,
              direction: 'RX',
            });
          });
          useLogStore.getState().addLog('info', 'Dashboard', '串口数据监听已启动');
        } catch (error) {
          const errorMsg = `启动串口监听失败: ${error}`;
          console.error(`[Dashboard] ${errorMsg}`);
          setLastError(errorMsg);
        }
      } else if (dataSourceType === 'ble') {
        try {
          listenersRef.current.bleData = await onBleData((event: BleDataEvent) => {
            addRawDataPoint({
              timestamp: event.timestamp ?? Date.now(),
              data: event.data,
              direction: 'RX',
            });
          });
          useLogStore.getState().addLog('info', 'Dashboard', '蓝牙数据监听已启动');
        } catch (error) {
          const errorMsg = `启动蓝牙监听失败: ${error}`;
          console.error(`[Dashboard] ${errorMsg}`);
          setLastError(errorMsg);
        }
      }

      try {
        listenersRef.current.parsedData = await onParsedData((event: ParsedDataEvent) => {
          addParsedDataPoint({
            timestamp: event.timestamp,
            values: event.values,
          });
        });
      } catch (error) {
        console.error('[Dashboard] Failed to register parsed data listener:', error);
      }
    };

    setupDataListeners();

    return () => {
      if (listenersRef.current.serialData) {
        listenersRef.current.serialData();
      }
      if (listenersRef.current.bleData) {
        listenersRef.current.bleData();
      }
      if (listenersRef.current.parsedData) {
        listenersRef.current.parsedData();
      }
    };
  }, [isRunning, dataSourceType, addRawDataPoint, addParsedDataPoint, setLastError]);

  const isJsonEditorActive = activeTabs.includes('jsonEditor');

  if (isJsonEditorActive) {
    return (
      <Layout style={{ height: '100%', background: 'transparent' }}>
        <DashboardTabs />
        <JsonEditor />
      </Layout>
    );
  }

  const showDashboard = activeTabs.includes('dashboard');
  const showConsole = activeTabs.includes('console');
  const showSettings = activeTabs.includes('settings');

  return (
    <Layout style={{ height: '100%', background: 'transparent' }}>
      <DashboardTabs />
      <Layout>
        <Content style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
          {showDashboard && !showConsole && <DashboardCanvas />}
          {showConsole && !showDashboard && <ConsolePanel />}
          {showDashboard && showConsole && (
            <Layout style={{ height: '100%' }}>
              <Content style={{ flex: 1 }}>
                <DashboardCanvas />
              </Content>
              <Sider width={400} theme={isDark ? 'dark' : 'light'} style={{ borderLeft: `1px solid ${token.colorBorderSecondary}` }}>
                <ConsolePanel />
              </Sider>
            </Layout>
          )}
          {!showDashboard && !showConsole && (
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: token.colorTextQuaternary }}>
              {t('selectTabHint') || '请选择要显示的标签页'}
            </div>
          )}
        </Content>
        {showSettings && (
          <Sider width={320} theme={isDark ? 'dark' : 'light'} style={{ borderLeft: `1px solid ${token.colorBorderSecondary}` }}>
            <SettingsPanel />
          </Sider>
        )}
      </Layout>
    </Layout>
  );
};

export default DashboardPage;
