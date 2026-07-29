import React from 'react';
import { Card, Form, AutoComplete, Select, InputNumber, Switch, Divider, Typography, Space } from 'antd';
import type { SerialConfig } from '../../types';
import { DEFAULT_BAUD_RATES, BAUD_RATE_MIN, BAUD_RATE_MAX } from '../../types';

const { Text, Title } = Typography;

interface SerialSettingsProps {
  config: SerialConfig;
  isConnected: boolean;
  onUpdateConfig: (config: Partial<SerialConfig>) => void;
}

const SerialSettings: React.FC<SerialSettingsProps> = ({
  config,
  isConnected,
  onUpdateConfig,
}) => {
  return (
    <Card title="串口设置" size="small">
      <Form layout="vertical" size="small">
        <Title level={5}>基本配置</Title>
        
        <Form.Item label="波特率">
          <AutoComplete
            value={config.baudRate.toString()}
            onChange={(value) => {
              const numValue = parseInt(value, 10);
              if (!isNaN(numValue)) {
                onUpdateConfig({ baudRate: numValue });
              }
            }}
            disabled={isConnected}
            options={DEFAULT_BAUD_RATES.map((rate) => ({
              value: rate.toString(),
              label: `${rate} bps`,
            }))}
            placeholder="选择或输入波特率"
            filterOption={(inputValue, option) =>
              option!.value.toLowerCase().indexOf(inputValue.toLowerCase()) !== -1
            }
          />
          <Text type="secondary" style={{ fontSize: 11, marginTop: 4 }}>
            支持 {BAUD_RATE_MIN} - {BAUD_RATE_MAX} bps
          </Text>
        </Form.Item>

        <Form.Item label="数据位">
          <Select
            value={config.dataBits}
            onChange={(value) => onUpdateConfig({ dataBits: value })}
            disabled={isConnected}
            options={[
              { value: 5, label: '5 位' },
              { value: 6, label: '6 位' },
              { value: 7, label: '7 位' },
              { value: 8, label: '8 位' },
            ]}
          />
        </Form.Item>

        <Form.Item label="停止位">
          <Select
            value={config.stopBits}
            onChange={(value) => onUpdateConfig({ stopBits: value })}
            disabled={isConnected}
            options={[
              { value: 1, label: '1 位' },
              { value: 2, label: '2 位' },
            ]}
          />
        </Form.Item>

        <Form.Item label="校验位">
          <Select
            value={config.parity}
            onChange={(value) => onUpdateConfig({ parity: value })}
            disabled={isConnected}
            options={[
              { value: 'none', label: '无校验' },
              { value: 'odd', label: '奇校验' },
              { value: 'even', label: '偶校验' },
            ]}
          />
        </Form.Item>

        <Form.Item label="流控制">
          <Select
            value={config.flowControl}
            onChange={(value) => onUpdateConfig({ flowControl: value })}
            disabled={isConnected}
            options={[
              { value: 'none', label: '无' },
              { value: 'hardware', label: '硬件 (RTS/CTS)' },
              { value: 'software', label: '软件 (XON/XOFF)' },
            ]}
          />
        </Form.Item>

        <Divider />

        <Title level={5}>显示设置</Title>
        
        <Form.Item label="时间戳格式">
          <Select
            defaultValue="full"
            options={[
              { value: 'full', label: '完整时间 (HH:mm:ss.SSS)' },
              { value: 'short', label: '简短时间 (HH:mm:ss)' },
              { value: 'relative', label: '相对时间' },
            ]}
          />
        </Form.Item>

        <Form.Item label="自动滚动">
          <Switch defaultChecked />
        </Form.Item>

        <Form.Item label="显示换行符">
          <Switch defaultChecked />
        </Form.Item>

        <Divider />

        <Title level={5}>高级设置</Title>

        <Form.Item label="读取超时 (ms)">
          <InputNumber
            defaultValue={1000}
            min={100}
            max={10000}
            step={100}
            style={{ width: '100%' }}
          />
        </Form.Item>

        <Form.Item label="写入超时 (ms)">
          <InputNumber
            defaultValue={1000}
            min={100}
            max={10000}
            step={100}
            style={{ width: '100%' }}
          />
        </Form.Item>

        <Divider />

        <Space vertical style={{ width: '100%' }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            当前配置:
          </Text>
          <Text code style={{ fontSize: 11 }}>
            {config.baudRate}, {config.dataBits}, {config.stopBits}, {config.parity}, {config.flowControl}
          </Text>
        </Space>
      </Form>
    </Card>
  );
};

export default SerialSettings;
