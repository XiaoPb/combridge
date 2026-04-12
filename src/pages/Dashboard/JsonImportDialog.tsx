import React, { useState } from 'react';
import {
  Modal,
  Input,
  Button,
  Checkbox,
  List,
  Typography,
  Space,
  Tag,
  Divider,
  message,
  theme,
} from 'antd';
import { useTranslation } from 'react-i18next';
import { dashboardApi } from '../../api/dashboard';
import type { JsonStructureInfo, JsonFieldInfo } from '../../types/dashboard';

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

interface JsonImportDialogProps {
  open: boolean;
  onClose: () => void;
  onImport: (name: string, content: string) => void;
}

const JsonImportDialog: React.FC<JsonImportDialogProps> = ({
  open,
  onClose,
  onImport,
}) => {
  const { t } = useTranslation('dashboard');
  const { token } = theme.useToken();

  const [jsonInput, setJsonInput] = useState('');
  const [structure, setStructure] = useState<JsonStructureInfo | null>(null);
  const [selectedFields, setSelectedFields] = useState<string[]>([]);
  const [scriptName, setScriptName] = useState('my_parser');
  const [generatedScript, setGeneratedScript] = useState<string | null>(null);
  const [step, setStep] = useState<'input' | 'select' | 'preview'>('input');
  const [loading, setLoading] = useState(false);

  const handleAnalyze = async () => {
    if (!jsonInput.trim()) {
      message.warning(t('jsonImport.noInput') || 'Please enter JSON data');
      return;
    }

    setLoading(true);
    try {
      const result = await dashboardApi.analyzeJsonStructure(jsonInput);
      setStructure(result);
      const numericFields = result.fields
        .filter((f) => f.field_type === 'number')
        .map((f) => f.path);
      setSelectedFields(numericFields);
      setStep('select');
    } catch (error) {
      message.error(t('jsonImport.analyzeError') || 'Failed to analyze JSON');
    } finally {
      setLoading(false);
    }
  };

  const handleGenerate = async () => {
    if (selectedFields.length === 0) {
      message.warning(t('jsonImport.noFields') || 'Please select at least one field');
      return;
    }

    setLoading(true);
    try {
      const script = await dashboardApi.generateParserFromJson(
        jsonInput,
        scriptName,
        selectedFields
      );
      setGeneratedScript(script);
      setStep('preview');
    } catch (error) {
      message.error(t('jsonImport.generateError') || 'Failed to generate script');
    } finally {
      setLoading(false);
    }
  };

  const handleConfirm = () => {
    if (generatedScript) {
      onImport(scriptName, generatedScript);
      handleClose();
    }
  };

  const handleClose = () => {
    setJsonInput('');
    setStructure(null);
    setSelectedFields([]);
    setScriptName('my_parser');
    setGeneratedScript(null);
    setStep('input');
    onClose();
  };

  const toggleField = (path: string) => {
    setSelectedFields((prev) =>
      prev.includes(path)
        ? prev.filter((p) => p !== path)
        : [...prev, path]
    );
  };

  return (
    <Modal
      title={t('jsonImport.title')}
      open={open}
      onCancel={handleClose}
      width={700}
      footer={
        step === 'input'
          ? [
              <Button key="cancel" onClick={handleClose}>
                {t('jsonImport.cancel')}
              </Button>,
              <Button key="analyze" type="primary" loading={loading} onClick={handleAnalyze}>
                {t('jsonImport.analyze') || 'Analyze'}
              </Button>,
            ]
          : step === 'select'
          ? [
              <Button key="back" onClick={() => setStep('input')}>
                {t('jsonImport.back') || 'Back'}
              </Button>,
              <Button key="generate" type="primary" loading={loading} onClick={handleGenerate}>
                {t('jsonImport.generate')}
              </Button>,
            ]
          : [
              <Button key="back" onClick={() => setStep('select')}>
                {t('jsonImport.back') || 'Back'}
              </Button>,
              <Button key="confirm" type="primary" onClick={handleConfirm}>
                {t('jsonImport.confirm') || 'Confirm'}
              </Button>,
            ]
      }
    >
      {step === 'input' && (
        <div>
          <Text strong>{t('jsonImport.pasteJson')}</Text>
          <TextArea
            value={jsonInput}
            onChange={(e) => setJsonInput(e.target.value)}
            placeholder={`{"data": {"temperature": 25.6, "humidity": 65.2}}`}
            rows={10}
            style={{ fontFamily: 'monospace', fontSize: 12, marginTop: 8 }}
          />
        </div>
      )}

      {step === 'select' && structure && (
        <div>
          <Space style={{ marginBottom: 16 }}>
            <Text strong>{t('jsonImport.scriptName')}:</Text>
            <Input
              value={scriptName}
              onChange={(e) => setScriptName(e.target.value)}
              style={{ width: 200 }}
            />
          </Space>

          <Divider />

          <Text strong style={{ marginBottom: 8, display: 'block' }}>
            {t('jsonImport.detectedFields')} ({structure.fields.length} fields, {structure.fields.filter(f => f.field_type === 'number').length} numeric)
          </Text>

          <List
            size="small"
            dataSource={structure.fields.filter((f) => f.field_type === 'number')}
            renderItem={(field: JsonFieldInfo) => (
              <List.Item>
                <Checkbox
                  checked={selectedFields.includes(field.path)}
                  onChange={() => toggleField(field.path)}
                >
                  <Text code>{field.path}</Text>
                </Checkbox>
                <Tag color="blue" style={{ marginLeft: 8 }}>
                  {field.field_type}
                </Tag>
                {field.sample_value !== undefined && (
                  <Text type="secondary" style={{ marginLeft: 8 }}>
                    {t('jsonImport.sample')}: {String(field.sample_value)}
                  </Text>
                )}
              </List.Item>
            )}
            style={{ maxHeight: 300, overflow: 'auto' }}
          />
        </div>
      )}

      {step === 'preview' && generatedScript && (
        <div>
          <Text strong>{t('jsonImport.preview') || 'Generated Script Preview'}</Text>
          <Paragraph
            code
            style={{
              background: token.colorFillSecondary,
              padding: 12,
              marginTop: 8,
              maxHeight: 400,
              overflow: 'auto',
              fontSize: 12,
            }}
          >
            <pre style={{ margin: 0, whiteSpace: 'pre-wrap' }}>{generatedScript}</pre>
          </Paragraph>
        </div>
      )}
    </Modal>
  );
};

export default JsonImportDialog;
