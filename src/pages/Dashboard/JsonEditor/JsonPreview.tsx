import React from 'react';
import { Button, Space, message } from 'antd';
import { CopyOutlined, DownloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { formatErrorMessage } from '../../../utils/errorMessage';
import { useDashboardStore } from '../../../stores/dashboardStore';

const terminalBg = '#1e1e1e';
const terminalText = '#d4d4d4';

const JsonPreview: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { jsonConfig } = useDashboardStore();

  const jsonString = JSON.stringify(jsonConfig, null, 2);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(jsonString);
      message.success(t('jsonEditor.copySuccess') || '已复制到剪贴板');
    } catch (error) {
      console.error('Failed to copy:', error);
      message.error(formatErrorMessage(error, t('jsonEditor.copyError')));
    }
  };

  const handleDownload = () => {
    const blob = new Blob([jsonString], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${jsonConfig.title || 'dashboard'}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div>
      <div style={{ marginBottom: 12, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span style={{ fontWeight: 500 }}>{t('jsonEditor.jsonPreview') || 'JSON预览'}</span>
        <Space>
          <Button icon={<CopyOutlined />} onClick={handleCopy} size="small">
            {t('jsonEditor.copy') || '复制'}
          </Button>
          <Button icon={<DownloadOutlined />} onClick={handleDownload} size="small">
            {t('jsonEditor.download') || '下载'}
          </Button>
        </Space>
      </div>

      <pre
        style={{
          background: terminalBg,
          color: terminalText,
          padding: 16,
          borderRadius: 4,
          overflow: 'auto',
          maxHeight: 'calc(100vh - 350px)',
          fontSize: 12,
          fontFamily: 'Consolas, Monaco, monospace',
        }}
      >
        {jsonString}
      </pre>
    </div>
  );
};

export default JsonPreview;
