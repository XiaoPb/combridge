import { useEffect, useState, useRef } from 'react';
import { Card, Table, Tag, Space, Button, Select, Input, Typography, Empty } from 'antd';
import {
  ClearOutlined,
  DownloadOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import {
  useLogStore,
  formatLogTimestamp,
  levelColors,
  levelTexts,
  type LogLevel,
  type LogEntry,
} from '../../stores/logStore';

const { Text } = Typography;
const { Search } = Input;

const LogViewer: React.FC = () => {
  const logs = useLogStore((state) => state.logs);
  const clearLogs = useLogStore((state) => state.clearLogs);
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
      .map((log: LogEntry) => `[${formatLogTimestamp(log.timestamp)}] [${log.level.toUpperCase()}] [${log.source}] ${log.message}`)
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
      title: '时间',
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 120,
      render: (timestamp: number) => (
        <Text style={{ fontSize: 12 }}>{formatLogTimestamp(timestamp)}</Text>
      ),
    },
    {
      title: '级别',
      dataIndex: 'level',
      key: 'level',
      width: 80,
      render: (level: LogLevel) => (
        <Tag color={levelColors[level]}>{levelTexts[level]}</Tag>
      ),
    },
    {
      title: '来源',
      dataIndex: 'source',
      key: 'source',
      width: 120,
      render: (source: string) => <Text code>{source}</Text>,
    },
    {
      title: '消息',
      dataIndex: 'message',
      key: 'message',
      render: (message: string) => <Text>{message}</Text>,
    },
  ];

  const sources = [...new Set(logs.map((log: LogEntry) => log.source))];

  return (
    <Card
      title="日志查看器"
      extra={
        <Space>
          <Button icon={<ClearOutlined />} onClick={handleClear}>
            清空
          </Button>
          <Button icon={<DownloadOutlined />} onClick={handleExport}>
            导出
          </Button>
          <Button
            icon={<ReloadOutlined />}
            type={autoScroll ? 'primary' : 'default'}
            onClick={() => setAutoScroll(!autoScroll)}
          >
            {autoScroll ? '停止滚动' : '自动滚动'}
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
            { value: 'all', label: '全部级别' },
            { value: 'debug', label: '调试' },
            { value: 'info', label: '信息' },
            { value: 'warn', label: '警告' },
            { value: 'error', label: '错误' },
          ]}
        />
        <Select
          value={sourceFilter}
          onChange={setSourceFilter}
          style={{ width: 150 }}
          placeholder="选择来源"
          allowClear
          options={sources.map((s) => ({ value: s, label: s }))}
        />
        <Search
          placeholder="搜索日志内容"
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
          <Empty description="暂无日志" />
        )}
      </div>
    </Card>
  );
};

export default LogViewer;
