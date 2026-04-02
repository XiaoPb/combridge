import React from 'react';
import { Button, Select, Spin, Typography, Flex } from 'antd';
import { ReloadOutlined, UsbOutlined, DisconnectOutlined } from '@ant-design/icons';
import type { SerialConfig } from '../../types';
import { DEFAULT_BAUD_RATES } from '../../types';

const { Text } = Typography;

interface SerialToolbarProps {
  ports: Array<{
    name: string;
    port_type: string;
    manufacturer?: string;
    product?: string;
  }>;
  currentPort: string | null;
  config: SerialConfig;
  isScanning: boolean;
  isConnected: boolean;
  onScan: () => void;
  onSelectPort: (portName: string) => void;
  onOpen: () => void;
  onClose: () => void;
  onUpdateConfig: (config: Partial<SerialConfig>) => void;
}

const SerialToolbar: React.FC<SerialToolbarProps> = ({
  ports,
  currentPort,
  config,
  isScanning,
  isConnected,
  onScan,
  onSelectPort,
  onOpen,
  onClose,
  onUpdateConfig,
}) => {
  return (
    <div style={{ padding: '12px', background: 'var(--bg-secondary)', borderRadius: '8px' }}>
      <Flex gap="middle" wrap="wrap">
        <Button
          icon={isScanning ? <Spin size="small" /> : <ReloadOutlined />}
          onClick={onScan}
          disabled={isScanning}
        >
          扫描端口
        </Button>

        <Select
          style={{ width: 200 }}
          placeholder="选择串口"
          value={currentPort}
          onChange={onSelectPort}
          disabled={isConnected}
        options={(ports || []).map((port) => ({
            value: port.name,
            label: (
              <div>
                <Text strong>{port.name}</Text>
                {port.manufacturer && (
                  <Text type="secondary" style={{ marginLeft: 8, fontSize: 12 }}>
                    {port.manufacturer}
                  </Text>
                )}
              </div>
            ),
          }))}
        />

        <Select
          style={{ width: 120 }}
          value={config.baudRate}
          onChange={(value) => onUpdateConfig({ baudRate: value })}
          disabled={isConnected}
          options={DEFAULT_BAUD_RATES.map((rate) => ({
            value: rate,
            label: `${rate}`,
          }))}
        />

        <Select
          style={{ width: 100 }}
          value={config.dataBits}
          onChange={(value) => onUpdateConfig({ dataBits: value })}
          disabled={isConnected}
          options={[
            { value: 5, label: '5' },
            { value: 6, label: '6' },
            { value: 7, label: '7' },
            { value: 8, label: '8' },
          ]}
        />

        <Select
          style={{ width: 100 }}
          value={config.stopBits}
          onChange={(value) => onUpdateConfig({ stopBits: value })}
          disabled={isConnected}
          options={[
            { value: 1, label: '1' },
            { value: 2, label: '2' },
          ]}
        />

        <Select
          style={{ width: 100 }}
          value={config.parity}
          onChange={(value) => onUpdateConfig({ parity: value })}
          disabled={isConnected}
          options={[
            { value: 'none', label: '无' },
            { value: 'odd', label: '奇' },
            { value: 'even', label: '偶' },
          ]}
        />

        {isConnected ? (
          <Button
            type="primary"
            danger
            icon={<DisconnectOutlined />}
            onClick={onClose}
          >
            关闭串口
          </Button>
        ) : (
          <Button
            type="primary"
            icon={<UsbOutlined />}
            onClick={onOpen}
            disabled={!currentPort}
          >
            打开串口
          </Button>
        )}
      </Flex>
    </div>
  );
};

export default SerialToolbar;
