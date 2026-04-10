import React from 'react';
import { Card, Row, Col, Typography, Button, Space } from 'antd';
import {
  UsbOutlined,
  ApiOutlined,
  CodeOutlined,
  LineChartOutlined,
  SettingOutlined,
  LineChartOutlined as RealtimeIcon,
  FileTextOutlined,
  CodeOutlined as EditorIcon,
  FolderOpenOutlined,
  ApiOutlined as Gh3036Icon,
  InfoCircleOutlined,
  FileTextOutlined as LogsIcon,
  SettingOutlined as SettingsIcon,
  RightOutlined,
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { usePageTabsStore } from '../../stores/pageTabsStore';

const { Title, Text } = Typography;

interface SubTab {
  key: string;
  label: string;
  icon: React.ReactNode;
}

interface ModuleCardProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  path: string;
  subTabs?: SubTab[];
  onTabClick?: (path: string, tabKey: string) => void;
  onCardClick: (path: string) => void;
}

const ModuleCard: React.FC<ModuleCardProps> = ({
  icon,
  title,
  description,
  path,
  subTabs,
  onTabClick,
  onCardClick,
}) => {
  return (
    <Card
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        borderRadius: 12,
        transition: 'all 0.3s ease',
      }}
      styles={{
        body: {
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          padding: 20,
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
          display: 'flex',
          alignItems: 'center',
          marginBottom: 12,
          cursor: 'pointer',
        }}
        onClick={() => onCardClick(path)}
      >
        <div
          style={{
            fontSize: 32,
            marginRight: 12,
            color: 'var(--ant-primary-color, #1890ff)',
          }}
        >
          {icon}
        </div>
        <div style={{ flex: 1 }}>
          <Title level={4} style={{ margin: 0, marginBottom: 4 }}>
            {title}
          </Title>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {description}
          </Text>
        </div>
        <RightOutlined style={{ color: 'var(--text-secondary)' }} />
      </div>

      {subTabs && subTabs.length > 0 && (
        <div style={{ marginTop: 'auto' }}>
          <div
            style={{
              borderTop: '1px solid var(--border-color)',
              paddingTop: 12,
              marginTop: 12,
            }}
          >
            <Space direction="vertical" style={{ width: '100%' }} size={8}>
              {subTabs.map((tab) => (
                <Button
                  key={tab.key}
                  type="text"
                  icon={tab.icon}
                  onClick={() => onTabClick?.(path, tab.key)}
                  style={{
                    width: '100%',
                    justifyContent: 'flex-start',
                    height: 36,
                    borderRadius: 8,
                    transition: 'all 0.2s',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.backgroundColor = 'var(--hover-bg)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.backgroundColor = 'transparent';
                  }}
                >
                  {tab.label}
                </Button>
              ))}
            </Space>
          </div>
        </div>
      )}
    </Card>
  );
};

const HomePage: React.FC = () => {
  const navigate = useNavigate();
  const { t } = useTranslation('home');
  const { setWaveformActiveTab, setProtocolActiveTab, setSystemActiveTab } = usePageTabsStore();

  const modules = [
    {
      key: 'serial',
      icon: <UsbOutlined />,
      path: '/serial',
      subTabs: [],
    },
    {
      key: 'ble',
      icon: <ApiOutlined />,
      path: '/ble',
      subTabs: [],
    },
    {
      key: 'protocol',
      icon: <CodeOutlined />,
      path: '/protocol',
      subTabs: [
        { key: 'editor', label: t('modules.protocol.tabs.editor'), icon: <EditorIcon /> },
        { key: 'bind', label: t('modules.protocol.tabs.bind'), icon: <FolderOpenOutlined /> },
        { key: 'gh3036', label: t('modules.protocol.tabs.gh3036'), icon: <Gh3036Icon /> },
      ],
    },
    {
      key: 'waveform',
      icon: <LineChartOutlined />,
      path: '/waveform',
      subTabs: [
        { key: 'realtime', label: t('modules.waveform.tabs.realtime'), icon: <RealtimeIcon /> },
        { key: 'csvLoader', label: t('modules.waveform.tabs.csvLoader'), icon: <FileTextOutlined /> },
      ],
    },
    {
      key: 'system',
      icon: <SettingOutlined />,
      path: '/system',
      subTabs: [
        { key: 'info', label: t('modules.system.tabs.info'), icon: <InfoCircleOutlined /> },
        { key: 'logs', label: t('modules.system.tabs.logs'), icon: <LogsIcon /> },
        { key: 'settings', label: t('modules.system.tabs.settings'), icon: <SettingsIcon /> },
      ],
    },
  ];

  const handleCardClick = (path: string) => {
    navigate(path);
  };

  const handleTabClick = (path: string, tabKey: string) => {
    switch (path) {
      case '/waveform':
        setWaveformActiveTab(tabKey as 'realtime' | 'csvLoader');
        break;
      case '/protocol':
        setProtocolActiveTab(tabKey as 'editor' | 'bind' | 'gh3036');
        break;
      case '/system':
        setSystemActiveTab(tabKey as 'info' | 'logs' | 'settings');
        break;
    }
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
              subTabs={module.subTabs}
              onTabClick={handleTabClick}
              onCardClick={handleCardClick}
            />
          </Col>
        ))}
      </Row>
    </div>
  );
};

export default HomePage;
