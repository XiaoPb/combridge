import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Card, Button, Space, Input, Select, Typography, Tooltip, Tag, Collapse, message, Popconfirm } from 'antd';
import { ClearOutlined, SendOutlined, LinkOutlined, DisconnectOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AtConnectionTab as AtConnectionTabType, AtDataEntry } from '../../stores/bleStore';
import { useBleStore, formatBleTimestamp, formatBleData, parseBleData } from '../../stores/bleStore';

const { Text } = Typography;
const { Panel } = Collapse;

interface AtConnectionTabProps {
  tab: AtConnectionTabType;
}

const AtConnectionTab: React.FC<AtConnectionTabProps> = ({ tab }) => {
  const { preferences, updatePreferences, clearAtTabData, removeAtTab } = useBleStore();
  const [inputValue, setInputValue] = useState('');
  const [isSending, setIsSending] = useState(false);
  const [uuidConfig, setUuidConfig] = useState({
    txUuid: tab.txUuid || '',
    rxUuid: tab.rxUuid || '',
  });
  const dataViewRef = useRef<HTMLDivElement>(null);
  const [unlisten, setUnlisten] = useState<(() => void) | null>(null);

  useEffect(() => {
    const setupListener = async () => {
      const unlistenFn = await listen<{ deviceId: string; data: number[] }>('ble-notify', (event) => {
        if (event.payload.deviceId === tab.address) {
          const entry: AtDataEntry = {
            id: `rx-${Date.now()}`,
            timestamp: Date.now(),
            data: event.payload.data,
            direction: 'receive',
          };
          useBleStore.getState().addAtReceivedData(tab.id, entry);
        }
      });
      setUnlisten(() => unlistenFn);
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [tab.address, tab.id]);

  useEffect(() => {
    if (preferences.autoScroll && dataViewRef.current) {
      dataViewRef.current.scrollTop = dataViewRef.current.scrollHeight;
    }
  }, [tab.receivedData, tab.sentData, preferences.autoScroll]);

  const handleSend = useCallback(async () => {
    if (!inputValue.trim()) return;

    setIsSending(true);
    try {
      const data = parseBleData(inputValue, preferences.inputFormat);
      await invoke('send_at_data', {
        deviceId: tab.address,
        data,
      });

      const entry: AtDataEntry = {
        id: `tx-${Date.now()}`,
        timestamp: Date.now(),
        data,
        direction: 'send',
      };
      useBleStore.getState().addAtSentData(tab.id, entry);
      setInputValue('');
    } catch (err) {
      message.error(`发送失败: ${err}`);
    } finally {
      setIsSending(false);
    }
  }, [inputValue, preferences.inputFormat, tab.address, tab.id]);

  const handleClear = useCallback(() => {
    clearAtTabData(tab.id);
  }, [clearAtTabData, tab.id]);

  const handleDisconnect = useCallback(async () => {
    try {
      await invoke('disconnect_ble', { deviceId: tab.address });
      removeAtTab(tab.id);
      message.success('已断开连接');
    } catch (err) {
      message.error(`断开连接失败: ${err}`);
    }
  }, [tab.address, tab.id, removeAtTab]);

  const handleUuidSave = useCallback(async () => {
    try {
      await invoke('update_at_uuid_config', {
        txUuid: uuidConfig.txUuid || null,
        rxUuid: uuidConfig.rxUuid || null,
        srvUuid: null,
      });
      message.success('UUID配置已保存，下次连接生效');
    } catch (err) {
      message.error(`保存UUID配置失败: ${err}`);
    }
  }, [uuidConfig]);

  const allData = [...tab.receivedData, ...tab.sentData]
    .sort((a, b) => a.timestamp - b.timestamp);

  return (
    <div className="at-connection-tab" style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Card
        size="small"
        style={{ marginBottom: 8 }}
        bodyStyle={{ padding: '8px 12px' }}
      >
        <Space>
          <Tag color="green" icon={<LinkOutlined />}>
            已连接
          </Tag>
          <Text type="secondary">
            {tab.name || tab.address}
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            TX: {tab.txUuid ? tab.txUuid.slice(0, 8) + '...' : '默认'}
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            RX: {tab.rxUuid ? tab.rxUuid.slice(0, 8) + '...' : '默认'}
          </Text>
        </Space>
      </Card>

      <Collapse
        defaultActiveKey={[]}
        style={{ marginBottom: 8 }}
        size="small"
      >
        <Panel header="UUID配置" key="uuid">
          <Space direction="vertical" style={{ width: '100%' }}>
            <div>
              <Text style={{ width: 80, display: 'inline-block' }}>TX UUID:</Text>
              <Input
                value={uuidConfig.txUuid}
                onChange={(e) => setUuidConfig({ ...uuidConfig, txUuid: e.target.value })}
                placeholder="接收数据特征UUID（如：0000FFE1-0000-1000-8000-00805F9B34FB）"
                style={{ width: 'calc(100% - 90px)' }}
              />
            </div>
            <div>
              <Text style={{ width: 80, display: 'inline-block' }}>RX UUID:</Text>
              <Input
                value={uuidConfig.rxUuid}
                onChange={(e) => setUuidConfig({ ...uuidConfig, rxUuid: e.target.value })}
                placeholder="发送数据特征UUID（如：0000FFE2-0000-1000-8000-00805F9B34FB）"
                style={{ width: 'calc(100% - 90px)' }}
              />
            </div>
            <Button type="primary" size="small" onClick={handleUuidSave}>
              保存配置
            </Button>
          </Space>
        </Panel>
      </Collapse>

      <Card
        size="small"
        title="数据视图"
        extra={
          <Space>
            <Select
              value={preferences.displayFormat}
              onChange={(v) => updatePreferences({ displayFormat: v })}
              options={[
                { value: 'text', label: '文本' },
                { value: 'hex', label: 'HEX' },
              ]}
              style={{ width: 80 }}
              size="small"
            />
            <Tooltip title="自动滚动">
              <Button
                type={preferences.autoScroll ? 'primary' : 'default'}
                size="small"
                onClick={() => updatePreferences({ autoScroll: !preferences.autoScroll })}
              >
                滚动
              </Button>
            </Tooltip>
            <Tooltip title="清空数据">
              <Button size="small" icon={<ClearOutlined />} onClick={handleClear} />
            </Tooltip>
          </Space>
        }
        style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}
        bodyStyle={{ flex: 1, overflow: 'auto', padding: 8 }}
      >
        <div ref={dataViewRef} style={{ fontFamily: 'monospace', fontSize: 12 }}>
          {allData.length === 0 ? (
            <Text type="secondary">暂无数据</Text>
          ) : (
            allData.map((entry) => (
              <div
                key={entry.id}
                style={{
                  padding: '4px 8px',
                  marginBottom: 4,
                  backgroundColor: entry.direction === 'send' ? '#e6f7ff' : '#f6ffed',
                  borderRadius: 4,
                }}
              >
                <Space>
                  <Tag color={entry.direction === 'send' ? 'blue' : 'green'}>
                    {entry.direction === 'send' ? 'TX' : 'RX'}
                  </Tag>
                  <Text type="secondary" style={{ fontSize: 11 }}>
                    {formatBleTimestamp(entry.timestamp)}
                  </Text>
                  <Text>
                    {formatBleData(entry.data, preferences.displayFormat)}
                  </Text>
                </Space>
              </div>
            ))
          )}
        </div>
      </Card>

      <Card size="small" style={{ marginTop: 8 }}>
        <Space.Compact style={{ width: '100%' }}>
          <Select
            value={preferences.inputFormat}
            onChange={(v) => updatePreferences({ inputFormat: v })}
            options={[
              { value: 'text', label: '文本' },
              { value: 'hex', label: 'HEX' },
            ]}
            style={{ width: 80 }}
            size="small"
          />
          <Input
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onPressEnter={handleSend}
            placeholder={preferences.inputFormat === 'hex' ? '输入十六进制数据（如：48656C6C6F）' : '输入文本数据'}
            style={{ flex: 1 }}
            size="small"
          />
          <Button
            type="primary"
            icon={<SendOutlined />}
            onClick={handleSend}
            loading={isSending}
            size="small"
          >
            发送
          </Button>
        </Space.Compact>
      </Card>

      <div style={{ marginTop: 8, textAlign: 'right' }}>
        <Popconfirm
          title="确定要断开连接吗？"
          onConfirm={handleDisconnect}
          okText="确定"
          cancelText="取消"
        >
          <Button danger size="small" icon={<DisconnectOutlined />}>
            断开连接
          </Button>
        </Popconfirm>
      </div>
    </div>
  );
};

export default AtConnectionTab;
