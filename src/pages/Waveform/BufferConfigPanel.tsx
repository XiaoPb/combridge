import React, { useState } from 'react';
import { Card, Form, InputNumber, Button, Space, Select, message, Input } from 'antd';
import { PlusOutlined, DeleteOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useWaveformStore, DEFAULT_BUFFER_CONFIG } from '../../stores/waveformStore';

const BufferConfigPanel: React.FC = () => {
  const { t } = useTranslation('waveform');
  const [form] = Form.useForm();
  const [newBufferId, setNewBufferId] = useState('');
  
  const {
    buffers,
    currentBuffer,
    createBuffer,
    removeBuffer,
    setCurrentBuffer,
    isLoading,
  } = useWaveformStore();

  const handleCreateBuffer = async () => {
    if (!newBufferId.trim()) {
      message.warning(t('buffer.idRequired'));
      return;
    }

    const values = await form.validateFields();
    await createBuffer(newBufferId.trim(), {
      capacity: values.capacity,
      column_names: values.column_names || DEFAULT_BUFFER_CONFIG.column_names,
    });
    setNewBufferId('');
    message.success(t('buffer.created'));
  };

  return (
    <Card size="small" title={t('buffer.title')} styles={{ body: { padding: 12 } }}>
      <Form
        form={form}
        layout="vertical"
        size="small"
        initialValues={{
          capacity: DEFAULT_BUFFER_CONFIG.capacity,
          column_names: DEFAULT_BUFFER_CONFIG.column_names,
        }}
      >
        <Form.Item label={t('buffer.select')}>
          <Select
            value={currentBuffer}
            onChange={setCurrentBuffer}
            placeholder={t('buffer.selectPlaceholder')}
            allowClear
          >
            {buffers.map((id) => (
              <Select.Option key={id} value={id}>
                {id}
              </Select.Option>
            ))}
          </Select>
        </Form.Item>

        <Form.Item label={t('buffer.newId')}>
          <Space.Compact style={{ width: '100%' }}>
            <Input
              value={newBufferId}
              onChange={(e) => setNewBufferId(e.target.value)}
              placeholder={t('buffer.idPlaceholder')}
              style={{ flex: 1 }}
            />
            <Button
              type="primary"
              onClick={handleCreateBuffer}
              loading={isLoading}
              icon={<PlusOutlined />}
            >
              {t('buffer.create')}
            </Button>
          </Space.Compact>
        </Form.Item>

        <Form.Item name="capacity" label={t('buffer.capacity')}>
          <InputNumber min={100} max={100000} style={{ width: '100%' }} />
        </Form.Item>

        {currentBuffer && (
          <Button
            danger
            icon={<DeleteOutlined />}
            onClick={() => removeBuffer(currentBuffer)}
            loading={isLoading}
            block
          >
            {t('buffer.delete')}
          </Button>
        )}
      </Form>
    </Card>
  );
};

export default BufferConfigPanel;
