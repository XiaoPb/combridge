import React from 'react';
import { LineChartOutlined, FileTextOutlined } from '@ant-design/icons';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { useTranslation } from 'react-i18next';

const WaveformTitleTabs: React.FC = () => {
  const { waveformActiveTab, setWaveformActiveTab } = usePageTabsStore();
  const { t } = useTranslation('waveform');

  const tabs = [
    { key: 'realtime', label: t('tabs.realtime'), icon: <LineChartOutlined /> },
    { key: 'csvLoader', label: t('tabs.csvLoader'), icon: <FileTextOutlined /> },
  ] as const;

  return (
    <div className="title-bar-tabs-container">
      {tabs.map((tab) => {
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
