import React, { useEffect } from 'react';
import { Card, Form, InputNumber, Switch, Divider, Button, Space, message, Typography, Row, Col } from 'antd';
import { LinkOutlined, SoundOutlined, ReloadOutlined } from '@ant-design/icons';
import configService, { AppConfig } from '../../services/configService';

const { Text } = Typography;

const SystemSettings: React.FC = () => {
  const [form] = Form.useForm<AppConfig>();

  useEffect(() => {
    const config = configService.getConfig();
    form.setFieldsValue(config);

    const unsubscribe = configService.subscribe((newConfig) => {
      form.setFieldsValue(newConfig);
    });

    return unsubscribe;
  }, [form]);

  const handleValuesChange = async (changedValues: Partial<AppConfig>) => {
    try {
      configService.updateConfig(changedValues);
      message.success('设置已保存');
    } catch (err) {
      console.error('Failed to save settings:', err);
      message.error('保存设置失败');
    }
  };

  const handleReset = () => {
    configService.resetConfig();
    form.setFieldsValue(configService.getConfig());
    message.info('设置已重置为默认值');
  };

  return (
    <div style={{ padding: '0 8px' }}>
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={12}>
          <Card
            title={
              <span>
                <LinkOutlined style={{ marginRight: 8 }} />
                连接设置
              </span>
            }
            size="small"
          >
            <Form
              form={form}
              layout="vertical"
              onValuesChange={handleValuesChange}
            >
              <Form.Item
                name="autoReconnect"
                label="自动重连"
                valuePropName="checked"
              >
                <Switch />
              </Form.Item>

              <Form.Item
                noStyle
                shouldUpdate={(prev, curr) => prev.autoReconnect !== curr.autoReconnect}
              >
                {({ getFieldValue }) =>
                  getFieldValue('autoReconnect') ? (
                    <Form.Item
                      name="autoReconnectInterval"
                      label="重连间隔 (毫秒)"
                    >
                      <InputNumber min={1000} max={60000} step={1000} style={{ width: '100%' }} />
                    </Form.Item>
                  ) : null
                }
              </Form.Item>

              <Form.Item name="maxLogLines" label="最大日志行数">
                <InputNumber min={100} max={10000} step={100} style={{ width: '100%' }} />
              </Form.Item>
            </Form>
          </Card>
        </Col>

        <Col xs={24} lg={12}>
          <Card
            title={
              <span>
                <SoundOutlined style={{ marginRight: 8 }} />
                声音设置
              </span>
            }
            size="small"
          >
            <Form
              form={form}
              layout="vertical"
              onValuesChange={handleValuesChange}
            >
              <Form.Item
                name="soundEnabled"
                label="启用声音"
                valuePropName="checked"
              >
                <Switch />
              </Form.Item>

              <Form.Item
                noStyle
                shouldUpdate={(prev, curr) => prev.soundEnabled !== curr.soundEnabled}
              >
                {({ getFieldValue }) =>
                  getFieldValue('soundEnabled') ? (
                    <>
                      <Form.Item
                        name="soundOnConnect"
                        label="连接时播放声音"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>

                      <Form.Item
                        name="soundOnDisconnect"
                        label="断开时播放声音"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>

                      <Form.Item
                        name="soundOnData"
                        label="数据传输时播放声音"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>
                    </>
                  ) : null
                }
              </Form.Item>
            </Form>
          </Card>
        </Col>

        <Col xs={24}>
          <Card size="small">
            <Space>
              <Button onClick={handleReset} icon={<ReloadOutlined />}>
                重置默认设置
              </Button>
            </Space>
            <Divider type="vertical" />
            <Text type="secondary">
              设置修改后会自动保存
            </Text>
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default SystemSettings;
