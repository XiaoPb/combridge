import React, { useState } from 'react';
import { Card, Button, Input, Segmented, Typography, Tag, message, Tooltip } from 'antd';
import { ReadOutlined, SendOutlined, BellOutlined, BellFilled, MenuFoldOutlined, MenuUnfoldOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { BleCharacteristic } from '../../types';
import { getShortUuid } from '../../stores/bleStore';
import { getCharacteristicName } from '../../types/ble';

const { Text } = Typography;
const { TextArea } = Input;

interface CharacteristicPanelProps {
  characteristic: BleCharacteristic | null;
  isSubscribed: boolean;
  collapsed: boolean;
  onToggleCollapse: () => void;
  inputFormat: 'hex' | 'text';
  withoutResponse: boolean;
  onInputFormatChange: (value: 'hex' | 'text') => void;
  onWithoutResponseChange: (value: boolean) => void;
  onRead: (uuid: string) => void;
  onWrite: (uuid: string, data: string, format: 'hex' | 'text', withoutResponse: boolean) => void;
  onSubscribe: (uuid: string) => void;
  onUnsubscribe: (uuid: string) => void;
}

const CharacteristicPanel: React.FC<CharacteristicPanelProps> = ({
  characteristic,
  isSubscribed,
  collapsed,
  onToggleCollapse,
  inputFormat,
  withoutResponse,
  onInputFormatChange,
  onWithoutResponseChange,
  onRead,
  onWrite,
  onSubscribe,
  onUnsubscribe,
}) => {
  const { t } = useTranslation('ble');
  const [inputData, setInputData] = useState('');

  if (!characteristic) {
    return (
      <Card 
        title={
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <Text style={{ fontSize: 13 }}>{t('title.characteristicPanel')}</Text>
            <Button
              type="text"
              size="small"
              icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
              onClick={onToggleCollapse}
            />
          </div>
        }
        size="small"
        styles={{ body: { display: collapsed ? 'none' : 'block', padding: 8 } }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 180, color: '#999' }}>
          {t('placeholder.selectCharacteristic')}
        </div>
      </Card>
    );
  }

  const { uuid, properties } = characteristic;
  const canWrite = properties.write || properties.writeWithoutResponse;
  const canRead = properties.read;
  const canNotify = properties.notify || properties.indicate;

  const handleWrite = () => {
    if (!inputData.trim()) {
      message.warning(t('placeholder.inputData'));
      return;
    }
    onWrite(uuid, inputData, inputFormat, withoutResponse);
  };

  const handleSubscribeToggle = () => {
    if (isSubscribed) {
      onUnsubscribe(uuid);
    } else {
      onSubscribe(uuid);
    }
  };

  const getWriteModeText = () => {
    if (properties.write && properties.writeWithoutResponse) {
      return t('label.writeSupport');
    } else if (properties.writeWithoutResponse) {
      return t('label.writeNoResponse');
    } else if (properties.write) {
      return t('label.writeWithResponse');
    }
    return '';
  };

  return (
    <Card 
      title={
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <Text style={{ fontSize: 13 }}>{t('title.characteristicPanel')}</Text>
          <Button
            type="text"
            size="small"
            icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
            onClick={onToggleCollapse}
          />
        </div>
      }
      size="small"
      styles={{ body: { display: collapsed ? 'none' : 'block', padding: '8px 12px', height: 180, overflow: 'hidden' } }}
    >
      <div style={{ display: 'flex', gap: 8, marginBottom: 8, flexShrink: 0 }}>
        <div style={{ flex: '1 1 0', minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
            <Text type="secondary" style={{ fontSize: 12, flexShrink: 0 }}>{t('label.uuid')}:</Text>
            <Text code style={{ fontSize: 11, wordBreak: 'break-all' }}>{uuid}</Text>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Text type="secondary" style={{ fontSize: 12, flexShrink: 0 }}>{t('label.name')}:</Text>
            <Text style={{ fontSize: 12 }}>{getCharacteristicName(uuid)} (0x{getShortUuid(uuid)})</Text>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginTop: 4 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>{getWriteModeText()}</Text>
            {properties.notify && <Tag color="orange" style={{ fontSize: 10, padding: '0 4px', margin: 0 }}>{t('characteristic.notify')}</Tag>}
            {properties.indicate && <Tag color="purple" style={{ fontSize: 10, padding: '0 4px', margin: 0 }}>{t('characteristic.indicate')}</Tag>}
          </div>
        </div>
        
        <div style={{ flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 4, alignItems: 'flex-end' }}>
          {canNotify && (
            <Tooltip title={isSubscribed ? t('tooltip.unsubscribeNotify') : t('tooltip.subscribeNotify')}>
              <Button
                size="small"
                type={isSubscribed ? 'primary' : 'default'}
                icon={isSubscribed ? <BellFilled /> : <BellOutlined />}
                onClick={handleSubscribeToggle}
              >
                {isSubscribed ? t('status.subscribed') : t('button.subscribe')}
              </Button>
            </Tooltip>
          )}
          {canRead && (
            <Tooltip title={t('tooltip.readValue')}>
              <Button
                size="small"
                icon={<ReadOutlined />}
                onClick={() => onRead(uuid)}
              >
                {t('button.read')}
              </Button>
            </Tooltip>
          )}
        </div>
      </div>

      {canWrite && (
        <div style={{ height: 'calc(100% - 80px)', display: 'flex', gap: 8, minHeight: 0 }}>
          <TextArea
            value={inputData}
            onChange={(e) => setInputData(e.target.value)}
            placeholder={inputFormat === 'hex' ? t('placeholder.inputHex') : t('placeholder.inputText')}
            style={{ 
              flex: '1 1 0', 
              resize: 'none',
              fontFamily: inputFormat === 'hex' ? 'Consolas, Monaco, monospace' : 'inherit',
              fontSize: 12,
            }}
          />
          
          <div style={{ flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 4, justifyContent: 'space-between' }}>
            <Segmented
              value={inputFormat}
              onChange={(value) => onInputFormatChange(value as 'hex' | 'text')}
              size="small"
              options={[
                { value: 'text', label: 'TEXT' },
                { value: 'hex', label: 'HEX' },
              ]}
            />
            
            {properties.writeWithoutResponse && (
              <Segmented
                value={withoutResponse ? 'noResponse' : 'withResponse'}
                onChange={(value) => onWithoutResponseChange(value === 'noResponse')}
                size="small"
                options={[
                  { value: 'withResponse', label: t('input.waitResponse') },
                  { value: 'noResponse', label: t('input.noResponse') },
                ]}
              />
            )}
            
            <Button
              type="primary"
              icon={<SendOutlined />}
              onClick={handleWrite}
              disabled={!inputData.trim()}
              block
            >
              {t('button.send')}
            </Button>
          </div>
        </div>
      )}
      
      {!canWrite && (
        <div style={{ height: 'calc(100% - 80px)', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#999', fontSize: 12 }}>
          {t('characteristic.notSupportWrite')}
        </div>
      )}
    </Card>
  );
};

export default CharacteristicPanel;
