import React from 'react';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { UpgradeTab } from './UpgradeTab';
import { HistoryTab } from './HistoryTab';

const OtaPage: React.FC = () => {
  const activeTab = usePageTabsStore((s) => s.otaActiveTab);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: 16 }}>
      {activeTab === 'history' ? <HistoryTab /> : <UpgradeTab />}
    </div>
  );
};

export default OtaPage;
