import React from 'react';
import { Tag } from 'antd';
import { CloseOutlined } from '@ant-design/icons';
import { useBleStore, formatMacAddress } from '../../stores/bleStore';
import { useBle } from '../../hooks/useBle';

const SCAN_TAB_KEY = 'scan';

const BleTitleTabs: React.FC = () => {
  const { connections, currentDevice, setCurrentDevice } = useBleStore();
  const { disconnectDevice } = useBle();

  const connectedDevices = connections.filter((c) => c.isConnected);

  const tabs = [
    { key: SCAN_TAB_KEY, label: '扫描', isConnected: false, isClosable: false },
    ...connectedDevices.map((conn) => ({
      key: conn.address,
      label: conn.name || formatMacAddress(conn.address),
      isConnected: conn.isConnected,
      isClosable: true,
    })),
  ];

  const handleClose = async (e: React.MouseEvent, deviceId: string) => {
    e.stopPropagation();
    try {
      await disconnectDevice(deviceId);
    } catch {
      // ignore errors
    }
  };

  return (
    <div className="title-tabs-container">
      {tabs.map((tab) => {
        const isActive = tab.key === (currentDevice || SCAN_TAB_KEY);

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => setCurrentDevice(tab.key === SCAN_TAB_KEY ? null : tab.key)}
          >
            <span>{tab.label}</span>
            {tab.isConnected && (
              <Tag color="success" style={{ marginLeft: 4, fontSize: 10, padding: '0 4px' }}>
                ●
              </Tag>
            )}
            {tab.isClosable && (
              <span
                className="tab-close-btn"
                onClick={(e) => handleClose(e, tab.key)}
              >
                <CloseOutlined style={{ fontSize: 10 }} />
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default BleTitleTabs;
