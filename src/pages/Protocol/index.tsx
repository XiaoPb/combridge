import React, { useEffect, useState } from 'react';
import { Card, Button, Space, Input, Modal, Typography, Alert, Tabs } from 'antd';
import {
  FolderOpenOutlined,
  ReloadOutlined,
  PlusOutlined,
  CodeOutlined,
} from '@ant-design/icons';
import { useProtocol } from '../../hooks/useProtocol';
import ProtocolList from './ProtocolList';
import ScriptEditor from './ScriptEditor';
import BindConfig from './BindConfig';
import type { PluginInfo } from '../../api/types';

const { Text } = Typography;

const ProtocolPage: React.FC = () => {
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

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {error && (
        <Alert
          title="错误"
          description={error}
          type="error"
          closable
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      <Card size="small" style={{ flex: '0 0 auto', marginBottom: 8, padding: 8 }} styles={{ body: { padding: 8 } }}>
        <Space wrap>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => setLoadModalVisible(true)}
          >
            加载协议
          </Button>
          <Button
            icon={<ReloadOutlined />}
            onClick={loadProtocols}
            loading={isLoading}
          >
            刷新列表
          </Button>
          <Text type="secondary">
            共 {protocols.length} 个协议，
            {protocols.filter((p) => p.state === 'Enabled').length} 个已启用
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
          <Tabs
            defaultActiveKey="editor"
            items={[
              {
                key: 'editor',
                label: (
                  <Space>
                    <CodeOutlined />
                    脚本编辑
                  </Space>
                ),
                children: (
                  <ScriptEditor
                    protocol={editProtocol || selectedProtocol}
                    onSave={handleSaveScript}
                  />
                ),
              },
              {
                key: 'bind',
                label: (
                  <Space>
                    <FolderOpenOutlined />
                    绑定配置
                  </Space>
                ),
                children: (
                  <BindConfig
                    protocols={protocols}
                    onBind={handleBind}
                    onUnbind={handleUnbind}
                  />
                ),
              },
            ]}
          />
        </div>
      </div>

      <Modal
        title="加载协议"
        open={loadModalVisible}
        onOk={handleLoadProtocol}
        onCancel={() => setLoadModalVisible(false)}
        okText="加载"
        cancelText="取消"
        confirmLoading={isLoading}
      >
        <Space vertical style={{ width: '100%' }} size="middle">
          <div>
            <Text>协议 ID</Text>
            <Input
              placeholder="输入协议唯一标识，如: my-protocol"
              value={newPluginId}
              onChange={(e) => setNewPluginId(e.target.value)}
              style={{ marginTop: 4 }}
            />
          </div>
          <div>
            <Text>脚本路径</Text>
            <Input
              placeholder="输入 Lua 脚本文件路径"
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
