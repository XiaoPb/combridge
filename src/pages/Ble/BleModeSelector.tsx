import React from 'react';
import { Card, Radio, Select, Space, Typography, Alert } from 'antd';
import { ApiOutlined, UsbOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { BleMode } from '../../stores/bleStore';

const { Text } = Typography;

interface BleModeSelectorProps {
  mode: BleMode;
  serialPort: string | null;
  ports: { name: string }[];
  onModeChange: (mode: BleMode) => void;
  onSerialPortChange: (port: string) => void;
}

const BleModeSelector: React.FC<BleModeSelectorProps> = ({
  mode,
  serialPort,
  ports,
  onModeChange,
  onSerialPortChange,
}) => {
  const { t } = useTranslation('ble');

  return (
    <Card title={t('title.bleModeConfig')} size="small">
      <Space vertical style={{ width: '100%' }}>
        <div>
          <Text type="secondary">{t('label.selectBleMode')}</Text>
          <Radio.Group
            value={mode}
            onChange={(e) => onModeChange(e.target.value)}
            style={{ marginTop: 8 }}
          >
            <Radio.Button value="native">
              <Space>
                <ApiOutlined />
                {t('mode.native')}
              </Space>
            </Radio.Button>
            <Radio.Button value="at">
              <Space>
                <UsbOutlined />
                {t('mode.at')}
              </Space>
            </Radio.Button>
          </Radio.Group>
        </div>

        {mode === 'native' && (
          <Alert
            title={t('mode.native')}
            description={t('mode.nativeDesc')}
            type="info"
            showIcon
          />
        )}

        {mode === 'at' && (
          <>
            <div>
              <Text type="secondary">{t('label.selectSerialPort')}</Text>
              <Select
                value={serialPort}
                onChange={onSerialPortChange}
                placeholder={t('placeholder.selectPort')}
                style={{ width: '100%', marginTop: 8 }}
                options={ports.map((p) => ({
                  label: p.name,
                  value: p.name,
                }))}
              />
            </div>
            <Alert
              title={t('mode.at')}
              description={t('mode.atDesc')}
              type="info"
              showIcon
            />
          </>
        )}
      </Space>
    </Card>
  );
};

export default BleModeSelector;
