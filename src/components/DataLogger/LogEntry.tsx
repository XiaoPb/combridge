import React, { useState, useMemo } from 'react';
import { Typography, Tooltip } from 'antd';

const { Text } = Typography;

interface LogEntryProps {
  data: number[];
  format: 'hex' | 'text' | 'binary';
  maxLength?: number;
  showCopy?: boolean;
}

const formatData = (data: number[], format: 'hex' | 'text' | 'binary'): string => {
  switch (format) {
    case 'hex':
      return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
    case 'text':
      try {
        return new TextDecoder().decode(new Uint8Array(data));
      } catch {
        return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
      }
    case 'binary':
      return data.map((b) => b.toString(2).padStart(8, '0')).join(' ');
    default:
      return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
  }
};

const LogEntry: React.FC<LogEntryProps> = ({
  data,
  format,
  maxLength = 100,
  showCopy = true,
}) => {
  const [copied, setCopied] = useState(false);

  const formattedData = useMemo(() => formatData(data, format), [data, format]);

  const displayData = useMemo(() => {
    if (formattedData.length > maxLength) {
      return formattedData.substring(0, maxLength) + '...';
    }
    return formattedData;
  }, [formattedData, maxLength]);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(formattedData);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        cursor: showCopy ? 'pointer' : 'default',
      }}
      onClick={showCopy ? handleCopy : undefined}
    >
      <Tooltip title={copied ? '已复制!' : showCopy ? '点击复制' : formattedData}>
        <Text
          style={{
            fontSize: 12,
            fontFamily: 'monospace',
            wordBreak: 'break-all',
          }}
        >
          {displayData}
        </Text>
      </Tooltip>
      {data.length > maxLength / 3 && (
        <Text type="secondary" style={{ fontSize: 11 }}>
          ({data.length} 字节)
        </Text>
      )}
    </div>
  );
};

export default LogEntry;
