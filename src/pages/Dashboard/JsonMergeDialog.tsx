import React, { useState, useEffect } from 'react';
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
  Select,
  theme,
} from 'antd';
import { useTranslation } from 'react-i18next';
import { dashboardApi } from '../../api/dashboard';
import type { JsonStructureInfo, JsonFieldInfo, ParserScriptInfo } from '../../types/dashboard';

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

interface JsonMergeDialogProps {
  open: boolean;
  onClose: () => void;
  scripts: ParserScriptInfo[];
  onMerge: (scriptName: string, content: string) => void;
}

const JsonMergeDialog: React.FC<JsonMergeDialogProps> = ({
  open,
  onClose,
  scripts,
  onMerge,
}) => {
  const { t } = useTranslation('dashboard');
  const { token } = theme.useToken();

  const [jsonInput, setJsonInput] = useState('');
  const [selectedScript, setSelectedScript] = useState<string | null>(null);
  const [structure, setStructure] = useState<JsonStructureInfo | null>(null);
  const [selectedFields, setSelectedFields] = useState<string[]>([]);
  const [mergedScript, setMergedScript] = useState<string | null>(null);
  const [step, setStep] = useState<'input' | 'select' | 'preview'>('input');
  const [loading, setLoading] = useState(false);
  const [existingFields, setExistingFields] = useState<string[]>([]);

  const editableScripts = scripts.filter((s) => !s.isBuiltIn);

  useEffect(() => {
    if (open) {
      setJsonInput('');
      setSelectedScript(null);
      setStructure(null);
      setSelectedFields([]);
      setMergedScript(null);
      setStep('input');
      setExistingFields([]);
    }
  }, [open]);

  const handleAnalyze = async () => {
    if (!jsonInput.trim() || !selectedScript) {
      message.warning(t('jsonMerge.noInput') || 'Please enter JSON data and select a script');
      return;
    }

    setLoading(true);
    try {
      const result = await dashboardApi.analyzeJsonStructure(jsonInput);
      setStructure(result);

      const scriptContent = await dashboardApi.getParserScriptContent(selectedScript);
      const existingPaths = extractExistingPaths(scriptContent);
      setExistingFields(existingPaths);

      const newNumericFields = result.fields
        .filter((f) => f.field_type === 'number' && !existingPaths.includes(f.path))
        .map((f) => f.path);
      setSelectedFields(newNumericFields);

      setStep('select');
    } catch (error) {
      message.error(t('jsonMerge.analyzeError') || 'Failed to analyze JSON');
    } finally {
      setLoading(false);
    }
  };

  const extractExistingPaths = (content: string): string[] => {
    const paths: string[] = [];
    const regex = /path\s*=\s*"([^"]+)"/g;
    let match;
    while ((match = regex.exec(content)) !== null) {
      paths.push(match[1]);
    }
    return paths;
  };

  const handleMerge = async () => {
    if (selectedFields.length === 0) {
      message.warning(t('jsonMerge.noFields') || 'Please select at least one field');
      return;
    }

    if (!selectedScript) return;

    setLoading(true);
    try {
      const script = await dashboardApi.mergeJsonToParser(
        jsonInput,
        selectedScript,
        selectedFields
      );
      setMergedScript(script);
      setStep('preview');
    } catch (error) {
      message.error(t('jsonMerge.mergeError') || 'Failed to merge fields');
    } finally {
      setLoading(false);
    }
  };

  const handleConfirm = () => {
    if (mergedScript && selectedScript) {
      onMerge(selectedScript, mergedScript);
      handleClose();
    }
  };

  const handleClose = () => {
    setJsonInput('');
    setSelectedScript(null);
    setStructure(null);
    setSelectedFields([]);
    setMergedScript(null);
    setStep('input');
    setExistingFields([]);
    onClose();
  };

  const toggleField = (path: string) => {
    setSelectedFields((prev) =>
      prev.includes(path)
        ? prev.filter((p) => p !== path)
        : [...prev, path]
    );
  };

  const getFieldStatus = (field: JsonFieldInfo): 'existing' | 'new' => {
    return existingFields.includes(field.path) ? 'existing' : 'new';
  };

  return (
    <Modal
      title={t('jsonMerge.title')}
      open={open}
      onCancel={handleClose}
      width={700}
      footer={
        step === 'input'
          ? [
              <Button key="cancel" onClick={handleClose}>
                {t('jsonMerge.cancel')}
              </Button>,
              <Button
                key="analyze"
                type="primary"
                loading={loading}
                onClick={handleAnalyze}
                disabled={!selectedScript}
              >
                {t('jsonMerge.analyze') || 'Analyze'}
              </Button>,
            ]
          : step === 'select'
          ? [
              <Button key="back" onClick={() => setStep('input')}>
                {t('jsonMerge.back') || 'Back'}
              </Button>,
              <Button
                key="merge"
                type="primary"
                loading={loading}
                onClick={handleMerge}
              >
                {t('jsonMerge.merge')}
              </Button>,
            ]
          : [
              <Button key="back" onClick={() => setStep('select')}>
                {t('jsonMerge.back') || 'Back'}
              </Button>,
              <Button key="confirm" type="primary" onClick={handleConfirm}>
                {t('jsonMerge.confirm') || 'Confirm'}
              </Button>,
            ]
      }
    >
      {step === 'input' && (
        <div>
          <Text strong>{t('jsonMerge.selectScript')}</Text>
          <Select
            value={selectedScript}
            onChange={setSelectedScript}
            options={editableScripts.map((s) => ({ label: s.name, value: s.name }))}
            placeholder={t('jsonMerge.selectScriptPlaceholder')}
            style={{ width: '100%', marginTop: 8, marginBottom: 16 }}
          />

          <Divider />

          <Text strong>{t('jsonMerge.pasteJson')}</Text>
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
          <Text strong style={{ marginBottom: 8, display: 'block' }}>
            {t('jsonMerge.detectedFields')} ({structure.fields.filter((f) => f.field_type === 'number').length} numeric fields)
          </Text>

          <List
            size="small"
            dataSource={structure.fields.filter((f) => f.field_type === 'number')}
            renderItem={(field: JsonFieldInfo) => {
              const status = getFieldStatus(field);
              const isExisting = status === 'existing';
              return (
                <List.Item>
                  <Checkbox
                    checked={isExisting || selectedFields.includes(field.path)}
                    onChange={() => toggleField(field.path)}
                    disabled={isExisting}
                  >
                    <Text code>{field.path}</Text>
                  </Checkbox>
                  <Tag color={isExisting ? 'default' : 'green'} style={{ marginLeft: 8 }}>
                    {isExisting ? t('jsonMerge.existing') : t('jsonMerge.new')}
                  </Tag>
                  {field.sample_value !== undefined && (
                    <Text type="secondary" style={{ marginLeft: 8 }}>
                      {t('jsonMerge.sample')}: {String(field.sample_value)}
                    </Text>
                  )}
                </List.Item>
              );
            }}
            style={{ maxHeight: 300, overflow: 'auto' }}
          />
        </div>
      )}

      {step === 'preview' && mergedScript && (
        <div>
          <Text strong>{t('jsonMerge.preview') || 'Merged Script Preview'}</Text>
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
            <pre style={{ margin: 0, whiteSpace: 'pre-wrap' }}>{mergedScript}</pre>
          </Paragraph>
        </div>
      )}
    </Modal>
  );
};

export default JsonMergeDialog;
