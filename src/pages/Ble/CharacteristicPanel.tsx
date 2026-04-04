import React, { useState } from 'react';
import { Card, Space, Button, Input, Segmented, Typography, Tag, message, Tooltip } from 'antd';
import { ReadOutlined, SendOutlined, BellOutlined, StopOutlined } from '@ant-design/icons';
import type { BleCharacteristic } from '../../types';
import { getShortUuid } from '../../stores/bleStore';
import { getCharacteristicName } from '../../types/ble';

const { Text } = Typography;
const { TextArea } = Input;

interface CharacteristicPanelProps {
  characteristic: BleCharacteristic | null;
  onRead: (uuid: string) => void;
  onWrite: (uuid: string, data: string, format: 'hex' | 'text', withoutResponse: boolean) => void;
  onSubscribe: (uuid: string) => void;
  onUnsubscribe: (uuid: string) => void;
}

const CharacteristicPanel: React.FC<CharacteristicPanelProps> = ({
  characteristic,
  onRead,
  onWrite,
  onSubscribe,
  onUnsubscribe,
}) => {
  const [inputData, setInputData] = useState('');
  const [inputFormat, setInputFormat] = useState<'hex' | 'text'>('text');
  const [withoutResponse, setWithoutResponse] = useState(false);

  if (!characteristic) {
    return (
      <Card 
        title={<Text style={{ fontSize: 13 }}>特征操作面板</Text>} 
        size="small"
        style={{ height: 320 }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 'calc(100% - 40px)', color: '#999' }}>
          请从 GATT 服务树中选择一个特征
        </div>
      </Card>
    );
  }

  const { uuid, properties } = characteristic;
  const canWrite = properties.write || properties.writeWithoutResponse;
  const canRead = properties.read;
  const canNotify = properties.notify || properties.indicate;

  const handleWrite = () => {
    if (!inputData.trim()) {
      message.warning('请输入数据');
      return;
    }
    onWrite(uuid, inputData, inputFormat, withoutResponse);
  };

  const getWriteModeText = () => {
    if (properties.write && properties.writeWithoutResponse) {
      return '写入: 支持';
    } else if (properties.writeWithoutResponse) {
      return '写入: 无响应写入';
    } else if (properties.write) {
      return '写入: 响应写入';
    }
    return '';
  };

  return (
    <Card 
      title={<Text style={{ fontSize: 13 }}>特征操作面板</Text>}
      size="small"
      style={{ height: 320 }}
      styles={{ body: { padding: '8px 12px', height: 'calc(100% - 40px)', display: 'flex', flexDirection: 'column' } }}
    >
      <div style={{ display: 'flex', gap: 8, marginBottom: 8, flexShrink: 0 }}>
        <div style={{ flex: '1 1 0', minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
            <Text type="secondary" style={{ fontSize: 12, flexShrink: 0 }}>UUID:</Text>
            <Text code style={{ fontSize: 11, wordBreak: 'break-all' }}>{uuid}</Text>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Text type="secondary" style={{ fontSize: 12, flexShrink: 0 }}>名称:</Text>
            <Text style={{ fontSize: 12 }}>{getCharacteristicName(uuid)} (0x{getShortUuid(uuid)})</Text>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginTop: 4 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>{getWriteModeText()}</Text>
            {properties.notify && <Tag color="orange" style={{ fontSize: 10, padding: '0 4px', margin: 0 }}>通知</Tag>}
            {properties.indicate && <Tag color="purple" style={{ fontSize: 10, padding: '0 4px', margin: 0 }}>指示</Tag>}
          </div>
        </div>
        
        <div style={{ flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 4, alignItems: 'flex-end' }}>
          {canNotify && (
            <Space size={4}>
              <Tooltip title="订阅通知">
                <Button
                  size="small"
                  icon={<BellOutlined />}
                  onClick={() => onSubscribe(uuid)}
                >
                  订阅
                </Button>
              </Tooltip>
              <Tooltip title="取消订阅">
                <Button
                  size="small"
                  icon={<StopOutlined />}
                  onClick={() => onUnsubscribe(uuid)}
                >
                  取消订阅
                </Button>
              </Tooltip>
            </Space>
          )}
          {canRead && (
            <Tooltip title="读取特征值">
              <Button
                size="small"
                type="primary"
                icon={<ReadOutlined />}
                onClick={() => onRead(uuid)}
              >
                读取
              </Button>
            </Tooltip>
          )}
        </div>
      </div>

      {canWrite && (
        <>
          <div style={{ flex: '1 1 0', display: 'flex', gap: 8, minHeight: 0 }}>
            <TextArea
              value={inputData}
              onChange={(e) => setInputData(e.target.value)}
              placeholder={inputFormat === 'hex' ? '输入十六进制数据，如：01 02 03 FF' : '输入要发送的文本数据'}
              style={{ 
                flex: '1 1 0', 
                resize: 'none',
                fontFamily: inputFormat === 'hex' ? 'Consolas, Monaco, monospace' : 'inherit',
                fontSize: 12,
              }}
            />
            
            <div style={{ flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 4, justifyContent: 'space-between' }}>
              <Segmented
                value={inputFormat}
                onChange={(value) => setInputFormat(value as 'hex' | 'text')}
                size="small"
                options={[
                  { value: 'text', label: 'TEXT' },
                  { value: 'hex', label: 'HEX' },
                ]}
              />
              
              {properties.writeWithoutResponse && (
                <Segmented
                  value={withoutResponse ? 'noResponse' : 'withResponse'}
                  onChange={(value) => setWithoutResponse(value === 'noResponse')}
                  size="small"
                  options={[
                    { value: 'withResponse', label: '等待响应' },
                    { value: 'noResponse', label: '无响应' },
                  ]}
                />
              )}
              
              <Button
                type="primary"
                icon={<SendOutlined />}
                onClick={handleWrite}
                disabled={!inputData.trim()}
                block
              >
                发送
              </Button>
            </div>
          </div>
        </>
      )}
      
      {!canWrite && (
        <div style={{ flex: '1 1 0', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#999', fontSize: 12 }}>
          该特征不支持写入操作
        </div>
      )}
    </Card>
  );
};

export default CharacteristicPanel;
