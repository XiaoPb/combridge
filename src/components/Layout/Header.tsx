import React from 'react';
import { Layout, Button, Badge, Tooltip } from 'antd';
import { SettingOutlined, LinkOutlined } from '@ant-design/icons';

const { Header: AntHeader } = Layout;

interface HeaderProps {
  onSettingsClick: () => void;
}

const Header: React.FC<HeaderProps> = ({ onSettingsClick }) => {
  return (
    <AntHeader
      style={{
        padding: '0 16px',
        background: 'var(--bg-primary)',
        borderBottom: '1px solid var(--border-color)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        height: 'var(--header-height)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
        <h2 style={{ margin: 0, fontSize: '16px', fontWeight: 500 }}>
          ComBridge - 串口与蓝牙调试工具
        </h2>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
        <Tooltip title="连接状态">
          <Badge status="default" text="未连接" />
        </Tooltip>
        <Button
          type="text"
          icon={<LinkOutlined />}
          style={{ fontSize: '16px' }}
        >
          连接
        </Button>
        <Button
          type="text"
          icon={<SettingOutlined />}
          style={{ fontSize: '16px' }}
          onClick={onSettingsClick}
        >
          设置
        </Button>
      </div>
    </AntHeader>
  );
};

export default Header;
