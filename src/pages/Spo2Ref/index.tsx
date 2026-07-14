import React, { useState, useEffect } from 'react';
import { ConfigProvider, theme, InputNumber, Button, Space, Typography, message, App } from 'antd';
import { MinusOutlined, PlusOutlined } from '@ant-design/icons';
import { emit } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import zhCN from 'antd/locale/zh_CN';
import enUS from 'antd/locale/en_US';
import i18n from 'i18next';
import { useTranslation } from 'react-i18next';
import { formatErrorMessage } from '../../utils/errorMessage';
import { useTheme } from '../../hooks';
import { gh3036Api } from '../../api/gh3036';
import '../../styles/global.css';

const { defaultAlgorithm, darkAlgorithm } = theme;
const { Text } = Typography;

const QUICK_ADJUSTMENTS = [
  { label: '-5', value: -5 },
  { label: '-2', value: -2 },
  { label: '-1', value: -1 },
  { label: '+1', value: 1 },
  { label: '+2', value: 2 },
  { label: '+5', value: 5 },
];

const DEFAULT_VALUE = 95;

function getInitialValueFromUrl(): number {
  const params = new URLSearchParams(window.location.search);
  const valueStr = params.get('value');
  if (valueStr) {
    const parsed = parseInt(valueStr, 10);
    if (!isNaN(parsed) && parsed >= 0 && parsed <= 100) {
      return parsed;
    }
  }
  return DEFAULT_VALUE;
}

const Spo2RefPage: React.FC = () => {
  const { t } = useTranslation('gh3036');
  const { isDark } = useTheme();
  const [value, setValue] = useState<number>(() => getInitialValueFromUrl());
  const [loading, setLoading] = useState(false);
  const [antdLocale, setAntdLocale] = useState(zhCN);

  useEffect(() => {
    const savedLanguage = localStorage.getItem('language') || 'zh-CN';
    if (i18n.language !== savedLanguage) {
      i18n.changeLanguage(savedLanguage);
    }
    setAntdLocale(savedLanguage === 'zh-CN' ? zhCN : enUS);
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        getCurrentWebviewWindow().close();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const handleQuickAdjust = (adjustment: number) => {
    setValue((prev) => {
      const newValue = prev + adjustment;
      return Math.max(0, Math.min(100, newValue));
    });
  };

  const handleConfirm = async () => {
    if (value < 0 || value > 100) {
      message.error(t('monitor.spo2RefRangeError'));
      return;
    }
    
    setLoading(true);
    try {
      await gh3036Api.setSpo2Ref([value]);
      await emit('spo2-ref-updated', { value });
      message.success(t('monitor.spo2RefSetSuccess'));
    } catch (err) {
      const errorMsg = formatErrorMessage(err, t('monitor.spo2RefSetFailed'));
      message.error(errorMsg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <ConfigProvider
      locale={antdLocale}
      theme={{
        algorithm: isDark ? darkAlgorithm : defaultAlgorithm,
        token: {
          colorPrimary: '#1890ff',
        },
      }}
    >
      <App>
        <div
          style={{
            padding: 16,
            height: '100vh',
            display: 'flex',
            flexDirection: 'column',
            backgroundColor: isDark ? '#141414' : '#fff',
          }}
        >
          <div style={{ marginBottom: 12 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t('monitor.spo2RefHint')}
            </Text>
          </div>
          
          <div style={{ marginBottom: 12 }}>
            <InputNumber
              min={0}
              max={100}
              value={value}
              onChange={(v) => setValue(v ?? 0)}
              style={{ width: '100%' }}
              size="large"
              addonAfter="%"
              disabled={loading}
            />
          </div>

          <div style={{ marginBottom: 8 }}>
            <Text type="secondary" style={{ fontSize: 11 }}>
              {t('monitor.quickAdjust')}
            </Text>
          </div>
          
          <Space wrap style={{ marginBottom: 16 }}>
            {QUICK_ADJUSTMENTS.map((adj) => (
              <Button
                key={adj.label}
                size="small"
                icon={adj.value < 0 ? <MinusOutlined /> : <PlusOutlined />}
                onClick={() => handleQuickAdjust(adj.value)}
                disabled={loading}
              >
                {adj.label}
              </Button>
            ))}
          </Space>

          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 'auto' }}>
            <Button
              onClick={() => getCurrentWebviewWindow().close()}
              disabled={loading}
            >
              {t('common:close')}
            </Button>
            <Button type="primary" onClick={handleConfirm} loading={loading}>
              {t('common:confirm')}
            </Button>
          </div>
        </div>
      </App>
    </ConfigProvider>
  );
};

export default Spo2RefPage;
