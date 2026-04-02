import React, { useState } from 'react';
import { Modal, Form, Select, InputNumber, Switch, Divider, Button, Space, message, Typography } from 'antd';
import { SettingOutlined, SoundOutlined, ReloadOutlined } from '@ant-design/icons';

const { Title } = Typography;

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  language: 'zh-CN' | 'en-US';
  autoReconnect: boolean;
  autoReconnectInterval: number;
  maxLogLines: number;
  soundEnabled: boolean;
  soundOnConnect: boolean;
  soundOnDisconnect: boolean;
  soundOnData: boolean;
}

const defaultSettings: AppSettings = {
  theme: 'system',
  language: 'zh-CN',
  autoReconnect: false,
  autoReconnectInterval: 3000,
  maxLogLines: 1000,
  soundEnabled: true,
  soundOnConnect: true,
  soundOnDisconnect: true,
  soundOnData: false,
};

const SettingsModal: React.FC<SettingsModalProps> = ({ open, onClose }) => {
  const [form] = Form.useForm<AppSettings>();
  const [loading, setLoading] = useState(false);

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      setLoading(true);

      console.log('Saving settings:', values);
      await new Promise((resolve) => setTimeout(resolve, 500));

      message.success('设置已保存');
      onClose();
    } catch (err) {
      message.error('保存设置失败');
    } finally {
      setLoading(false);
    }
  };

  const handleReset = () => {
    form.setFieldsValue(defaultSettings);
    message.info('设置已重置为默认值');
  };

  return (
    <Modal
      open={open}
      onCancel={onClose}
      title={
        <span>
          <SettingOutlined style={{ marginRight: 8 }} />
          系统设置
        </span>
      }
      width={600}
      footer={
        <Space>
          <Button onClick={handleReset} icon={<ReloadOutlined />}>
            重置默认
          </Button>
          <Button onClick={onClose}>取消</Button>
          <Button type="primary" onClick={handleSave} loading={loading}>
            保存
          </Button>
        </Space>
      }
    >
      <Form
        form={form}
        layout="vertical"
        initialValues={defaultSettings}
      >
        <Title level={5}>外观设置</Title>

        <Form.Item name="theme" label="主题">
          <Select
            options={[
              { value: 'light', label: '浅色' },
              { value: 'dark', label: '深色' },
              { value: 'system', label: '跟随系统' },
            ]}
          />
        </Form.Item>

        <Form.Item name="language" label="语言">
          <Select
            options={[
              { value: 'zh-CN', label: '简体中文' },
              { value: 'en-US', label: 'English' },
            ]}
          />
        </Form.Item>

        <Divider />

        <Title level={5}>连接设置</Title>

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

        <Divider />

        <Title level={5}>
          <SoundOutlined style={{ marginRight: 8 }} />
          声音设置
        </Title>

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
    </Modal>
  );
};

export default SettingsModal;
