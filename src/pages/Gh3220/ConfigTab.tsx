import React from 'react';
import { Collapse } from 'antd';
import { CodeOutlined, SettingOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import Gh3220ChannelConfig from './Gh3220ChannelConfig';
import Gh3220CommandConsole from './Gh3220CommandConsole';

/**
 * GH3220 配置页，结构对齐 GH3036：
 * ① 通道配置（链路 / GATT 特征 / MTU / CSV 偏好 / 连接动作）
 * ② UProtocol 命令收发（全量命令目录、参数、响应与收发日志）
 */
const ConfigTab: React.FC = () => {
  const { t } = useTranslation('gh3220');

  const collapseItems = [
    {
      key: 'channel',
      label: (
        <span>
          <SettingOutlined style={{ marginRight: 8 }} />
          {t('config.channelTitle')}
        </span>
      ),
      children: <Gh3220ChannelConfig />,
    },
    {
      key: 'commands',
      label: (
        <span>
          <CodeOutlined style={{ marginRight: 8 }} />
          {t('config.commandsTitle')}
        </span>
      ),
      children: <Gh3220CommandConsole />,
    },
  ];

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '8px 0' }}>
      <Collapse defaultActiveKey={['channel', 'commands']} items={collapseItems} size="small" />
    </div>
  );
};

export default ConfigTab;
