import { useState } from 'react';
import { Card, Form, Input, InputNumber, Switch, Button, Space, message, Divider, Typography } from 'antd';
import { SaveOutlined, ReloadOutlined, LinkOutlined, DisconnectOutlined } from '@ant-design/icons';

const { Text } = Typography;

interface WebSocketConfig {
  url: string;
  autoReconnect: boolean;
  maxReconnectAttempts: number;
  reconnectInterval: number;
  pingInterval: number;
  pingTimeout: number;
}

const WebSocketConfig: React.FC = () => {
  const [form] = Form.useForm<WebSocketConfig>();
  const [connecting, setConnecting] = useState(false);
  const [connected, setConnected] = useState(false);

  const handleConnect = async () => {
    try {
      const values = await form.validateFields();
      setConnecting(true);

      console.log('Connecting to:', values.url);
      await new Promise((resolve) => setTimeout(resolve, 1000));

      setConnected(true);
      message.success(`已连接到 ${values.url}`);
    } catch (err) {
      message.error('连接失败: ' + (err instanceof Error ? err.message : '未知错误'));
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    try {
      console.log('Disconnecting...');
      await new Promise((resolve) => setTimeout(resolve, 500));
      setConnected(false);
      message.info('已断开连接');
    } catch (err) {
      message.error('断开连接失败');
    }
  };

  const handleSaveConfig = async () => {
    try {
      const values = await form.validateFields();
      console.log('Saving config:', values);
      message.success('配置已保存');
    } catch (err) {
      message.error('保存配置失败');
    }
  };

  return (
    <Card
      title={
        <span>
          <LinkOutlined style={{ marginRight: 8 }} />
          WebSocket 配置
        </span>
      }
      extra={
        <Space>
          {connected ? (
            <Button
              danger
              icon={<DisconnectOutlined />}
              onClick={handleDisconnect}
            >
              断开连接
            </Button>
          ) : (
            <Button
              type="primary"
              icon={<LinkOutlined />}
              loading={connecting}
              onClick={handleConnect}
            >
              连接
            </Button>
          )}
        </Space>
      }
    >
      <Form
        form={form}
        layout="vertical"
        initialValues={{
          url: 'ws://localhost:8080',
          autoReconnect: true,
          maxReconnectAttempts: 5,
          reconnectInterval: 3000,
          pingInterval: 30000,
          pingTimeout: 5000,
        }}
      >
        <Form.Item
          name="url"
          label="服务器地址"
          rules={[
            { required: true, message: '请输入服务器地址' },
            { pattern: /^wss?:\/\/.+/, message: '请输入有效的 WebSocket 地址' },
          ]}
        >
          <Input
            placeholder="ws://localhost:8080"
            disabled={connected}
          />
        </Form.Item>

        <Divider>重连设置</Divider>

        <Form.Item
          name="autoReconnect"
          label="自动重连"
          valuePropName="checked"
        >
          <Switch disabled={connected} />
        </Form.Item>

        <Form.Item
          noStyle
          shouldUpdate={(prev, curr) => prev.autoReconnect !== curr.autoReconnect}
        >
          {({ getFieldValue }) =>
            getFieldValue('autoReconnect') ? (
              <>
                <Form.Item
                  name="maxReconnectAttempts"
                  label="最大重连次数"
                >
                  <InputNumber
                    min={1}
                    max={100}
                    style={{ width: '100%' }}
                    disabled={connected}
                  />
                </Form.Item>

                <Form.Item
                  name="reconnectInterval"
                  label="重连间隔 (毫秒)"
                >
                  <InputNumber
                    min={1000}
                    max={60000}
                    step={1000}
                    style={{ width: '100%' }}
                    disabled={connected}
                  />
                </Form.Item>
              </>
            ) : null
          }
        </Form.Item>

        <Divider>心跳设置</Divider>

        <Form.Item
          name="pingInterval"
          label="心跳间隔 (毫秒)"
        >
          <InputNumber
            min={0}
            max={300000}
            step={1000}
            style={{ width: '100%' }}
            disabled={connected}
          />
        </Form.Item>

        <Form.Item
          name="pingTimeout"
          label="心跳超时 (毫秒)"
        >
          <InputNumber
            min={1000}
            max={60000}
            step={1000}
            style={{ width: '100%' }}
            disabled={connected}
          />
        </Form.Item>

        <Form.Item>
          <Space>
            <Button
              type="primary"
              icon={<SaveOutlined />}
              onClick={handleSaveConfig}
            >
              保存配置
            </Button>
            <Button
              icon={<ReloadOutlined />}
              onClick={() => form.resetFields()}
            >
              重置
            </Button>
          </Space>
        </Form.Item>
      </Form>

      {connected && (
        <>
          <Divider>连接状态</Divider>
          <Space direction="vertical" style={{ width: '100%' }}>
            <Text>
              状态: <Text type="success">已连接</Text>
            </Text>
            <Text type="secondary">
              已发送: 1,234 字节 | 已接收: 5,678 字节
            </Text>
          </Space>
        </>
      )}
    </Card>
  );
};

export default WebSocketConfig;
