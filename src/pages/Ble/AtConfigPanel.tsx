import React, { useState } from 'react';
import { Card, Form, Select, Input, Button, Space, Typography, Divider, Alert, Table, Tag } from 'antd';
import { SendOutlined, ClearOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { SerialPortInfo } from '../../types';

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

interface AtCommandConfig {
  name: string;
  command: string;
  description: string;
  params?: string;
}

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
  const { t } = useTranslation('ble');
  const [form] = Form.useForm();
  const [customCommand, setCustomCommand] = useState('');
  const [commandParam, setCommandParam] = useState('');
  const [logs, setLogs] = useState<AtLogEntry[]>([]);
  const [selectedCommand, setSelectedCommand] = useState<string | null>(null);

  const AT_COMMANDS: AtCommandConfig[] = [
    { name: t('atCommand.reset'), command: 'AT+RESET', description: t('atCommand.resetDesc') },
    { name: t('atCommand.version'), command: 'AT+VERSION', description: t('atCommand.versionDesc') },
    { name: t('atCommand.setName'), command: 'AT+NAME', description: t('atCommand.setNameDesc'), params: t('label.name') },
    { name: t('atCommand.queryName'), command: 'AT+NAME?', description: t('atCommand.queryNameDesc') },
    { name: t('atCommand.setBaud'), command: 'AT+BAUD', description: t('atCommand.setBaudDesc'), params: t('label.baudRate', { ns: 'serial' }) },
    { name: t('atCommand.queryBaud'), command: 'AT+BAUD?', description: t('atCommand.queryBaudDesc') },
    { name: t('atCommand.setAdvInterval'), command: 'AT+ADVI', description: t('atCommand.setAdvIntervalDesc'), params: 'ms' },
    { name: t('atCommand.setConnInterval'), command: 'AT+CONN', description: t('atCommand.setConnIntervalDesc') },
    { name: t('atCommand.sleep'), command: 'AT+SLEEP', description: t('atCommand.sleepDesc') },
    { name: t('atCommand.wakeup'), command: 'AT+WAKEUP', description: t('atCommand.wakeupDesc') },
    { name: t('atCommand.factoryReset'), command: 'AT+DEFAULT', description: t('atCommand.factoryResetDesc') },
  ];

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
      title: t('label.time'),
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 100,
    },
    {
      title: t('label.direction'),
      dataIndex: 'direction',
      key: 'direction',
      width: 80,
      render: (dir: string) => (
        <Tag color={dir === 'send' ? 'blue' : 'green'}>
          {dir === 'send' ? t('display.send', { ns: 'serial' }) : t('display.receive', { ns: 'serial' })}
        </Tag>
      ),
    },
    {
      title: t('label.data'),
      dataIndex: 'data',
      key: 'data',
      render: (data: string) => <Text code>{data}</Text>,
    },
  ];

  return (
    <Card title={t('title.atCommandConfig')} size="small">
      <Space vertical style={{ width: '100%' }}>
        <Alert
          title={t('mode.at')}
          description={t('mode.atDesc')}
          type="info"
          showIcon
        />

        <Form form={form} layout="vertical" size="small">
          <Form.Item label={t('label.selectSerial')}>
            <Select
              value={selectedPort}
              onChange={onPortChange}
              placeholder={t('placeholder.selectPort')}
              options={ports.map((p) => ({
                label: p.name,
                value: p.name,
              }))}
            />
          </Form.Item>
        </Form>

        <Divider style={{ margin: '12px 0' }}>{t('label.presetCommands')}</Divider>

        <Select
          value={selectedCommand}
          onChange={(v) => setSelectedCommand(v)}
          placeholder={t('placeholder.selectPresetCommand')}
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
                placeholder={`${t('placeholder.inputParam')}: ${AT_COMMANDS.find((c) => c.command === selectedCommand)?.params}`}
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
              {t('button.sendCommand')}
            </Button>
          </>
        )}

        <Divider style={{ margin: '12px 0' }}>{t('label.customCommand')}</Divider>

        <TextArea
          value={customCommand}
          onChange={(e) => setCustomCommand(e.target.value)}
          placeholder={t('placeholder.inputCustomAt')}
          rows={2}
        />
        <Button
          type="primary"
          icon={<SendOutlined />}
          onClick={handleSendCustom}
          disabled={!selectedPort || !customCommand.trim()}
        >
          {t('button.send')}
        </Button>

        <Divider style={{ margin: '12px 0' }}>{t('label.commLog')}</Divider>

        <div style={{ marginBottom: 8 }}>
          <Button
            size="small"
            icon={<ClearOutlined />}
            onClick={handleClearLogs}
          >
            {t('button.clearLog')}
          </Button>
        </div>

        <Table
          dataSource={logs}
          columns={columns}
          size="small"
          pagination={false}
          scroll={{ y: 200 }}
          locale={{ emptyText: t('placeholder.noLog') }}
        />
      </Space>
    </Card>
  );
};

export default AtConfigPanel;
