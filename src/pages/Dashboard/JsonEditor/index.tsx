import React, { useState, useEffect } from 'react';
import { Layout, Card, Tabs, Button, Space, message, Modal, Input, theme } from 'antd';
import { PlusOutlined, DeleteOutlined, SaveOutlined, FileOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../../stores/dashboardStore';
import { useTheme } from '../../../hooks';
import type { DashboardJsonConfig } from '../../../types/dashboard';
import FrameConfigEditor from './FrameConfigEditor';
import GroupEditor from './GroupEditor';
import JsonPreview from './JsonPreview';

const { Sider, Content } = Layout;

const JsonEditor: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { token } = theme.useToken();
  const { isDark } = useTheme();
  const {
    jsonConfig,
    setJsonConfig,
    jsonFiles,
    setJsonFiles,
  } = useDashboardStore();

  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [isNewModalOpen, setIsNewModalOpen] = useState(false);
  const [newFileName, setNewFileName] = useState('');

  useEffect(() => {
    loadJsonFiles();
  }, []);

  const loadJsonFiles = async () => {
    try {
      const files = await import('../../../api/dashboard').then((m) =>
        m.dashboardApi.getJsonFiles()
      );
      setJsonFiles(files);
    } catch (error) {
      console.error('Failed to load JSON files:', error);
    }
  };

  const handleNewFile = () => {
    setIsNewModalOpen(true);
    setNewFileName('');
  };

  const handleCreateFile = () => {
    if (!newFileName.trim()) {
      message.warning(t('jsonEditor.fileNameRequired') || '请输入文件名');
      return;
    }

    const fileName = newFileName.endsWith('.json')
      ? newFileName
      : `${newFileName}.json`;

    const newConfig: DashboardJsonConfig = {
      ...jsonConfig,
      title: fileName.replace('.json', ''),
    };

    setJsonConfig(newConfig);
    setSelectedFile(fileName);
    setIsNewModalOpen(false);
    message.success(t('jsonEditor.fileCreated') || '文件已创建');
  };

  const handleSaveFile = async () => {
    if (!selectedFile) {
      message.warning(t('jsonEditor.noFileSelected') || '请先选择或创建文件');
      return;
    }

    try {
      console.debug('[JsonEditor] Saving file:', selectedFile);
      console.debug('[JsonEditor] Config data:', JSON.stringify(jsonConfig, null, 2));
      
      const api = await import('../../../api/dashboard');
      await api.dashboardApi.saveJsonFile(selectedFile, jsonConfig);
      
      console.debug('[JsonEditor] Save successful');
      message.success(t('jsonEditor.saveSuccess') || '保存成功');
      loadJsonFiles();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error('[JsonEditor] Failed to save JSON file:', errorMessage);
      console.error('[JsonEditor] Error details:', error);
      message.error(`${t('jsonEditor.saveError') || '保存失败'}: ${errorMessage}`);
    }
  };

  const handleDeleteFile = async () => {
    if (!selectedFile) return;

    Modal.confirm({
      title: t('jsonEditor.deleteConfirm') || '确认删除',
      content: t('jsonEditor.deleteConfirmMessage') || '确定要删除此文件吗？',
      onOk: async () => {
        try {
          await import('../../../api/dashboard').then((m) =>
            m.dashboardApi.deleteJsonFile(selectedFile)
          );
          setSelectedFile(null);
          loadJsonFiles();
          message.success(t('jsonEditor.deleteSuccess') || '删除成功');
        } catch (error) {
          console.error('Failed to delete JSON file:', error);
          message.error(t('jsonEditor.deleteError') || '删除失败');
        }
      },
    });
  };

  const handleSelectFile = async (fileName: string) => {
    try {
      const config = await import('../../../api/dashboard').then((m) =>
        m.dashboardApi.loadJsonFile(fileName)
      );
      setJsonConfig(config);
      setSelectedFile(fileName);
    } catch (error) {
      console.error('Failed to load JSON file:', error);
      message.error(t('jsonEditor.loadError') || '加载失败');
    }
  };

  const tabItems = [
    {
      key: 'frame',
      label: t('jsonEditor.frameConfig') || '帧配置',
      children: <FrameConfigEditor />,
    },
    {
      key: 'groups',
      label: t('jsonEditor.groups') || '组件组',
      children: <GroupEditor />,
    },
    {
      key: 'preview',
      label: t('jsonEditor.preview') || '预览',
      children: <JsonPreview />,
    },
  ];

  return (
    <Layout style={{ height: '100%', background: token.colorBgContainer }}>
      <Sider
        width={200}
        theme={isDark ? 'dark' : 'light'}
        style={{ borderRight: `1px solid ${token.colorBorderSecondary}` }}
      >
        <div style={{ padding: 12 }}>
          <Space direction="vertical" style={{ width: '100%' }}>
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={handleNewFile}
              block
            >
              {t('jsonEditor.newFile') || '新建'}
            </Button>
            <Button
              icon={<SaveOutlined />}
              onClick={handleSaveFile}
              disabled={!selectedFile}
              block
            >
              {t('jsonEditor.save') || '保存'}
            </Button>
            <Button
              icon={<DeleteOutlined />}
              danger
              onClick={handleDeleteFile}
              disabled={!selectedFile}
              block
            >
              {t('jsonEditor.delete') || '删除'}
            </Button>
          </Space>
        </div>

        <div style={{ padding: '0 12px', marginTop: 12 }}>
          <div style={{ fontWeight: 500, marginBottom: 8 }}>
            {t('jsonEditor.fileList') || '文件列表'}
          </div>
          <div style={{ maxHeight: 'calc(100vh - 300px)', overflow: 'auto' }}>
            {jsonFiles.map((file) => (
              <div
                key={file}
                onClick={() => handleSelectFile(file)}
                style={{
                  padding: '8px 12px',
                  cursor: 'pointer',
                  borderRadius: 4,
                  background: selectedFile === file ? token.colorPrimaryBg : 'transparent',
                  marginBottom: 4,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                }}
              >
                <FileOutlined />
                <span style={{ fontSize: 13 }}>{file}</span>
              </div>
            ))}
            {jsonFiles.length === 0 && (
              <div style={{ color: token.colorTextQuaternary, fontSize: 12, textAlign: 'center', padding: 20 }}>
                {t('jsonEditor.noFiles') || '暂无文件'}
              </div>
            )}
          </div>
        </div>
      </Sider>

      <Content style={{ padding: 16, overflow: 'auto' }}>
        <Card
          title={
            <Space>
              <span>{jsonConfig.title || t('jsonEditor.untitled') || '未命名'}</span>
              {selectedFile && (
                <span style={{ fontSize: 12, color: token.colorTextQuaternary }}>({selectedFile})</span>
              )}
            </Space>
          }
          size="small"
          style={{ height: '100%' }}
        >
          <Tabs items={tabItems} />
        </Card>
      </Content>

      <Modal
        title={t('jsonEditor.newFile') || '新建文件'}
        open={isNewModalOpen}
        onOk={handleCreateFile}
        onCancel={() => setIsNewModalOpen(false)}
      >
        <Input
          placeholder={t('jsonEditor.fileNamePlaceholder') || '请输入文件名'}
          value={newFileName}
          onChange={(e) => setNewFileName(e.target.value)}
          suffix=".json"
        />
      </Modal>
    </Layout>
  );
};

export default JsonEditor;
