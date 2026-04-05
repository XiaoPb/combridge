import React, { useState } from 'react';
import { Button, Input, Space, Typography, Tag, message, Row, Col } from 'antd';
import { PlayCircleOutlined, CaretRightOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import type { Gh3036RpcCommand, Gh3036RpcParam } from '../../api/types';

const { Text } = Typography;

const Gh3036RpcList: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { rpcCommands, expandedCommand, setExpandedCommand, sendData, txChannel } = useGh3036Store();
  const [paramValues, setParamValues] = useState<Record<string, Record<string, string>>>({});

  const handleParamChange = (commandKey: string, paramName: string, value: string) => {
    setParamValues((prev) => ({
      ...prev,
      [commandKey]: {
        ...prev[commandKey],
        [paramName]: value,
      },
    }));
  };

  const buildCommandData = (command: Gh3036RpcCommand): number[] => {
    const header = [0xAA, 0x11];
    const keyBytes = new TextEncoder().encode(command.key);
    const keyLen = keyBytes.length;
    
    const data: number[] = [...header, keyLen, ...keyBytes];
    
    return data;
  };

  const handleExecute = async (command: Gh3036RpcCommand) => {
    if (!txChannel) {
      message.error(t('gh3036.noTxChannel'));
      return;
    }

    const data = buildCommandData(command);
    const success = await sendData(data);
    if (success) {
      message.success(t('gh3036.commandSent', { name: command.name }));
    }
  };

  const renderParamInput = (command: Gh3036RpcCommand, param: Gh3036RpcParam) => {
    const value = paramValues[command.key]?.[param.name] ?? param.default_value ?? '';

    if (param.param_type.includes('[]')) {
      return (
        <Input.TextArea
          size="small"
          placeholder={t('gh3036.paramArrayPlaceholder')}
          value={value}
          onChange={(e) => handleParamChange(command.key, param.name, e.target.value)}
          rows={2}
        />
      );
    }

    return (
      <Input
        size="small"
        placeholder={param.description}
        value={value}
        onChange={(e) => handleParamChange(command.key, param.name, e.target.value)}
      />
    );
  };

  const renderCommand = (command: Gh3036RpcCommand) => {
    const isActive = expandedCommand === command.key;

    return (
      <div key={command.key} style={{ marginBottom: 4 }}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '4px 8px',
            background: isActive ? '#e6f7ff' : '#fafafa',
            borderRadius: 4,
            cursor: 'pointer',
          }}
          onClick={() => setExpandedCommand(isActive ? null : command.key)}
        >
          <Space>
            <CaretRightOutlined rotate={isActive ? 90 : 0} style={{ fontSize: 10 }} />
            <Text strong style={{ fontSize: 12 }}>{command.name}</Text>
            <Tag color="blue" style={{ fontSize: 10, margin: 0 }}>{command.key}</Tag>
          </Space>
          <Button
            type="primary"
            size="small"
            icon={<PlayCircleOutlined />}
            onClick={(e) => {
              e.stopPropagation();
              handleExecute(command);
            }}
          >
            {t('gh3036.execute')}
          </Button>
        </div>

        {isActive && (
          <div style={{ padding: '8px 8px 8px 24px', background: '#fff', borderRadius: '0 0 4px 4px' }}>
            <Text type="secondary" style={{ fontSize: 11 }}>{command.description}</Text>
            
            {command.params.length > 0 && (
              <div style={{ marginTop: 8 }}>
                {command.params.map((param) => (
                  <div key={param.name} style={{ marginBottom: 8 }}>
                    <div style={{ marginBottom: 4 }}>
                      <Text style={{ fontSize: 11 }}>{param.name}</Text>
                      <Text type="secondary" style={{ fontSize: 10, marginLeft: 4 }}>
                        ({param.param_type})
                      </Text>
                    </div>
                    {renderParamInput(command, param)}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    );
  };

  return (
    <Row gutter={[8, 8]}>
      {rpcCommands.map((command) => (
        <Col span={12} key={command.key}>
          {renderCommand(command)}
        </Col>
      ))}
    </Row>
  );
};

export default Gh3036RpcList;
