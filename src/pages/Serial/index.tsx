import React from 'react';
import { Card, Typography } from 'antd';
import { UsbOutlined } from '@ant-design/icons';

const { Title, Paragraph } = Typography;

const SerialPage: React.FC = () => {
  return (
    <div>
      <Card>
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <UsbOutlined style={{ fontSize: '64px', color: 'var(--primary-color)' }} />
          <Title level={2} style={{ marginTop: '24px' }}>
            串口调试
          </Title>
          <Paragraph style={{ fontSize: '16px', color: 'var(--text-secondary)' }}>
            串口通信调试功能正在开发中...
          </Paragraph>
        </div>
      </Card>
    </div>
  );
};

export default SerialPage;
