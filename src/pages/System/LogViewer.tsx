import { useEffect, useState, useRef, useMemo } from 'react';
import { Card, Table, Tag, Space, Button, Select, Input, Typography, Empty, Switch, Divider, message } from 'antd';
import {
  ClearOutlined,
  DownloadOutlined,
  ReloadOutlined,
  SaveOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import {
  useLogStore,
  formatLogTimestamp,
  levelColors,
  levelTexts,
  type LogLevel,
  type LogEntry,
} from '../../stores/logStore';
import { useConfigStore } from '../../stores/configStore';
import { systemApi, type LogConfig, type LogLevel as BackendLogLevel } from '../../api/tauri';

const { Text } = Typography;
const { Search } = Input;

const LOG_LEVEL_OPTIONS: Array<{ value: BackendLogLevel; label: string }> = [
  { value: 'error', label: 'Error' },
  { value: 'warn', label: 'Warn' },
  { value: 'info', label: 'Info' },
  { value: 'debug', label: 'Debug' },
  { value: 'trace', label: 'Trace' },
];

const DEFAULT_LOG_CONFIG: LogConfig = {
  level: 'info',
  maxFiles: 10,
  maxSizeMb: 10,
  consoleEnabled: true,
  fileEnabled: true,
  filePath: '',
  modules: [
    { name: 'rpc-core', enabled: true, level: 'info' },
    { name: 'gh3036', enabled: true, level: 'info' },
    { name: 'ble', enabled: true, level: 'info' },
    { name: 'event-bus', enabled: true, level: 'warn' },
    { name: 'device', enabled: true, level: 'info' },
    { name: 'frontend', enabled: true, level: 'info' },
  ],
};

const LogViewer: React.FC = () => {
  const { t } = useTranslation('system');
  const logs = useLogStore((state) => state.logs);
  const clearLogs = useLogStore((state) => state.clearLogs);
  const timezone = useConfigStore((state) => state.settings.timezone);
  const hasHydrated = useConfigStore((state) => state._hasHydrated);
  const [filteredLogs, setFilteredLogs] = useState<LogEntry[]>([]);
  const [levelFilter, setLevelFilter] = useState<string>('all');
  const [sourceFilter, setSourceFilter] = useState<string>('');
  const [searchText, setSearchText] = useState('');
  const [autoScroll, setAutoScroll] = useState(true);
  const [logConfig, setLogConfig] = useState<LogConfig>(DEFAULT_LOG_CONFIG);
  const [savingConfig, setSavingConfig] = useState(false);
  const tableRef = useRef<HTMLDivElement>(null);

  const effectiveTimezone = hasHydrated ? timezone : 'Asia/Shanghai';

  const columns = useMemo(() => [
    {
      title: t('logViewer.timestamp'),
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 120,
      render: (timestamp: number) => (
        <Text style={{ fontSize: 12 }}>{formatLogTimestamp(timestamp, effectiveTimezone)}</Text>
      ),
    },
    {
      title: t('logViewer.level'),
      dataIndex: 'level',
      key: 'level',
      width: 80,
      render: (level: LogLevel) => (
        <Tag color={levelColors[level]}>{levelTexts[level]}</Tag>
      ),
    },
    {
      title: t('logViewer.source'),
      dataIndex: 'source',
      key: 'source',
      width: 120,
      render: (source: string) => <Text style={{ fontSize: 12 }}>{source}</Text>,
    },
    {
      title: t('logViewer.message'),
      dataIndex: 'message',
      key: 'message',
      render: (message: string) => (
        <Text style={{ fontSize: 12, wordBreak: 'break-all' }}>{message}</Text>
      ),
    },
  ], [t, effectiveTimezone]);

  useEffect(() => {
    const loadLogConfig = async () => {
      try {
        const config = await systemApi.getLogConfig();
        setLogConfig({
          ...DEFAULT_LOG_CONFIG,
          ...config,
          modules: config.modules?.length ? config.modules : DEFAULT_LOG_CONFIG.modules,
        });
      } catch (error) {
        console.error('Failed to load log config:', error);
      }
    };
    loadLogConfig();
  }, []);

  useEffect(() => {
    let filtered: LogEntry[] = logs;

    if (levelFilter !== 'all') {
      filtered = filtered.filter((log: LogEntry) => log.level === levelFilter);
    }

    if (sourceFilter) {
      filtered = filtered.filter((log: LogEntry) =>
        log.source.toLowerCase().includes(sourceFilter.toLowerCase())
      );
    }

    if (searchText) {
      filtered = filtered.filter((log: LogEntry) =>
        log.message.toLowerCase().includes(searchText.toLowerCase())
      );
    }

    setFilteredLogs(filtered);
  }, [logs, levelFilter, sourceFilter, searchText]);

  useEffect(() => {
    if (autoScroll && tableRef.current) {
      tableRef.current.scrollTop = tableRef.current.scrollHeight;
    }
  }, [filteredLogs, autoScroll]);

  const handleClear = () => {
    clearLogs();
  };

  const handleExport = () => {
    const content = logs
      .map((log: LogEntry) => `[${formatLogTimestamp(log.timestamp, effectiveTimezone)}] [${log.level.toUpperCase()}] [${log.source}] ${log.message}`)
      .join('\n');
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `combridge-log-${new Date().toISOString().slice(0, 10)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleSaveLogConfig = async () => {
    setSavingConfig(true);
    try {
      await systemApi.configureLog(logConfig);
      message.success(t('logViewer.config.saved'));
    } catch (error) {
      console.error('Failed to save log config:', error);
      message.error(t('logViewer.config.saveFailed'));
    } finally {
      setSavingConfig(false);
    }
  };

  const handleResetLogConfig = () => {
    setLogConfig(DEFAULT_LOG_CONFIG);
  };

  const updateModule = (name: string, patch: Partial<LogConfig['modules'][number]>) => {
    setLogConfig((current) => ({
      ...current,
      modules: current.modules.map((module) =>
        module.name === name ? { ...module, ...patch } : module
      ),
    }));
  };

  const sources = [...new Set(logs.map((log: LogEntry) => log.source))];

  return (
    <Card
      title={t('logViewer.title')}
      extra={
        <Space>
          <Button icon={<ClearOutlined />} onClick={handleClear}>
            {t('logViewer.action.clear')}
          </Button>
          <Button icon={<DownloadOutlined />} onClick={handleExport}>
            {t('logViewer.action.export')}
          </Button>
          <Button
            icon={<ReloadOutlined />}
            type={autoScroll ? 'primary' : 'default'}
            onClick={() => setAutoScroll(!autoScroll)}
          >
            {autoScroll ? t('auto', { ns: 'common' }) : t('manual', { ns: 'common' })}
          </Button>
        </Space>
      }
    >
      <div style={{ marginBottom: 16 }}>
        <Space wrap align="center">
          <Text strong>{t('logViewer.config.title')}</Text>
          <Select
            value={logConfig.level}
            onChange={(level) => setLogConfig((current) => ({ ...current, level }))}
            style={{ width: 112 }}
            options={LOG_LEVEL_OPTIONS}
          />
          <Switch
            checked={logConfig.consoleEnabled}
            onChange={(checked) => setLogConfig((current) => ({ ...current, consoleEnabled: checked }))}
            checkedChildren={t('logViewer.config.console')}
            unCheckedChildren={t('logViewer.config.console')}
          />
          <Switch
            checked={logConfig.fileEnabled}
            onChange={(checked) => setLogConfig((current) => ({ ...current, fileEnabled: checked }))}
            checkedChildren={t('logViewer.config.file')}
            unCheckedChildren={t('logViewer.config.file')}
          />
          <Button icon={<SaveOutlined />} type="primary" loading={savingConfig} onClick={handleSaveLogConfig}>
            {t('logViewer.config.save')}
          </Button>
          <Button onClick={handleResetLogConfig}>
            {t('logViewer.config.reset')}
          </Button>
        </Space>

        <Space wrap style={{ marginTop: 12 }}>
          {logConfig.modules.map((module) => (
            <Space
              key={module.name}
              size={6}
              style={{
                padding: '4px 8px',
                border: '1px solid #f0f0f0',
                borderRadius: 6,
                background: module.enabled ? '#fff' : '#fafafa',
              }}
            >
              <Switch
                size="small"
                checked={module.enabled}
                onChange={(checked) => updateModule(module.name, { enabled: checked })}
              />
              <Text style={{ width: 76, fontSize: 12 }}>{module.name}</Text>
              <Select
                size="small"
                value={module.level}
                disabled={!module.enabled}
                onChange={(level) => updateModule(module.name, { level })}
                style={{ width: 92 }}
                options={LOG_LEVEL_OPTIONS}
              />
            </Space>
          ))}
        </Space>
      </div>

      <Divider style={{ margin: '8px 0 16px' }} />

      <Space style={{ marginBottom: 16 }} wrap>
        <Select
          value={levelFilter}
          onChange={setLevelFilter}
          style={{ width: 120 }}
          options={[
            { value: 'all', label: t('logViewer.filter.all') },
            { value: 'debug', label: 'Debug' },
            { value: 'info', label: t('logViewer.filter.info') },
            { value: 'warn', label: t('logViewer.filter.warning') },
            { value: 'error', label: t('logViewer.filter.error') },
          ]}
        />
        <Select
          value={sourceFilter}
          onChange={setSourceFilter}
          style={{ width: 150 }}
          placeholder={t('logViewer.source')}
          allowClear
          options={sources.map((s) => ({ value: s, label: s }))}
        />
        <Search
          placeholder={t('search', { ns: 'common' })}
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          style={{ width: 200 }}
          allowClear
        />
      </Space>

      <div
        ref={tableRef}
        style={{ maxHeight: 400, overflow: 'auto' }}
      >
        {filteredLogs.length > 0 ? (
          <Table
            key={effectiveTimezone}
            dataSource={filteredLogs}
            columns={columns}
            rowKey="id"
            size="small"
            pagination={false}
            showHeader={true}
          />
        ) : (
          <Empty description={t('noData', { ns: 'common' })} />
        )}
      </div>
    </Card>
  );
};

export default LogViewer;
