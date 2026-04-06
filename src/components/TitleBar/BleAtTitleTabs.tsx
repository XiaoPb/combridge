import React from 'react';
import { Tabs, Badge } from 'antd';
import { ApiOutlined } from '@ant-design/icons';
import type { AtConnectionTab } from '../../stores/bleStore';

interface BleAtTitleTabsProps {
  tabs: AtConnectionTab[];
  activeTabId: string | null;
  onTabChange: (tabId: string) => void;
  onTabClose: (tabId: string) => void;
}

const BleAtTitleTabs: React.FC<BleAtTitleTabsProps> = ({
  tabs,
  activeTabId,
  onTabChange,
  onTabClose,
}) => {
  if (tabs.length === 0) {
    return null;
  }

  const items = tabs.map((tab) => ({
    key: tab.id,
    label: (
      <span>
        <ApiOutlined style={{ marginRight: 4, color: '#1890ff' }} />
        {tab.name || tab.address.slice(-8)}
        <Badge
          count={tab.receivedData.length}
          size="small"
          style={{ marginLeft: 8 }}
          overflowCount={99}
        />
      </span>
    ),
    closable: true,
  }));

  return (
    <div className="ble-at-title-tabs" style={{ display: 'flex', alignItems: 'center' }}>
      <Tabs
        activeKey={activeTabId || undefined}
        onChange={onTabChange}
        items={items}
        type="editable-card"
        hideAdd
        onEdit={(targetKey, action) => {
          if (action === 'remove' && typeof targetKey === 'string') {
            onTabClose(targetKey);
          }
        }}
        style={{ marginBottom: 0 }}
      />
    </div>
  );
};

export default BleAtTitleTabs;
