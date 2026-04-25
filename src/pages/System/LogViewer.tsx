import { useEffect, useState, useRef } from 'react';
import { Card, Table, Tag, Space, Button, Select, Input, Typography, Empty } from 'antd';
import {
  ClearOutlined,
  DownloadOutlined,
  ReloadOutlined,
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

const { Text } = Typography;
const { Search } = Input;

const LogViewer: React.FC = () => {
  const { t } = useTranslation('system');
  const logs = useLogStore((state) => state.logs);
  const clearLogs = useLogStore((state) => state.clearLogs);
  const timezone = useConfigStore((state) => state.settings.timezone);
  const [filteredLogs, setFilteredLogs] = useState<LogEntry[]>([]);
  const [levelFilter, setLevelFilter] = useState<string>('all');
  const [sourceFilter, setSourceFilter] = useState<string>('');
  const [searchText, setSearchText] = useState('');
  const [autoScroll, setAutoScroll] = useState(true);
  const tableRef = useRef<HTMLDivElement>(null);

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
      .map((log: LogEntry) => `[${formatLogTimestamp(log.timestamp, timezone)}] [${log.level.toUpperCase()}] [${log.source}] ${log.message}`)
      .join('\n');
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `combridge-log-${new Date().toISOString().slice(0, 10)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const columns = [
    {
      title: t('logViewer.timestamp'),
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 120,
      render: (timestamp: number) => (
        <Text style={{ fontSize: 12 }}>{formatLogTimestamp(timestamp, timezone)}</Text>
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
      render: (source: string) => <Text code>{source}</Text>,
    },
    {
      title: t('logViewer.message'),
      dataIndex: 'message',
      key: 'message',
      render: (message: string) => <Text>{message}</Text>,
    },
  ];

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
