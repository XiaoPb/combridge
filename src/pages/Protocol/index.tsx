import React from 'react';
import { Card, Typography } from 'antd';
import { CodeOutlined } from '@ant-design/icons';

const { Title, Paragraph } = Typography;

const ProtocolPage: React.FC = () => {
  return (
    <div>
      <Card>
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <CodeOutlined style={{ fontSize: '64px', color: 'var(--primary-color)' }} />
          <Title level={2} style={{ marginTop: '24px' }}>
            协议配置
          </Title>
          <Paragraph style={{ fontSize: '16px', color: 'var(--text-secondary)' }}>
            通信协议配置功能正在开发中...
          </Paragraph>
        </div>
      </Card>
    </div>
  );
};

export default ProtocolPage;
