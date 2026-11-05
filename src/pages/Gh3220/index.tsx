import React, { useEffect } from 'react';
import { Tabs, Tag } from 'antd';
import { useTranslation } from 'react-i18next';
import { useGh3220Store } from '../../stores/gh3220Store';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import ConnectTab from './ConnectTab';
import ControlTab from './ControlTab';
import CaptureTab from './CaptureTab';
import EventTab from './EventTab';
import DebugTab from './DebugTab';

const Gh3220Page: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const { gh3220ActiveTab, setGh3220ActiveTab } = usePageTabsStore();
  const { status, subscribeEvents, unsubscribeEvents, refreshStatus, loadPreferences } =
    useGh3220Store();

  useEffect(() => {
    void refreshStatus();
    void loadPreferences();
    void subscribeEvents();

    return () => {
      unsubscribeEvents();
    };
  }, [refreshStatus, loadPreferences, subscribeEvents, unsubscribeEvents]);

  const items = [
    {
      key: 'connect',
      label: t('tabs.connect'),
      children: <ConnectTab />,
    },
    {
      key: 'control',
      label: t('tabs.control'),
      children: <ControlTab />,
    },
    {
      key: 'capture',
      label: t('tabs.capture'),
      children: <CaptureTab />,
    },
    {
      key: 'events',
      label: t('tabs.events'),
      children: <EventTab />,
    },
    ...(import.meta.env.DEV
      ? [
          {
            key: 'debug',
            label: t('tabs.debug'),
            children: <DebugTab />,
          },
        ]
      : []),
  ];

  return (
    <div style={{ padding: 16, height: '100%', overflow: 'auto' }}>
      <Tabs
        activeKey={gh3220ActiveTab}
        onChange={(key) => setGh3220ActiveTab(key as typeof gh3220ActiveTab)}
        items={items}
        tabBarExtraContent={
          <Tag color={status.state === 'disconnected' ? 'default' : 'success'}>
            {t(`state.${status.state}`)}
          </Tag>
        }
      />
    </div>
  );
};

export default Gh3220Page;
