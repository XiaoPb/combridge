import React, { useEffect, useState, useRef } from 'react';
import { Card, Table, Tag, Space, Button, Select, Input, Typography, Empty } from 'antd';
import {
  ClearOutlined,
  DownloadOutlined,
  FilterOutlined,
  ReloadOutlined,
} from '@ant-design/icons';

const { Text } = Typography;
const { Search } = Input;

interface LogEntry {
  id: string;
  timestamp: number;
  level: 'info' | 'warn' | 'error' | 'debug';
  source: string;
  message: string;
}

const levelColors = {
  debug: 'default',
  info: 'blue',
  warn: 'orange',
  error: 'red',
};

const levelTexts = {
  debug: '调试',
  info: '信息',
  warn: '警告',
  error: '错误',
};

const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp);
  return date.toLocaleString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    fractionalSecondDigits: 3,
  });
};

const LogViewer: React.FC = () => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [filteredLogs, setFilteredLogs] = useState<LogEntry[]>([]);
  const [levelFilter, setLevelFilter] = useState<string>('all');
  const [sourceFilter, setSourceFilter] = useState<string>('');
  const [searchText, setSearchText] = useState('');
  const [autoScroll, setAutoScroll] = useState(true);
  const tableRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const mockLogs: LogEntry[] = [
      {
        id: '1',
        timestamp: Date.now(),
        level: 'info',
        source: 'SerialManager',
        message: '串口 COM3 已打开',
      },
      {
        id: '2',
        timestamp: Date.now() - 1000,
        level: 'debug',
        source: 'SerialPort',
        message: '接收到数据: 48 65 6C 6C 6F',
      },
      {
        id: '3',
        timestamp: Date.now() - 2000,
        level: 'warn',
        source: 'BleManager',
        message: 'BLE设备连接超时，正在重试...',
      },
      {
        id: '4',
        timestamp: Date.now() - 3000,
        level: 'error',
        source: 'WebSocket',
        message: 'WebSocket连接失败: Connection refused',
      },
    ];
    setLogs(mockLogs);
  }, []);

  useEffect(() => {
    let filtered = logs;

    if (levelFilter !== 'all') {
      filtered = filtered.filter((log) => log.level === levelFilter);
    }

    if (sourceFilter) {
      filtered = filtered.filter((log) =>
        log.source.toLowerCase().includes(sourceFilter.toLowerCase())
      );
    }

    if (searchText) {
      filtered = filtered.filter((log) =>
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
    setLogs([]);
  };

  const handleExport = () => {
    const content = logs
      .map((log) => `[${formatTimestamp(log.timestamp)}] [${log.level.toUpperCase()}] [${log.source}] ${log.message}`)
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
        <Text style={{ fontSize: 12 }}>{formatTimestamp(timestamp)}</Text>
      ),
    },
    {
      title: '级别',
      dataIndex: 'level',
      key: 'level',
      width: 80,
      render: (level: keyof typeof levelColors) => (
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

  const sources = [...new Set(logs.map((log) => log.source))];

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
