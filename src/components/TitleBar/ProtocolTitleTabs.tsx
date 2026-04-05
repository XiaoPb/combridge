import React from 'react';
import { CodeOutlined, FolderOpenOutlined, ApiOutlined } from '@ant-design/icons';
import { usePageTabsStore } from '../../stores/pageTabsStore';
import { useTranslation } from 'react-i18next';

const ProtocolTitleTabs: React.FC = () => {
  const { protocolActiveTab, setProtocolActiveTab } = usePageTabsStore();
  const { t } = useTranslation('protocol');

  const tabs = [
    { key: 'editor', label: t('title.scriptEditor'), icon: <CodeOutlined /> },
    { key: 'bind', label: t('title.bindConfig'), icon: <FolderOpenOutlined /> },
    { key: 'gh3036', label: t('title.gh3036'), icon: <ApiOutlined /> },
  ] as const;

  return (
    <div className="title-tabs-container">
      {tabs.map((tab) => {
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
