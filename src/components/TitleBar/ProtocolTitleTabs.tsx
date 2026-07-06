import React, { useEffect } from 'react';
import { CodeOutlined, FolderOpenOutlined } from '@ant-design/icons';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { useTranslation } from 'react-i18next';
import { useMenuVisibilityStore } from '../../stores/menuVisibilityStore';

const ProtocolTitleTabs: React.FC = () => {
  const { protocolActiveTab, setProtocolActiveTab } = usePageTabsStore();
  const { menuVisibility } = useMenuVisibilityStore();
  const { t } = useTranslation('protocol');

  const tabs = [
    { key: 'editor', label: t('title.scriptEditor'), icon: <CodeOutlined /> },
    { key: 'bind', label: t('title.bindConfig'), icon: <FolderOpenOutlined /> },
  ] as const;

  const visibleTabs = tabs.filter((tab) => menuVisibility.home.protocol.tabs[tab.key]);

  useEffect(() => {
    if (visibleTabs.length > 0 && !visibleTabs.some((tab) => tab.key === protocolActiveTab)) {
      setProtocolActiveTab(visibleTabs[0].key);
    }
  }, [protocolActiveTab, setProtocolActiveTab, visibleTabs]);

  return (
    <div className="title-tabs-container">
      {visibleTabs.map((tab) => {
        const isActive = tab.key === protocolActiveTab;

        return (
          <div
            key={tab.key}
            className={`title-bar-tab ${isActive ? 'active' : ''}`}
            onClick={() => setProtocolActiveTab(tab.key)}
          >
            {tab.icon}
            <span>{tab.label}</span>
          </div>
        );
      })}
    </div>
  );
};

export default ProtocolTitleTabs;
