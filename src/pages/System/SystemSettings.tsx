import React, { useEffect } from 'react';
import { Card, Form, InputNumber, Switch, Divider, Button, Space, message, Typography, Row, Col } from 'antd';
import { LinkOutlined, SoundOutlined, ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import configService, { AppConfig } from '../../services/configService';

const { Text } = Typography;

const SystemSettings: React.FC = () => {
  const { t } = useTranslation('system');
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
      message.success(t('message.settingsSaved'));
    } catch (err) {
      console.error('Failed to save settings:', err);
      message.error(t('message.saveFailed'));
    }
  };

  const handleReset = () => {
    configService.resetConfig();
    form.setFieldsValue(configService.getConfig());
    message.info(t('message.settingsReset'));
  };

  return (
    <div style={{ padding: '0 8px' }}>
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={12}>
          <Card
            title={
              <span>
                <LinkOutlined style={{ marginRight: 8 }} />
                {t('title.connectionSettings')}
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
                label={t('label.autoReconnect')}
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
                      label={t('label.reconnectInterval')}
                    >
                      <InputNumber min={1000} max={60000} step={1000} style={{ width: '100%' }} />
                    </Form.Item>
                  ) : null
                }
              </Form.Item>

              <Form.Item name="maxLogLines" label={t('label.maxLogLines')}>
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
                {t('title.soundSettings')}
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
                label={t('label.soundEnabled')}
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
                        label={t('label.soundOnConnect')}
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>

                      <Form.Item
                        name="soundOnDisconnect"
                        label={t('label.soundOnDisconnect')}
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>

                      <Form.Item
                        name="soundOnData"
                        label={t('label.soundOnData')}
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
                {t('button.resetDefaults')}
              </Button>
            </Space>
            <Divider type="vertical" />
            <Text type="secondary">
              {t('message.autoSaveHint')}
            </Text>
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default SystemSettings;
