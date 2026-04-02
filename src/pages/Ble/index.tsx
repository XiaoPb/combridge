import React from 'react';
import { Card, Typography } from 'antd';
import { ApiOutlined } from '@ant-design/icons';

const { Title, Paragraph } = Typography;

const BlePage: React.FC = () => {
  return (
    <div>
      <Card>
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <ApiOutlined style={{ fontSize: '64px', color: 'var(--primary-color)' }} />
          <Title level={2} style={{ marginTop: '24px' }}>
            BLE 调试
          </Title>
          <Paragraph style={{ fontSize: '16px', color: 'var(--text-secondary)' }}>
            蓝牙低功耗调试功能正在开发中...
          </Paragraph>
        </div>
      </Card>
    </div>
  );
};

export default BlePage;
