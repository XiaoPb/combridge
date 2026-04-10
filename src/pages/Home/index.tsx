import React from 'react';
import { Card, Row, Col, Typography } from 'antd';
import {
  UsbOutlined,
  ApiOutlined,
  CodeOutlined,
  LineChartOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

const { Title, Text } = Typography;

interface ModuleCardProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  path: string;
  onClick: (path: string) => void;
}

const ModuleCard: React.FC<ModuleCardProps> = ({ icon, title, description, path, onClick }) => {
  return (
    <Card
      hoverable
      onClick={() => onClick(path)}
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        borderRadius: 12,
        transition: 'all 0.3s ease',
        cursor: 'pointer',
      }}
      styles={{
        body: {
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          padding: 24,
        },
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.transform = 'translateY(-4px)';
        e.currentTarget.style.boxShadow = '0 8px 24px rgba(0, 0, 0, 0.12)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.transform = 'translateY(0)';
        e.currentTarget.style.boxShadow = 'none';
      }}
    >
      <div
        style={{
          fontSize: 48,
          marginBottom: 16,
          color: 'var(--ant-primary-color, #1890ff)',
        }}
      >
        {icon}
      </div>
      <Title level={4} style={{ marginBottom: 8, textAlign: 'center' }}>
        {title}
      </Title>
      <Text type="secondary" style={{ textAlign: 'center', fontSize: 13 }}>
        {description}
      </Text>
    </Card>
  );
};

const HomePage: React.FC = () => {
  const navigate = useNavigate();
  const { t } = useTranslation('home');

  const modules = [
    {
      key: 'serial',
      icon: <UsbOutlined />,
      path: '/serial',
    },
    {
      key: 'ble',
      icon: <ApiOutlined />,
      path: '/ble',
    },
    {
      key: 'protocol',
      icon: <CodeOutlined />,
      path: '/protocol',
    },
    {
      key: 'waveform',
      icon: <LineChartOutlined />,
      path: '/waveform',
    },
    {
      key: 'system',
      icon: <SettingOutlined />,
      path: '/system',
    },
  ];

  const handleCardClick = (path: string) => {
    navigate(path);
  };

  return (
    <div
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        padding: 24,
        overflow: 'auto',
      }}
    >
      <div style={{ textAlign: 'center', marginBottom: 32 }}>
        <Title level={2} style={{ marginBottom: 8 }}>
          {t('title')}
        </Title>
        <Text type="secondary" style={{ fontSize: 16 }}>
          {t('subtitle')}
        </Text>
      </div>

      <Row gutter={[24, 24]} style={{ flex: 1 }}>
        {modules.map((module) => (
          <Col key={module.key} xs={24} sm={12} md={12} lg={8} xl={8}>
            <ModuleCard
              icon={module.icon}
              title={t(`modules.${module.key}.name`)}
              description={t(`modules.${module.key}.description`)}
              path={module.path}
              onClick={handleCardClick}
            />
          </Col>
        ))}
      </Row>
    </div>
  );
};

export default HomePage;
