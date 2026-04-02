import React, { useState } from 'react';
import { Card, Input, Button, Space, Segmented, Select, Switch, Typography, Tooltip } from 'antd';
import { SendOutlined, HistoryOutlined, DeleteOutlined } from '@ant-design/icons';

const { TextArea } = Input;
const { Text } = Typography;

interface SerialSendPanelProps {
  isConnected: boolean;
  onSend: (data: string, format: 'hex' | 'text') => void;
}

const SerialSendPanel: React.FC<SerialSendPanelProps> = ({
  isConnected,
  onSend,
}) => {
  const [inputData, setInputData] = useState('');
  const [format, setFormat] = useState<'hex' | 'text'>('text');
  const [appendNewline, setAppendNewline] = useState(true);
  const [history, setHistory] = useState<string[]>([]);
  const [showHistory, setShowHistory] = useState(false);

  const handleSend = () => {
    if (!inputData.trim()) return;

    let dataToSend = inputData;
    if (format === 'text' && appendNewline) {
      dataToSend += '\n';
    }

    onSend(dataToSend, format);
    
    if (!history.includes(inputData)) {
      setHistory((prev) => [inputData, ...prev].slice(0, 20));
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && e.ctrlKey) {
      handleSend();
    }
  };

  const handleHistorySelect = (value: string) => {
    setInputData(value);
    setShowHistory(false);
  };

  const clearHistory = () => {
    setHistory([]);
  };

  return (
    <Card
      title="发送面板"
      size="small"
      extra={
        <Space>
          <Segmented
            value={format}
            onChange={(value) => setFormat(value as 'hex' | 'text')}
            options={[
              { value: 'text', label: '文本' },
              { value: 'hex', label: 'HEX' },
            ]}
          />
          {format === 'text' && (
            <Tooltip title="追加换行符">
              <Space>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  追加换行
                </Text>
                <Switch
                  size="small"
                  checked={appendNewline}
                  onChange={setAppendNewline}
                />
              </Space>
            </Tooltip>
          )}
        </Space>
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} size="middle">
        <div style={{ display: 'flex', gap: 8 }}>
          <TextArea
            value={inputData}
            onChange={(e) => setInputData(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder={format === 'hex' ? '输入十六进制数据，如: 01 02 03 FF' : '输入要发送的文本'}
            disabled={!isConnected}
            autoSize={{ minRows: 3, maxRows: 6 }}
            style={{ flex: 1, fontFamily: format === 'hex' ? 'Consolas, Monaco, monospace' : 'inherit' }}
          />
          <Button
            type="primary"
            icon={<SendOutlined />}
            onClick={handleSend}
            disabled={!isConnected || !inputData.trim()}
            style={{ height: 'auto' }}
          >
            发送
          </Button>
        </div>

        {history.length > 0 && (
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
              <Button
                type="text"
                size="small"
                icon={<HistoryOutlined />}
                onClick={() => setShowHistory(!showHistory)}
              >
                历史记录 ({history.length})
              </Button>
              {showHistory && (
                <Button
                  type="text"
                  size="small"
                  danger
                  icon={<DeleteOutlined />}
                  onClick={clearHistory}
                >
                  清空
                </Button>
              )}
            </div>
            {showHistory && (
              <Select
                style={{ width: '100%' }}
                placeholder="选择历史记录"
                onChange={handleHistorySelect}
                options={history.map((item) => ({
                  value: item,
                  label: (
                    <div style={{ maxWidth: 400, overflow: 'hidden', textOverflow: 'ellipsis' }}>
                      {item}
                    </div>
                  ),
                }))}
              />
            )}
          </div>
        )}

        <Text type="secondary" style={{ fontSize: 12 }}>
          提示: 按 Ctrl+Enter 快速发送
        </Text>
      </Space>
    </Card>
  );
};

export default SerialSendPanel;
