import React from 'react';
import { Tag } from 'antd';
import { useBleStore, formatMacAddress } from '../../stores/bleStore';

const SCAN_TAB_KEY = 'scan';

const BleTitleTabs: React.FC = () => {
  const { connections, currentDevice, setCurrentDevice, isScanning } = useBleStore();

  const connectedDevices = connections.filter((c) => c.isConnected);

  const tabs = [
    { key: SCAN_TAB_KEY, label: '扫描', isConnected: false },
    ...connectedDevices.map((conn) => ({
      key: conn.address,
      label: conn.name || formatMacAddress(conn.address),
      isConnected: conn.isConnected,
    })),
  ];

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
            {tab.key === SCAN_TAB_KEY && isScanning && (
              <Tag color="processing" style={{ marginLeft: 4, fontSize: 10, padding: '0 4px' }}>
                ●
              </Tag>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default BleTitleTabs;
