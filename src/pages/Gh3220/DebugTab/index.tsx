import React from 'react';
import { Alert, Collapse, Space } from 'antd';
import {
  AlertOutlined,
  BugOutlined,
  ExperimentOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import CaptureSection from './CaptureSection';
import ControlSection from './ControlSection';
import EventSection from './EventSection';
import ChannelSection from './ChannelSection';

/**
 * GH3220 调试页：把 V1 的 capture/control/events/debug 四个页签
 * 合并为四个折叠分区，主界面只保留 config/monitor/version/debug 四个页签。
 */
const DebugTab: React.FC = () => {
  const { t } = useTranslation('gh3220');

  const items = [
    {
      key: 'capture',
      label: (
        <Space size={6}>
          <ThunderboltOutlined />
          {t('debug.sections.capture')}
        </Space>
      ),
      children: <CaptureSection />,
    },
    {
      key: 'control',
      label: (
        <Space size={6}>
          <ExperimentOutlined />
          {t('debug.sections.control')}
        </Space>
      ),
      children: <ControlSection />,
    },
    {
      key: 'events',
      label: (
        <Space size={6}>
          <AlertOutlined />
          {t('debug.sections.events')}
        </Space>
      ),
      children: <EventSection />,
    },
    {
      key: 'channel',
      label: (
        <Space size={6}>
          <BugOutlined />
          {t('debug.sections.channel')}
        </Space>
      ),
      children: <ChannelSection />,
    },
  ];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, paddingBottom: 8 }}>
      <Alert type="warning" showIcon message={t('debug.warning')} />
      <Collapse items={items} size="small" defaultActiveKey={['capture']} />
    </div>
  );
};

export default DebugTab;
