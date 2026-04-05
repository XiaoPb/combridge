import { useState, useMemo } from 'react';
import { Card, Table, Tag, Space, Button, Typography, Empty, Tooltip } from 'antd';
import {
  ClearOutlined,
  DownloadOutlined,
  ArrowUpOutlined,
  ArrowDownOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import LogEntry from './LogEntry';
import LogFilter from './LogFilter';

const { Text } = Typography;

export interface DataLogEntry {
  id: string;
  timestamp: number;
  direction: 'send' | 'receive';
  data: number[];
  format: 'hex' | 'text' | 'binary';
  source?: string;
  note?: string;
}

interface DataLoggerProps {
  entries: DataLogEntry[];
  onClear?: () => void;
  onExport?: () => void;
  maxHeight?: number;
  showFilter?: boolean;
  showExport?: boolean;
  autoScroll?: boolean;
}

const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp);
  const ms = String(date.getMilliseconds()).padStart(3, '0');
  return date.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }) + '.' + ms;
};

const formatData = (data: number[], format: 'hex' | 'text' | 'binary'): string => {
  switch (format) {
    case 'hex':
      return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
    case 'text':
      return new TextDecoder().decode(new Uint8Array(data));
    case 'binary':
      return data.map((b) => b.toString(2).padStart(8, '0')).join(' ');
    default:
      return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
  }
};

const DataLogger: React.FC<DataLoggerProps> = ({
  entries,
  onClear,
  onExport,
  maxHeight = 400,
  showFilter = true,
  showExport = true,
  autoScroll: _autoScroll = true,
}) => {
  const { t } = useTranslation('common');
  const [directionFilter, setDirectionFilter] = useState<'all' | 'send' | 'receive'>('all');
  const [formatFilter, setFormatFilter] = useState<'hex' | 'text' | 'binary'>('hex');
  const [searchText, setSearchText] = useState('');

  const filteredEntries = useMemo(() => {
    let filtered = entries;

    if (directionFilter !== 'all') {
      filtered = filtered.filter((entry) => entry.direction === directionFilter);
    }

    if (searchText) {
      filtered = filtered.filter((entry) => {
        const dataStr = formatData(entry.data, formatFilter);
        return dataStr.toLowerCase().includes(searchText.toLowerCase());
      });
    }

    return filtered;
  }, [entries, directionFilter, formatFilter, searchText]);

  const handleExport = () => {
    if (onExport) {
      onExport();
      return;
    }

    const content = filteredEntries
      .map((entry) => {
        const dir = entry.direction === 'send' ? 'TX' : 'RX';
        const data = formatData(entry.data, formatFilter);
        return `[${formatTimestamp(entry.timestamp)}] [${dir}] ${data}`;
      })
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
      title: t('time'),
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 100,
      render: (timestamp: number) => (
        <Text style={{ fontSize: 11, fontFamily: 'monospace' }}>
          {formatTimestamp(timestamp)}
        </Text>
      ),
    },
    {
      title: t('direction', { ns: 'serial' }),
      dataIndex: 'direction',
      key: 'direction',
      width: 60,
      render: (direction: 'send' | 'receive') => (
        <Tag color={direction === 'send' ? 'blue' : 'green'} style={{ margin: 0 }}>
          {direction === 'send' ? <ArrowUpOutlined /> : <ArrowDownOutlined />}
        </Tag>
      ),
    },
    {
      title: t('data', { ns: 'ble' }),
      dataIndex: 'data',
      key: 'data',
      render: (data: number[], _record: DataLogEntry) => (
        <LogEntry data={data} format={formatFilter} />
      ),
    },
    {
      title: t('size'),
      dataIndex: 'data',
      key: 'length',
      width: 60,
      render: (data: number[]) => <Text type="secondary">{data.length}</Text>,
    },
  ];

  return (
    <Card
      size="small"
      extra={
        <Space>
          {showExport && (
            <Tooltip title={t('export')}>
              <Button size="small" icon={<DownloadOutlined />} onClick={handleExport}>
                {t('export')}
              </Button>
            </Tooltip>
          )}
          <Tooltip title={t('clear')}>
            <Button size="small" icon={<ClearOutlined />} onClick={onClear} danger>
              {t('clear')}
            </Button>
          </Tooltip>
        </Space>
      }
    >
      {showFilter && (
        <LogFilter
          directionFilter={directionFilter}
          formatFilter={formatFilter}
          searchText={searchText}
          onDirectionChange={setDirectionFilter}
          onFormatChange={setFormatFilter}
          onSearchChange={setSearchText}
        />
      )}

      <div style={{ maxHeight, overflow: 'auto' }}>
        {filteredEntries.length > 0 ? (
          <Table
            dataSource={filteredEntries}
            columns={columns}
            rowKey="id"
            size="small"
            pagination={false}
            showHeader={true}
          />
        ) : (
          <Empty description={t('noData')} style={{ padding: 20 }} />
        )}
      </div>
    </Card>
  );
};

export default DataLogger;
