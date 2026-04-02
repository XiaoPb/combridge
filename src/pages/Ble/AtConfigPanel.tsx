import React, { useState } from 'react';
import { Card, Form, Select, Input, Button, Space, Typography, Divider, Alert, Table, Tag } from 'antd';
import { SendOutlined, ClearOutlined } from '@ant-design/icons';
import type { SerialPortInfo } from '../../types';

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

interface AtCommandConfig {
  name: string;
  command: string;
  description: string;
  params?: string;
}

const AT_COMMANDS: AtCommandConfig[] = [
  { name: '复位', command: 'AT+RESET', description: '复位模块' },
  { name: '版本', command: 'AT+VERSION', description: '查询固件版本' },
  { name: '设置名称', command: 'AT+NAME', description: '设置设备名称', params: '名称' },
  { name: '查询名称', command: 'AT+NAME?', description: '查询当前设备名称' },
  { name: '设置波特率', command: 'AT+BAUD', description: '设置串口波特率', params: '波特率' },
  { name: '查询波特率', command: 'AT+BAUD?', description: '查询当前波特率' },
  { name: '设置广播间隔', command: 'AT+ADVI', description: '设置广播间隔', params: '间隔(ms)' },
  { name: '设置连接间隔', command: 'AT+CONN', description: '设置连接参数' },
  { name: '进入休眠', command: 'AT+SLEEP', description: '进入低功耗模式' },
  { name: '唤醒', command: 'AT+WAKEUP', description: '从休眠唤醒' },
  { name: '恢复出厂', command: 'AT+DEFAULT', description: '恢复出厂设置' },
];

interface AtConfigPanelProps {
  ports: SerialPortInfo[];
  selectedPort: string | null;
  onPortChange: (port: string) => void;
  onSendCommand: (command: string) => void;
}

interface AtLogEntry {
  key: string;
  timestamp: string;
  direction: 'send' | 'receive';
  data: string;
}

const AtConfigPanel: React.FC<AtConfigPanelProps> = ({
  ports,
  selectedPort,
  onPortChange,
  onSendCommand,
}) => {
  const [form] = Form.useForm();
  const [customCommand, setCustomCommand] = useState('');
  const [commandParam, setCommandParam] = useState('');
  const [logs, setLogs] = useState<AtLogEntry[]>([]);
  const [selectedCommand, setSelectedCommand] = useState<string | null>(null);

  const addLog = (direction: 'send' | 'receive', data: string) => {
    const entry: AtLogEntry = {
      key: `${Date.now()}-${Math.random()}`,
      timestamp: new Date().toLocaleTimeString(),
      direction,
      data,
    };
    setLogs((prev) => [...prev.slice(-100), entry]);
  };

  const handleSendPreset = () => {
    if (!selectedCommand) return;
    const cmd = AT_COMMANDS.find((c) => c.command === selectedCommand);
    if (!cmd) return;

    let fullCommand = cmd.command;
    if (cmd.params && commandParam) {
      fullCommand = `${cmd.command}${commandParam}`;
    }

    onSendCommand(fullCommand);
    addLog('send', fullCommand);
  };

  const handleSendCustom = () => {
    if (!customCommand.trim()) return;
    onSendCommand(customCommand);
    addLog('send', customCommand);
    setCustomCommand('');
  };

  const handleClearLogs = () => {
    setLogs([]);
  };

  const columns = [
    {
      title: '时间',
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 100,
    },
    {
      title: '方向',
      dataIndex: 'direction',
      key: 'direction',
      width: 80,
      render: (dir: string) => (
        <Tag color={dir === 'send' ? 'blue' : 'green'}>
          {dir === 'send' ? '发送' : '接收'}
        </Tag>
      ),
    },
    {
      title: '数据',
      dataIndex: 'data',
      key: 'data',
      render: (data: string) => <Text code>{data}</Text>,
    },
  ];

  return (
    <Card title="AT 指令配置" size="small">
      <Space direction="vertical" style={{ width: '100%' }}>
        <Alert
          message="AT 模式说明"
          description="通过串口发送 AT 指令控制 BLE 模块。请先选择串口并确保模块已正确连接。"
          type="info"
          showIcon
        />

        <Form form={form} layout="vertical" size="small">
          <Form.Item label="串口选择">
            <Select
              value={selectedPort}
              onChange={onPortChange}
              placeholder="选择串口"
              options={ports.map((p) => ({
                label: p.portName,
                value: p.portName,
              }))}
            />
          </Form.Item>
        </Form>

        <Divider style={{ margin: '12px 0' }}>预设指令</Divider>

        <Select
          value={selectedCommand}
          onChange={(v) => setSelectedCommand(v)}
          placeholder="选择预设指令"
          style={{ width: '100%' }}
          options={AT_COMMANDS.map((cmd) => ({
            label: `${cmd.name} (${cmd.command})`,
            value: cmd.command,
          }))}
        />

        {selectedCommand && (
          <>
            <Paragraph type="secondary" style={{ marginBottom: 8 }}>
              {AT_COMMANDS.find((c) => c.command === selectedCommand)?.description}
            </Paragraph>
            {AT_COMMANDS.find((c) => c.command === selectedCommand)?.params && (
              <Input
                placeholder={`输入参数: ${AT_COMMANDS.find((c) => c.command === selectedCommand)?.params}`}
                value={commandParam}
                onChange={(e) => setCommandParam(e.target.value)}
              />
            )}
            <Button
              type="primary"
              icon={<SendOutlined />}
              onClick={handleSendPreset}
              disabled={!selectedPort}
            >
              发送指令
            </Button>
          </>
        )}

        <Divider style={{ margin: '12px 0' }}>自定义指令</Divider>

        <TextArea
          value={customCommand}
          onChange={(e) => setCustomCommand(e.target.value)}
          placeholder="输入自定义 AT 指令"
          rows={2}
        />
        <Button
          type="primary"
          icon={<SendOutlined />}
          onClick={handleSendCustom}
          disabled={!selectedPort || !customCommand.trim()}
        >
          发送
        </Button>

        <Divider style={{ margin: '12px 0' }}>通信日志</Divider>

        <div style={{ marginBottom: 8 }}>
          <Button
            size="small"
            icon={<ClearOutlined />}
            onClick={handleClearLogs}
          >
            清空日志
          </Button>
        </div>

        <Table
          dataSource={logs}
          columns={columns}
          size="small"
          pagination={false}
          scroll={{ y: 200 }}
          locale={{ emptyText: '暂无日志' }}
        />
      </Space>
    </Card>
  );
};

export default AtConfigPanel;
