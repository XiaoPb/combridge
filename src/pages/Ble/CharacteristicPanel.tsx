import React, { useState } from 'react';
import { Card, Space, Button, Input, Radio, Typography, Tag, Descriptions, Divider, message } from 'antd';
import { ReadOutlined, SendOutlined, BellOutlined, StopOutlined, CopyOutlined } from '@ant-design/icons';
import type { BleCharacteristic } from '../../types';
import { formatBleData } from '../../stores/bleStore';

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
      <Card title="特征操作面板" size="small">
        <div style={{ textAlign: 'center', padding: '40px 0', color: '#999' }}>
          请从 GATT 浏览器中选择一个特征
        </div>
      </Card>
    );
  }

  const { uuid, properties, value } = characteristic;

  const handleWrite = () => {
    if (!inputData.trim()) {
      message.warning('请输入数据');
      return;
    }
    onWrite(uuid, inputData, inputFormat, withoutResponse);
  };

  const handleCopyValue = () => {
    if (value) {
      const text = formatBleData(value, inputFormat);
      navigator.clipboard.writeText(text);
      message.success('已复制到剪贴板');
    }
  };

  return (
    <Card title="特征操作面板" size="small">
      <Space vertical style={{ width: '100%' }}>
        <Descriptions size="small" column={1} bordered>
          <Descriptions.Item label="UUID">
            <Text code style={{ fontSize: '11px' }}>
              {uuid}
            </Text>
          </Descriptions.Item>
          <Descriptions.Item label="属性">
            <Space wrap>
              {properties.read && <Tag color="green">读取</Tag>}
              {properties.write && <Tag color="blue">写入</Tag>}
              {properties.writeWithoutResponse && <Tag color="cyan">无响应写入</Tag>}
              {properties.notify && <Tag color="orange">通知</Tag>}
              {properties.indicate && <Tag color="purple">指示</Tag>}
            </Space>
          </Descriptions.Item>
          {value && (
            <Descriptions.Item label="当前值">
              <Space vertical style={{ width: '100%' }}>
                <Space>
                  <Text code>{formatBleData(value, 'hex')}</Text>
                  <Button
                    type="text"
                    size="small"
                    icon={<CopyOutlined />}
                    onClick={handleCopyValue}
                  />
                </Space>
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  {formatBleData(value, 'text')}
                </Text>
              </Space>
            </Descriptions.Item>
          )}
        </Descriptions>

        <Divider style={{ margin: '12px 0' }} />

        <Space wrap>
          {properties.read && (
            <Button
              type="primary"
              icon={<ReadOutlined />}
              onClick={() => onRead(uuid)}
            >
              读取
            </Button>
          )}
          {properties.notify && (
            <>
              <Button
                icon={<BellOutlined />}
                onClick={() => onSubscribe(uuid)}
              >
                订阅通知
              </Button>
              <Button
                icon={<StopOutlined />}
                onClick={() => onUnsubscribe(uuid)}
              >
                取消订阅
              </Button>
            </>
          )}
        </Space>

        {(properties.write || properties.writeWithoutResponse) && (
          <>
            <Divider style={{ margin: '12px 0' }} />

            <div>
              <Space style={{ marginBottom: 8 }}>
                <Text>数据格式：</Text>
                <Radio.Group
                  value={inputFormat}
                  onChange={(e) => setInputFormat(e.target.value)}
                  size="small"
                >
                  <Radio.Button value="text">文本</Radio.Button>
                  <Radio.Button value="hex">HEX</Radio.Button>
                </Radio.Group>
                {properties.writeWithoutResponse && (
                  <Radio.Group
                    value={withoutResponse}
                    onChange={(e) => setWithoutResponse(e.target.value)}
                    size="small"
                  >
                    <Radio.Button value={false}>等待响应</Radio.Button>
                    <Radio.Button value={true}>无响应</Radio.Button>
                  </Radio.Group>
                )}
              </Space>

              <TextArea
                value={inputData}
                onChange={(e) => setInputData(e.target.value)}
                placeholder={inputFormat === 'hex' ? '输入十六进制数据，如：01 02 03' : '输入文本数据'}
                rows={3}
                style={{ marginBottom: 8 }}
              />

              <Button
                type="primary"
                icon={<SendOutlined />}
                onClick={handleWrite}
              >
                写入
              </Button>
            </div>
          </>
        )}
      </Space>
    </Card>
  );
};

export default CharacteristicPanel;
