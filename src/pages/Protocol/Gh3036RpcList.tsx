import React, { useState } from 'react';
import { Button, Input, Space, Typography, Tag, message, Row, Col, theme } from 'antd';
import { PlayCircleOutlined, CaretRightOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import type { Gh3036RpcCommand, Gh3036RpcParam } from '../../api/types';

const { Text } = Typography;
const { useToken } = theme;

const Gh3036RpcList: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { token } = useToken();
  const { rpcCommands, expandedCommand, setExpandedCommand, executeRpc, txChannel } = useGh3036Store();
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

  const getParamValues = (command: Gh3036RpcCommand): string[] => {
    return command.params.map((param) => {
      return paramValues[command.key]?.[param.name] ?? param.default_value ?? '';
    });
  };

  const handleExecute = async (command: Gh3036RpcCommand) => {
    if (!txChannel) {
      message.error(t('gh3036.noTxChannel'));
      return;
    }

    const params = getParamValues(command);
    console.log('GH3036 handleExecute:', command.key, params);
    
    const success = await executeRpc(command.key, params);
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
            background: isActive ? token.colorPrimaryBg : token.colorFillSecondary,
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
          <div style={{ padding: '8px 8px 8px 24px', background: token.colorBgContainer, borderRadius: '0 0 4px 4px', border: `1px solid ${token.colorBorderSecondary}`, borderTop: 'none' }}>
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
