import React, { useEffect } from 'react';
import { LineChartOutlined, FileTextOutlined } from '@ant-design/icons';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { useTranslation } from 'react-i18next';
import { useMenuVisibilityStore } from '../../stores/menuVisibilityStore';

const WaveformTitleTabs: React.FC = () => {
  const { waveformActiveTab, setWaveformActiveTab } = usePageTabsStore();
  const { menuVisibility } = useMenuVisibilityStore();
  const { t } = useTranslation('waveform');

  const tabs = [
    { key: 'realtime', label: t('tabs.realtime'), icon: <LineChartOutlined /> },
    { key: 'csvLoader', label: t('tabs.csvLoader'), icon: <FileTextOutlined /> },
  ] as const;

  const visibleTabs = tabs.filter((tab) => menuVisibility.home.waveform.tabs[tab.key]);

  useEffect(() => {
    if (visibleTabs.length > 0 && !visibleTabs.some((tab) => tab.key === waveformActiveTab)) {
      setWaveformActiveTab(visibleTabs[0].key);
    }
  }, [setWaveformActiveTab, visibleTabs, waveformActiveTab]);

  return (
    <div className="title-bar-tabs-container">
      {visibleTabs.map((tab) => {
        const isActive = tab.key === waveformActiveTab;

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => setWaveformActiveTab(tab.key)}
          >
            {tab.icon}
            <span>{tab.label}</span>
          </div>
        );
      })}
    </div>
  );
};

export default WaveformTitleTabs;
