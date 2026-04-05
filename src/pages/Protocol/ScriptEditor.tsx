import React, { useState, useEffect } from 'react';
import { Card, Button, Space, Typography, message, Tooltip, Select, Input, Alert } from 'antd';
import {
  SaveOutlined,
  UndoOutlined,
  CopyOutlined,
  FullscreenOutlined,
  FullscreenExitOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { PluginInfo } from '../../api/types';

const { Text } = Typography;
const { TextArea } = Input;

const LUA_KEYWORDS = [
  'and', 'break', 'do', 'else', 'elseif', 'end', 'false', 'for', 'function',
  'if', 'in', 'local', 'nil', 'not', 'or', 'repeat', 'return', 'then',
  'true', 'until', 'while'
];

const ScriptEditor: React.FC<ScriptEditorProps> = ({
  protocol,
  onSave,
  readOnly = false,
}) => {
  const { t } = useTranslation('protocol');

  const getDefaultTemplate = () => `-- ${t('editor.templateTitle')}
-- ${t('editor.mustDefine')}:
-- PROTOCOL_NAME: ${t('editor.protocolName')}
-- PROTOCOL_VERSION: ${t('editor.protocolVersion')}

PROTOCOL_NAME = "MyProtocol"
PROTOCOL_VERSION = "1.0.0"
PROTOCOL_DESCRIPTION = "${t('editor.protocolDesc')}"
PROTOCOL_AUTHOR = "${t('editor.author')}"

-- ${t('editor.optionalHooks')}:
-- on_data_received(data): ${t('editor.hookOnDataReceived')}
-- on_data_send(data): ${t('editor.hookOnDataSend')}
-- on_connect(): ${t('editor.hookOnConnect')}
-- on_disconnect(): ${t('editor.hookOnDisconnect')}

function on_data_received(data)
    -- ${t('editor.processReceivedData')}
    -- data ${t('editor.isByteArray')}
    log("Received " .. #data .. " bytes")
    return data
end

function on_data_send(data)
    -- ${t('editor.processSendData')}
    return data
end

function on_connect()
    log("Device connected")
end

function on_disconnect()
    log("Device disconnected")
end
`;

  const [content, setContent] = useState(getDefaultTemplate());
  const [originalContent, setOriginalContent] = useState(getDefaultTemplate());
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [fontSize, setFontSize] = useState(14);
  const [isSaving, setIsSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);

  useEffect(() => {
    if (protocol) {
      const scriptContent = `-- ${protocol.name} v${protocol.version}
-- ${protocol.description || t('editor.noDescription')}
-- ${t('editor.author')}: ${protocol.author || t('editor.unknown')}

PROTOCOL_NAME = "${protocol.name}"
PROTOCOL_VERSION = "${protocol.version}"
PROTOCOL_DESCRIPTION = "${protocol.description || ''}"
PROTOCOL_AUTHOR = "${protocol.author || ''}"

-- ${t('editor.registeredHooks')}: ${protocol.hooks.join(', ') || t('editor.none')}
`;
      setContent(scriptContent);
      setOriginalContent(scriptContent);
    }
  }, [protocol, t]);

  useEffect(() => {
    setHasChanges(content !== originalContent);
  }, [content, originalContent]);

  const handleSave = async () => {
    if (!onSave) return;
    
    setIsSaving(true);
    try {
      await onSave(content);
      setOriginalContent(content);
      message.success(t('message.saveSuccess'));
    } catch (err) {
      message.error(err instanceof Error ? err.message : t('message.saveFailed'));
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
      message.success(t('message.copySuccess'));
    } catch {
      message.error(t('message.copyFailed'));
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
          <Text strong>{t('editor.title')}</Text>
          {protocol && (
            <Text type="secondary">
              {protocol.name} v{protocol.version}
            </Text>
          )}
          {hasChanges && <Text type="warning">({t('editor.unsaved')})</Text>}
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
          <Tooltip title={t('tooltip.copy')}>
            <Button size="small" icon={<CopyOutlined />} onClick={handleCopy} />
          </Tooltip>
          <Tooltip title={t('tooltip.undo')}>
            <Button
              size="small"
              icon={<UndoOutlined />}
              onClick={handleUndo}
              disabled={!hasChanges}
            />
          </Tooltip>
          <Tooltip title={isFullscreen ? t('tooltip.exitFullscreen') : t('tooltip.fullscreen')}>
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
              {t('editor.save')}
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
            <Text type="secondary">Ctrl+S {t('editor.save')}</Text>
            <Text type="secondary">Ctrl+Z {t('tooltip.undo')}</Text>
            <Text type="secondary">Tab {t('editor.insertIndent')}</Text>
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
        placeholder={`-- ${t('editor.writeLuaScript')}...`}
      />
      <div style={{ marginTop: 8, display: 'flex', justifyContent: 'space-between' }}>
        <Space>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t('editor.keywords')}: {LUA_KEYWORDS.slice(0, 5).join(', ')}...
          </Text>
        </Space>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {t('editor.lines')}: {content.split('\n').length} | {t('editor.chars')}: {content.length}
        </Text>
      </div>
    </Card>
  );
};

interface ScriptEditorProps {
  protocol?: PluginInfo | null;
  onSave?: (content: string) => Promise<void>;
  readOnly?: boolean;
}

export default ScriptEditor;
