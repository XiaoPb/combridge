import React from 'react';
import { Select, Button, Tooltip, Tag, Space, Typography } from 'antd';
import { SettingOutlined, CodeOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import type { ParserScriptInfo } from '../../types/dashboard';

const { Text } = Typography;

interface ParserSelectorProps {
  onOpenManager: () => void;
}

const ParserSelector: React.FC<ParserSelectorProps> = ({ onOpenManager }) => {
  const { t } = useTranslation('dashboard');
  const { parserScripts, parserScript, setParserScript } = useDashboardStore();

  const handleSelectChange = (value: string | null) => {
    setParserScript(value);
  };

  const renderOption = (script: ParserScriptInfo) => (
    <Select.Option key={script.name} value={script.name}>
      <Space>
        <Text>{script.name}</Text>
        {script.isBuiltIn && (
          <Tag color="blue" style={{ marginLeft: 4 }}>
            {t('parser.builtIn')}
          </Tag>
        )}
      </Space>
    </Select.Option>
  );

  const selectedScript = parserScripts.find((s) => s.name === parserScript);

  return (
    <Space>
      <Tooltip
        title={
          selectedScript
            ? `${selectedScript.description}\n${t('parser.author')}: ${selectedScript.author}\n${t('parser.version')}: ${selectedScript.version}`
            : t('parser.selectParser')
        }
        placement="topLeft"
      >
        <Select
          value={parserScript}
          onChange={handleSelectChange}
          placeholder={t('parser.selectParser')}
          style={{ minWidth: 180 }}
          allowClear
          suffixIcon={<CodeOutlined />}
          optionLabelProp="label"
          notFoundContent={
            <Text type="secondary">{t('parser.selectParser')}</Text>
          }
        >
          {parserScripts.map(renderOption)}
        </Select>
      </Tooltip>

      <Tooltip title={t('parser.scriptManager')}>
        <Button icon={<SettingOutlined />} onClick={onOpenManager} />
      </Tooltip>
    </Space>
  );
};

export default ParserSelector;
