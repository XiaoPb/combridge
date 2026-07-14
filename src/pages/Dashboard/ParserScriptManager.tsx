import React, { useState, useEffect } from 'react';
import {
  Modal,
  List,
  Button,
  Input,
  Space,
  Tag,
  message,
  Popconfirm,
  Typography,
  Divider,
  theme,
} from 'antd';
import {
  EditOutlined,
  DeleteOutlined,
  PlayCircleOutlined,
  PlusOutlined,
  ImportOutlined,
  MergeCellsOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { formatErrorMessage } from '../../utils/errorMessage';
import { useDashboardStore } from '../../stores/dashboardStore';
import { dashboardApi } from '../../api/dashboard';
import type { ParserScriptInfo } from '../../types/dashboard';
import JsonImportDialog from './JsonImportDialog';
import JsonMergeDialog from './JsonMergeDialog';

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

interface ParserScriptManagerProps {
  open: boolean;
  onClose: () => void;
}

const ParserScriptManager: React.FC<ParserScriptManagerProps> = ({ open, onClose }) => {
  const { t } = useTranslation('dashboard');
  const { token } = theme.useToken();
  const { parserScripts, setParserScripts } = useDashboardStore();

  const [editingScript, setEditingScript] = useState<string | null>(null);
  const [scriptContent, setScriptContent] = useState('');
  const [testData, setTestData] = useState('');
  const [testResult, setTestResult] = useState<Record<string, number> | null>(null);
  const [showImport, setShowImport] = useState(false);
  const [showMerge, setShowMerge] = useState(false);

  useEffect(() => {
    if (open) {
      loadScripts();
    }
  }, [open]);

  const loadScripts = async () => {
    try {
      const scripts = await dashboardApi.getParserScripts();
      setParserScripts(scripts);
    } catch (error) {
      console.error('Failed to load scripts:', error);
    }
  };

  const handleEdit = async (name: string) => {
    try {
      const content = await dashboardApi.getParserScriptContent(name);
      setEditingScript(name);
      setScriptContent(content);
    } catch (error) {
      message.error(formatErrorMessage(error, t('parser.loadError')));
    }
  };

  const handleSave = async () => {
    if (!editingScript) return;

    try {
      await dashboardApi.saveParserScript(editingScript, scriptContent);
      message.success(t('parser.saveSuccess') || 'Script saved');
      setEditingScript(null);
      loadScripts();
    } catch (error) {
      message.error(formatErrorMessage(error, t('parser.saveError')));
    }
  };

  const handleDelete = async (name: string) => {
    try {
      await dashboardApi.deleteParserScript(name);
      message.success(t('parser.deleteSuccess') || 'Script deleted');
      loadScripts();
    } catch (error) {
      message.error(formatErrorMessage(error, t('parser.deleteError')));
    }
  };

  const handleTest = async () => {
    if (!editingScript || !testData) return;

    try {
      const result = await dashboardApi.executeParserScript(editingScript, testData);
      setTestResult(result);
      message.success(t('parser.testSuccess') || 'Test passed');
    } catch (error) {
      message.error(formatErrorMessage(error, t('parser.testError')));
      setTestResult(null);
    }
  };

  const handleNewScript = () => {
    setEditingScript('new_script');
    setScriptContent(`-- Custom Parser Script
local parser = {}

parser.name = "New Parser"
parser.description = "Custom parser"
parser.author = "User"
parser.version = "1.0.0"

parser.fields = {}

function parser.parse(data)
    local success, json_obj = pcall(json.decode, data)
    if not success or type(json_obj) ~= "table" then
        return nil
    end
    
    local result = {}
    -- Add your parsing logic here
    return result
end

function parser.validate(data)
    return data ~= nil and #data > 0
end

return parser
`);
  };

  const handleImportGenerated = async (name: string, content: string) => {
    try {
      await dashboardApi.saveParserScript(name, content);
      message.success(t('parser.importSuccess') || 'Script imported');
      setShowImport(false);
      loadScripts();
    } catch (error) {
      message.error(formatErrorMessage(error, t('parser.importError')));
    }
  };

  const handleMergeFields = async (name: string, content: string) => {
    try {
      await dashboardApi.saveParserScript(name, content);
      message.success(t('parser.mergeSuccess') || 'Fields merged');
      setShowMerge(false);
      loadScripts();
    } catch (error) {
      message.error(formatErrorMessage(error, t('parser.mergeError')));
    }
  };

  return (
    <>
      <Modal
        title={t('parser.scriptManager')}
        open={open}
        onCancel={onClose}
        width={900}
        footer={null}
      >
        {!editingScript ? (
          <>
            <Space style={{ marginBottom: 16 }}>
              <Button icon={<PlusOutlined />} onClick={handleNewScript}>
                {t('parser.newScript')}
              </Button>
              <Button icon={<ImportOutlined />} onClick={() => setShowImport(true)}>
                {t('parser.importJson')}
              </Button>
              <Button icon={<MergeCellsOutlined />} onClick={() => setShowMerge(true)}>
                {t('parser.mergeJson')}
              </Button>
            </Space>

            <List
              dataSource={parserScripts}
              renderItem={(script: ParserScriptInfo) => (
                <List.Item
                  actions={[
                    <Button
                      key="edit"
                      type="link"
                      icon={<EditOutlined />}
                      onClick={() => handleEdit(script.name)}
                    >
                      {t('parser.edit')}
                    </Button>,
                    <Popconfirm
                      key="delete"
                      title={t('parser.deleteConfirm')}
                      onConfirm={() => handleDelete(script.name)}
                      disabled={script.isBuiltIn}
                    >
                      <Button
                        type="link"
                        danger
                        icon={<DeleteOutlined />}
                        disabled={script.isBuiltIn}
                      >
                        {t('parser.delete')}
                      </Button>
                    </Popconfirm>,
                  ]}
                >
                  <List.Item.Meta
                    title={
                      <Space>
                        <Text strong>{script.name}</Text>
                        {script.isBuiltIn && <Tag color="blue">{t('parser.builtIn')}</Tag>}
                      </Space>
                    }
                    description={script.description}
                  />
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {t('parser.author')}: {script.author} | {t('parser.version')}: {script.version}
                  </Text>
                </List.Item>
              )}
            />
          </>
        ) : (
          <div>
            <Space style={{ marginBottom: 16 }}>
              <Button onClick={() => setEditingScript(null)}>{t('parser.back')}</Button>
              <Button type="primary" onClick={handleSave}>
                {t('parser.save')}
              </Button>
            </Space>

            <Text strong>{t('parser.scriptContent')}</Text>
            <TextArea
              value={scriptContent}
              onChange={(e) => setScriptContent(e.target.value)}
              rows={15}
              style={{ fontFamily: 'monospace', fontSize: 12, marginBottom: 16 }}
            />

            <Divider />

            <Text strong>{t('parser.test')}</Text>
            <TextArea
              value={testData}
              onChange={(e) => setTestData(e.target.value)}
              placeholder={t('parser.testPlaceholder')}
              rows={3}
              style={{ fontFamily: 'monospace', fontSize: 12, marginTop: 8 }}
            />
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              onClick={handleTest}
              style={{ marginTop: 8 }}
            >
              {t('parser.runTest')}
            </Button>

            {testResult && (
              <div style={{ marginTop: 16 }}>
                <Text strong>{t('parser.result')}</Text>
                <Paragraph
                  code
                  style={{
                    background: token.colorFillSecondary,
                    padding: 8,
                    marginTop: 8,
                  }}
                >
                  {JSON.stringify(testResult, null, 2)}
                </Paragraph>
              </div>
            )}
          </div>
        )}
      </Modal>

      <JsonImportDialog
        open={showImport}
        onClose={() => setShowImport(false)}
        onImport={handleImportGenerated}
      />

      <JsonMergeDialog
        open={showMerge}
        onClose={() => setShowMerge(false)}
        scripts={parserScripts}
        onMerge={handleMergeFields}
      />
    </>
  );
};

export default ParserScriptManager;
