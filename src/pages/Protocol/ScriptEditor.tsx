import React, { useState, useEffect } from 'react';
import { Card, Button, Space, Typography, message, Tooltip, Select, Input, Alert } from 'antd';
import {
  SaveOutlined,
  UndoOutlined,
  CopyOutlined,
  FullscreenOutlined,
  FullscreenExitOutlined,
} from '@ant-design/icons';
import type { PluginInfo } from '../../api/types';

const { Text } = Typography;
const { TextArea } = Input;

const LUA_KEYWORDS = [
  'and', 'break', 'do', 'else', 'elseif', 'end', 'false', 'for', 'function',
  'if', 'in', 'local', 'nil', 'not', 'or', 'repeat', 'return', 'then',
  'true', 'until', 'while'
];

const DEFAULT_TEMPLATE = `-- 协议脚本模板
-- 必须定义以下常量:
-- PROTOCOL_NAME: 协议名称
-- PROTOCOL_VERSION: 协议版本

PROTOCOL_NAME = "MyProtocol"
PROTOCOL_VERSION = "1.0.0"
PROTOCOL_DESCRIPTION = "协议描述"
PROTOCOL_AUTHOR = "作者"

-- 可选的钩子函数:
-- on_data_received(data): 接收数据处理
-- on_data_send(data): 发送数据处理
-- on_connect(): 连接事件
-- on_disconnect(): 断开事件

function on_data_received(data)
    -- 处理接收到的数据
    -- data 是字节数组
    log("Received " .. #data .. " bytes")
    return data
end

function on_data_send(data)
    -- 处理要发送的数据
    return data
end

function on_connect()
    log("Device connected")
end

function on_disconnect()
    log("Device disconnected")
end
`;

interface ScriptEditorProps {
  protocol?: PluginInfo | null;
  onSave?: (content: string) => Promise<void>;
  readOnly?: boolean;
}

const ScriptEditor: React.FC<ScriptEditorProps> = ({
  protocol,
  onSave,
  readOnly = false,
}) => {
  const [content, setContent] = useState(DEFAULT_TEMPLATE);
  const [originalContent, setOriginalContent] = useState(DEFAULT_TEMPLATE);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [fontSize, setFontSize] = useState(14);
  const [isSaving, setIsSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);

  useEffect(() => {
    if (protocol) {
      const scriptContent = `-- ${protocol.name} v${protocol.version}
-- ${protocol.description || '无描述'}
-- 作者: ${protocol.author || '未知'}

PROTOCOL_NAME = "${protocol.name}"
PROTOCOL_VERSION = "${protocol.version}"
PROTOCOL_DESCRIPTION = "${protocol.description || ''}"
PROTOCOL_AUTHOR = "${protocol.author || ''}"

-- 已注册的钩子: ${protocol.hooks.join(', ') || '无'}
`;
      setContent(scriptContent);
      setOriginalContent(scriptContent);
    }
  }, [protocol]);

  useEffect(() => {
    setHasChanges(content !== originalContent);
  }, [content, originalContent]);

  const handleSave = async () => {
    if (!onSave) return;
    
    setIsSaving(true);
    try {
      await onSave(content);
      setOriginalContent(content);
      message.success('保存成功');
    } catch (err) {
      message.error(err instanceof Error ? err.message : '保存失败');
    } finally {
      setIsSaving(false);
    }
  };

  const handleUndo = () => {
    setContent(originalContent);
  };

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(content);
      message.success('已复制到剪贴板');
    } catch {
      message.error('复制失败');
    }
  };

  const toggleFullscreen = () => {
    setIsFullscreen(!isFullscreen);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey) {
      if (e.key === 's') {
        e.preventDefault();
        if (!readOnly && hasChanges) {
          handleSave();
        }
      } else if (e.key === 'z') {
        e.preventDefault();
        handleUndo();
      }
    }
    
    if (e.key === 'Tab') {
      e.preventDefault();
      const target = e.target as HTMLTextAreaElement;
      const start = target.selectionStart;
      const end = target.selectionEnd;
      const newContent = content.substring(0, start) + '    ' + content.substring(end);
      setContent(newContent);
      setTimeout(() => {
        target.selectionStart = target.selectionEnd = start + 4;
      }, 0);
    }
  };

  const editorStyle: React.CSSProperties = {
    fontFamily: 'Consolas, Monaco, "Courier New", monospace',
    fontSize: fontSize,
    lineHeight: 1.5,
    backgroundColor: '#1e1e1e',
    color: '#d4d4d4',
    border: 'none',
    resize: 'none',
    tabSize: 4,
  };

  const containerStyle: React.CSSProperties = isFullscreen
    ? {
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        zIndex: 1000,
        backgroundColor: '#1e1e1e',
        padding: 16,
      }
    : {};

  return (
    <Card
      title={
        <Space>
          <Text strong>脚本编辑器</Text>
          {protocol && (
            <Text type="secondary">
              {protocol.name} v{protocol.version}
            </Text>
          )}
          {hasChanges && <Text type="warning">(未保存)</Text>}
        </Space>
      }
      size="small"
      style={containerStyle}
      extra={
        <Space>
          <Select
            value={fontSize}
            onChange={setFontSize}
            size="small"
            style={{ width: 80 }}
            options={[
              { value: 12, label: '12px' },
              { value: 14, label: '14px' },
              { value: 16, label: '16px' },
              { value: 18, label: '18px' },
            ]}
          />
          <Tooltip title="复制">
            <Button size="small" icon={<CopyOutlined />} onClick={handleCopy} />
          </Tooltip>
          <Tooltip title="撤销">
            <Button
              size="small"
              icon={<UndoOutlined />}
              onClick={handleUndo}
              disabled={!hasChanges}
            />
          </Tooltip>
          <Tooltip title={isFullscreen ? '退出全屏' : '全屏'}>
            <Button
              size="small"
              icon={isFullscreen ? <FullscreenExitOutlined /> : <FullscreenOutlined />}
              onClick={toggleFullscreen}
            />
          </Tooltip>
          {!readOnly && (
            <Button
              type="primary"
              size="small"
              icon={<SaveOutlined />}
              loading={isSaving}
              disabled={!hasChanges}
              onClick={handleSave}
            >
              保存
            </Button>
          )}
        </Space>
      }
    >
      <Alert
        type="info"
        showIcon
        style={{ marginBottom: 12, fontSize: 12 }}
        title={
          <Space separator={<Text type="secondary">|</Text>}>
            <Text type="secondary">Ctrl+S 保存</Text>
            <Text type="secondary">Ctrl+Z 撤销</Text>
            <Text type="secondary">Tab 插入缩进</Text>
          </Space>
        }
      />
      <TextArea
        value={content}
        onChange={(e) => setContent(e.target.value)}
        onKeyDown={handleKeyDown}
        readOnly={readOnly}
        style={{
          ...editorStyle,
          height: isFullscreen ? 'calc(100vh - 150px)' : 400,
        }}
        placeholder="-- 在此编写 Lua 脚本..."
      />
      <div style={{ marginTop: 8, display: 'flex', justifyContent: 'space-between' }}>
        <Space>
          <Text type="secondary" style={{ fontSize: 12 }}>
            关键字: {LUA_KEYWORDS.slice(0, 5).join(', ')}...
          </Text>
        </Space>
        <Text type="secondary" style={{ fontSize: 12 }}>
          行数: {content.split('\n').length} | 字符: {content.length}
        </Text>
      </div>
    </Card>
  );
};

export default ScriptEditor;
