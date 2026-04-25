import React, { useEffect, useRef, useState } from 'react';
import { Card, Button, Space, Segmented, Empty, Typography, Tag } from 'antd';
import { ClearOutlined, DownloadOutlined, ArrowDownOutlined, ArrowUpOutlined } from '@ant-design/icons';
import type { DataEntry } from '../../stores/serialStore';
import { formatTimestamp, formatData } from '../../stores/serialStore';
import { useConfigStore } from '../../stores/configStore';

const { Text } = Typography;

interface SerialDataViewProps {
  receivedData: DataEntry[];
  sentData: DataEntry[];
  onClear: () => void;
}

const SerialDataView: React.FC<SerialDataViewProps> = ({
  receivedData,
  sentData,
  onClear,
}) => {
  const [displayFormat, setDisplayFormat] = useState<'hex' | 'text'>('hex');
  const [displayMode, setDisplayMode] = useState<'all' | 'receive' | 'send'>('all');
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const timezone = useConfigStore((state) => state.settings.timezone);
  const hasHydrated = useConfigStore((state) => state._hasHydrated);
  
  const effectiveTimezone = hasHydrated ? timezone : 'Asia/Shanghai';

  const allData = [...(receivedData || []), ...(sentData || [])].sort((a, b) => a.timestamp - b.timestamp);

  const filteredData = displayMode === 'all'
    ? allData
    : displayMode === 'receive'
    ? receivedData
            : sentData;

    useEffect(() => {
        if (autoScroll && containerRef.current) {
            containerRef.current.scrollTop = containerRef.current.scrollHeight;
        }
    }, [filteredData, autoScroll]);

    const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
        const target = e.currentTarget;
        const isAtBottom = target.scrollHeight - target.scrollTop === target.clientHeight;
        setAutoScroll(isAtBottom);
    };

    const handleExport = () => {
        const content = filteredData
            .map((entry) => {
                const timestamp = formatTimestamp(entry.timestamp, effectiveTimezone);
                const direction = entry.direction === 'receive' ? 'RX' : 'TX';
                const data = formatData(entry.data, displayFormat);
                return `[${timestamp}] ${direction}: ${data}`;
            })
            .join('\n');

        const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `serial-data-${Date.now()}.txt`;
        a.click();
        URL.revokeObjectURL(url);
    };

    
    if (import.meta.env.DEV) {
        console.debug('[SerialDataView] props:', { receivedData, sentData, onClear });
        console.debug('[SerialDataView] receivedData:', receivedData);
        console.debug('[SerialDataView] sentData:', sentData);
        console.debug('[SerialDataView] filteredData:', filteredData);
    }
    

    return (
        <Card
            title="数据视图"
            size="small"
            extra={
                <Space>
                    <Segmented
                        value={displayMode}
                        onChange={(value) => setDisplayMode(value as 'all' | 'receive' | 'send')}
                        options={[
                            { value: 'all', label: '全部' },
                            { value: 'receive', label: '接收' },
                            { value: 'send', label: '发送' },
                        ]}
                    />
                    <Segmented
                        value={displayFormat}
                        onChange={(value) => setDisplayFormat(value as 'hex' | 'text')}
                        options={[
                            { value: 'hex', label: 'HEX' },
                            { value: 'text', label: 'TEXT' },
                        ]}
                    />
                    <Button icon={<DownloadOutlined />} onClick={handleExport} disabled={(filteredData || []).length === 0}>
                        导出
                    </Button>
                    <Button icon={<ClearOutlined />} onClick={onClear} disabled={(filteredData || []).length === 0}>
                        清空
                    </Button>
                </Space>
            }
        >
            <div
                ref={containerRef}
                onScroll={handleScroll}
                style={{
                    height: 400,
                    overflow: 'auto',
                    background: 'var(--bg-primary)',
                    padding: 8,
                    borderRadius: 4,
                    fontFamily: 'Consolas, Monaco, monospace',
                    fontSize: 13,
                }}
            >
                {(filteredData || []).length === 0 ? (
                    <Empty description="暂无数据" style={{ marginTop: 150 }} />
                ) : (
                    (filteredData || []).map((entry) => (
                        <div
                            key={entry.id}
                            style={{
                                padding: '4px 8px',
                                marginBottom: 4,
                                background: entry.direction === 'receive' ? 'rgba(82, 196, 26, 0.1)' : 'rgba(24, 144, 255, 0.1)',
                                borderRadius: 4,
                                borderLeft: `3px solid ${entry.direction === 'receive' ? '#52c41a' : '#1890ff'}`,
                            }}
                        >
                            <Space size={8}>
                                <Tag color={entry.direction === 'receive' ? 'success' : 'processing'}>
                                    {entry.direction === 'receive' ? <ArrowDownOutlined /> : <ArrowUpOutlined />}
                                    <span style={{ marginLeft: 4 }}>{entry.direction === 'receive' ? 'RX' : 'TX'}</span>
                                </Tag>
                                <Text type="secondary" style={{ fontSize: 12 }}>
                                    {formatTimestamp(entry.timestamp, effectiveTimezone)}
                                </Text>
                                <Text>[{(entry.data || []).length} bytes]</Text>
                            </Space>
                            <div style={{ marginTop: 4, wordBreak: 'break-all' }}>
                                {formatData(entry.data, displayFormat)}
                            </div>
                        </div>
                    ))
                )}
            </div>
        </Card>
    );
};

export default SerialDataView;
