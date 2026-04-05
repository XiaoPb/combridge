import React from 'react';
import SystemInfo from './SystemInfo';
import LogViewer from './LogViewer';
import SystemSettings from './SystemSettings';
import { usePageTabsStore } from '../../stores/pageTabsStore';

const SystemPage: React.FC = () => {
  const { systemActiveTab } = usePageTabsStore();

  const renderContent = () => {
    switch (systemActiveTab) {
      case 'logs':
        return <LogViewer />;
      case 'settings':
        return <SystemSettings />;
      case 'info':
      default:
        return <SystemInfo />;
    }
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden', padding: 8 }}>
      {renderContent()}
    </div>
  );
};

export default SystemPage;
