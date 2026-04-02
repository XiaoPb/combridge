import React from 'react';
import { Card, Typography } from 'antd';
import { SettingOutlined } from '@ant-design/icons';

const { Title, Paragraph } = Typography;

const SystemPage: React.FC = () => {
  return (
    <div>
      <Card>
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <SettingOutlined style={{ fontSize: '64px', color: 'var(--primary-color)' }} />
          <Title level={2} style={{ marginTop: '24px' }}>
            系统设置
          </Title>
          <Paragraph style={{ fontSize: '16px', color: 'var(--text-secondary)' }}>
            系统配置功能正在开发中...
          </Paragraph>
        </div>
      </Card>
    </div>
  );
};

export default SystemPage;
