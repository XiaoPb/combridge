import React from 'react';
import { Collapse } from 'antd';
import { SettingOutlined, CodeOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import Gh3036ChannelConfig from '../Protocol/Gh3036ChannelConfig';
import Gh3036RpcList from '../Protocol/Gh3036RpcList';

const ConfigTab: React.FC = () => {
  const { t } = useTranslation('protocol');

  const collapseItems = [
    {
      key: 'channel',
      label: (
        <span>
          <SettingOutlined style={{ marginRight: 8 }} />
          {t('gh3036.channelConfig')}
        </span>
      ),
      children: <Gh3036ChannelConfig />,
    },
    {
      key: 'commands',
      label: (
        <span>
          <CodeOutlined style={{ marginRight: 8 }} />
          {t('gh3036.rpcCommands')}
        </span>
      ),
      children: <Gh3036RpcList />,
    },
  ];

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '8px 0' }}>
      <Collapse
        defaultActiveKey={['channel', 'commands']}
        items={collapseItems}
        size="small"
      />
    </div>
  );
};

export default ConfigTab;
