import React, { useEffect } from 'react';
import { Card, Form, InputNumber, Switch, Divider, Button, Space, message, Typography, Row, Col, Select } from 'antd';
import { LinkOutlined, SoundOutlined, ReloadOutlined, ClockCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useConfigStore, type AppConfig } from '../../stores/configStore';
import { systemApi } from '../../api/tauri';

const { Text } = Typography;

const TIMEZONE_OPTIONS = [
  { value: 'Asia/Shanghai', label: '中国标准时间 (UTC+8)' },
  { value: 'Asia/Tokyo', label: '日本标准时间 (UTC+9)' },
  { value: 'Asia/Seoul', label: '韩国标准时间 (UTC+9)' },
  { value: 'Asia/Singapore', label: '新加坡标准时间 (UTC+8)' },
  { value: 'Asia/Hong_Kong', label: '香港标准时间 (UTC+8)' },
  { value: 'Asia/Taipei', label: '台北标准时间 (UTC+8)' },
  { value: 'America/New_York', label: '美国东部时间 (UTC-5/-4)' },
  { value: 'America/Los_Angeles', label: '美国太平洋时间 (UTC-8/-7)' },
  { value: 'America/Chicago', label: '美国中部时间 (UTC-6/-5)' },
  { value: 'Europe/London', label: '伦敦时间 (UTC+0/+1)' },
  { value: 'Europe/Paris', label: '巴黎时间 (UTC+1/+2)' },
  { value: 'Europe/Berlin', label: '柏林时间 (UTC+1/+2)' },
  { value: 'UTC', label: '协调世界时 (UTC)' },
];

const SystemSettings: React.FC = () => {
  const { t } = useTranslation('system');
  const [form] = Form.useForm<AppConfig>();
  const settings = useConfigStore((s) => s.settings);
  const updateConfig = useConfigStore((s) => s.updateConfig);
  const resetConfig = useConfigStore((s) => s.resetConfig);
  const getConfig = useConfigStore((s) => s.getConfig);

  useEffect(() => {
    form.setFieldsValue(settings);
    
    const syncTimezone = async () => {
      try {
        await systemApi.setTimezone(settings.timezone);
      } catch (error) {
        console.error('Failed to sync timezone:', error);
      }
    };
    syncTimezone();
  }, [settings, form]);

  const handleValuesChange = async (changedValues: Partial<AppConfig>, _allValues: AppConfig) => {
    try {
      updateConfig(changedValues);
      message.success(t('message.settingsSaved'));
      
      if (changedValues.timezone) {
        try {
          await systemApi.setTimezone(changedValues.timezone);
          message.success(t('message.timezoneUpdated', { defaultValue: '时区设置已更新' }));
        } catch (error) {
          console.error('Failed to update timezone:', error);
          message.error(t('message.timezoneUpdateFailed', { defaultValue: '时区设置更新失败' }));
        }
      }
    } catch (err) {
      console.error('Failed to save settings:', err);
      message.error(t('message.saveFailed'));
    }
  };

  const handleReset = () => {
    resetConfig();
    form.setFieldsValue(getConfig());
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

        <Col xs={24} lg={12}>
          <Card
            title={
              <span>
                <ClockCircleOutlined style={{ marginRight: 8 }} />
                {t('title.timezoneSettings')}
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
                name="timezone"
                label={t('label.timezone')}
              >
                <Select
                  options={TIMEZONE_OPTIONS}
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t('label.timezoneHint')}
              </Text>
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
            <Divider orientation="vertical" />
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
