import React, { useEffect, useState } from 'react';
import { Card, Button, Space, Input, Modal, Typography, Alert } from 'antd';
import {
  ReloadOutlined,
  PlusOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useProtocol } from '../../hooks/useProtocol';
import ProtocolList from './ProtocolList';
import ScriptEditor from './ScriptEditor';
import BindConfig from './BindConfig';
import type { PluginInfo } from '../../api/types';
import { usePageTabsStore } from '../../stores/pageTabsStore';

const { Text } = Typography;

const ProtocolPage: React.FC = () => {
  const { t } = useTranslation('protocol');
  const {
    protocols,
    currentProtocol,
    isLoading,
    error,
    loadProtocols,
    loadProtocol,
    unloadProtocol,
    enableProtocol,
    disableProtocol,
    bindProtocol,
    unbindProtocol,
    setCurrentProtocol,
  } = useProtocol();

  const { protocolActiveTab } = usePageTabsStore();

  const [loadModalVisible, setLoadModalVisible] = useState(false);
  const [newPluginId, setNewPluginId] = useState('');
  const [newPluginPath, setNewPluginPath] = useState('');
  const [editProtocol, setEditProtocol] = useState<PluginInfo | null>(null);

  useEffect(() => {
    loadProtocols();
  }, []);

  const handleLoadProtocol = async () => {
    if (!newPluginId.trim() || !newPluginPath.trim()) {
      return;
    }

    const info = await loadProtocol(newPluginId.trim(), newPluginPath.trim());
    if (info) {
      setLoadModalVisible(false);
      setNewPluginId('');
      setNewPluginPath('');
    }
  };

  const handleEnable = async (pluginId: string) => {
    await enableProtocol(pluginId);
  };

  const handleDisable = async (pluginId: string) => {
    await disableProtocol(pluginId);
  };

  const handleUnload = async (pluginId: string) => {
    await unloadProtocol(pluginId);
  };

  const handleEdit = (pluginId: string) => {
    const protocol = protocols.find((p) => p.id === pluginId);
    if (protocol) {
      setEditProtocol(protocol);
    }
  };

  const handleBind = async (pluginId: string, deviceId: string) => {
    return bindProtocol(pluginId, deviceId);
  };

  const handleUnbind = async (pluginId: string, deviceId: string) => {
    return unbindProtocol(pluginId, deviceId);
  };

  const handleSaveScript = async (content: string) => {
    console.log('Save script:', content);
  };

  const selectedProtocol = protocols.find((p) => p.id === currentProtocol);

  const renderRightContent = () => {
    switch (protocolActiveTab) {
      case 'bind':
        return (
          <BindConfig
            protocols={protocols}
            onBind={handleBind}
            onUnbind={handleUnbind}
          />
        );
      case 'editor':
      default:
        return (
          <ScriptEditor
            protocol={editProtocol || selectedProtocol}
            onSave={handleSaveScript}
          />
        );
    }
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden', padding: 8 }}>
      {error && (
        <Alert
          message={t('common:common.error')}
          description={error}
          type="error"
          closable
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      <Card size="small" style={{ flex: '0 0 auto', marginBottom: 8 }} styles={{ body: { padding: 8 } }}>
        <Space wrap>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => setLoadModalVisible(true)}
            size="small"
          >
            {t('title.loadProtocol')}
          </Button>
          <Button
            icon={<ReloadOutlined />}
            onClick={loadProtocols}
            loading={isLoading}
            size="small"
          >
            {t('button.refreshList')}
          </Button>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t('message.totalProtocols', { total: protocols.length, enabled: protocols.filter((p) => p.state === 'Enabled').length })}
          </Text>
        </Space>
      </Card>

      <div style={{ flex: '1 1 0', display: 'flex', gap: 8, minHeight: 0, overflow: 'hidden' }}>
        <div style={{ flex: '1 1 0', minWidth: 0, overflow: 'auto' }}>
          <ProtocolList
            protocols={protocols}
            loading={isLoading}
            currentProtocol={currentProtocol}
            onSelect={setCurrentProtocol}
            onEnable={handleEnable}
            onDisable={handleDisable}
            onUnload={handleUnload}
            onEdit={handleEdit}
          />
        </div>

        <div style={{ flex: '1 1 0', minWidth: 0, overflow: 'auto' }}>
          {renderRightContent()}
        </div>
      </div>

      <Modal
        title={t('title.loadProtocol')}
        open={loadModalVisible}
        onOk={handleLoadProtocol}
        onCancel={() => setLoadModalVisible(false)}
        okText={t('common:confirm')}
        cancelText={t('common:cancel')}
        confirmLoading={isLoading}
      >
        <Space orientation="vertical" style={{ width: '100%' }} size="middle">
          <div>
            <Text>{t('label.protocolId')}</Text>
            <Input
              placeholder={t('placeholder.protocolId')}
              value={newPluginId}
              onChange={(e) => setNewPluginId(e.target.value)}
              style={{ marginTop: 4 }}
            />
          </div>
          <div>
            <Text>{t('label.scriptPath')}</Text>
            <Input
              placeholder={t('placeholder.scriptPath')}
              value={newPluginPath}
              onChange={(e) => setNewPluginPath(e.target.value)}
              style={{ marginTop: 4 }}
            />
          </div>
        </Space>
      </Modal>
    </div>
  );
};

export default ProtocolPage;
