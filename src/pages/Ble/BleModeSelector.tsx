import React from 'react';
import { Card, Radio, Select, Space, Typography, Alert } from 'antd';
import { ApiOutlined, UsbOutlined } from '@ant-design/icons';
import type { BleMode } from '../../stores/bleStore';

const { Text } = Typography;

interface BleModeSelectorProps {
  mode: BleMode;
  serialPort: string | null;
  ports: { portName: string }[];
  onModeChange: (mode: BleMode) => void;
  onSerialPortChange: (port: string) => void;
}

const BleModeSelector: React.FC<BleModeSelectorProps> = ({
  mode,
  serialPort,
  ports,
  onModeChange,
  onSerialPortChange,
}) => {
  return (
    <Card title="BLE 模式配置" size="small">
      <Space direction="vertical" style={{ width: '100%' }}>
        <div>
          <Text type="secondary">选择 BLE 工作模式：</Text>
          <Radio.Group
            value={mode}
            onChange={(e) => onModeChange(e.target.value)}
            style={{ marginTop: 8 }}
          >
            <Radio.Button value="native">
              <Space>
                <ApiOutlined />
                Native 模式
              </Space>
            </Radio.Button>
            <Radio.Button value="at">
              <Space>
                <UsbOutlined />
                AT 模式
              </Space>
            </Radio.Button>
          </Radio.Group>
        </div>

        {mode === 'native' && (
          <Alert
            message="Native 模式"
            description="使用系统原生 BLE 适配器进行蓝牙通信，支持完整的 BLE 功能。"
            type="info"
            showIcon
          />
        )}

        {mode === 'at' && (
          <>
            <div>
              <Text type="secondary">选择串口设备：</Text>
              <Select
                value={serialPort}
                onChange={onSerialPortChange}
                placeholder="选择串口"
                style={{ width: '100%', marginTop: 8 }}
                options={ports.map((p) => ({
                  label: p.portName,
                  value: p.portName,
                }))}
              />
            </div>
            <Alert
              message="AT 模式"
              description="通过串口连接 BLE 模块（如 HC-08、CC2541 等），使用 AT 指令进行通信。"
              type="info"
              showIcon
            />
          </>
        )}
      </Space>
    </Card>
  );
};

export default BleModeSelector;
