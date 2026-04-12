import React from 'react';
import { Form, Input, InputNumber, Select, Switch, Card } from 'antd';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../../stores/dashboardStore';

const FrameConfigEditor: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { jsonConfig, setJsonConfig } = useDashboardStore();

  const handleFieldChange = (field: string, value: unknown) => {
    setJsonConfig({
      ...jsonConfig,
      [field]: value,
    });
  };

  return (
    <div style={{ maxWidth: 800 }}>
      <Card title={t('jsonEditor.basicConfig') || '基本配置'} size="small" style={{ marginBottom: 16 }}>
        <Form layout="vertical" size="small">
          <Form.Item label={t('jsonEditor.title') || '仪表盘标题'}>
            <Input
              value={jsonConfig.title}
              onChange={(e) => handleFieldChange('title', e.target.value)}
              placeholder={t('jsonEditor.titlePlaceholder') || '请输入标题'}
            />
          </Form.Item>
        </Form>
      </Card>

      <Card title={t('jsonEditor.frameConfig') || '帧配置'} size="small" style={{ marginBottom: 16 }}>
        <Form layout="vertical" size="small">
          <Form.Item label={t('jsonEditor.decoder') || '解码器类型'}>
            <Select
              value={jsonConfig.decoder}
              onChange={(value) => handleFieldChange('decoder', value)}
              options={[
                { value: 0, label: t('jsonEditor.decoderRaw') || '原始数据' },
                { value: 1, label: t('jsonEditor.decoderCustom') || '自定义解析' },
              ]}
            />
          </Form.Item>

          <Form.Item label={t('jsonEditor.frameDetection') || '帧检测模式'}>
            <Select
              value={jsonConfig.frameDetection}
              onChange={(value) => handleFieldChange('frameDetection', value)}
              options={[
                { value: 0, label: t('jsonEditor.frameDetectionNone') || '无帧检测' },
                { value: 1, label: t('jsonEditor.frameDetectionMarker') || '帧标志检测' },
              ]}
            />
          </Form.Item>

          {jsonConfig.frameDetection === 1 && (
            <>
              <Form.Item label={t('jsonEditor.frameStart') || '帧起始标志'}>
                <Input
                  value={jsonConfig.frameStart}
                  onChange={(e) => handleFieldChange('frameStart', e.target.value)}
                  placeholder="$"
                  style={{ width: 100 }}
                />
              </Form.Item>

              <Form.Item label={t('jsonEditor.frameEnd') || '帧结束标志'}>
                <Input
                  value={jsonConfig.frameEnd}
                  onChange={(e) => handleFieldChange('frameEnd', e.target.value)}
                  placeholder=";"
                  style={{ width: 100 }}
                />
              </Form.Item>
            </>
          )}
        </Form>
      </Card>

      <Card title={t('jsonEditor.parserFunction') || '解析函数'} size="small">
        <Form layout="vertical" size="small">
          <Form.Item 
            label={
              <span style={{ color: '#666', fontSize: 12 }}>
                {t('jsonEditor.parserFunctionHint') || 'JavaScript函数，接收帧数据字符串，返回解析后的数组'}
              </span>
            }
          >
            <Input.TextArea
              value={jsonConfig.frameParser}
              onChange={(e) => handleFieldChange('frameParser', e.target.value)}
              rows={12}
              style={{ fontFamily: 'Consolas, Monaco, monospace', fontSize: 12 }}
              placeholder={`function parse(frame) {
  // 解析帧数据
  return frame.split(',');
}`}
            />
          </Form.Item>
        </Form>
      </Card>
    </div>
  );
};

export default FrameConfigEditor;
