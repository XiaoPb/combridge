import React, { useMemo } from 'react';
import { Card, Select, Checkbox, Space, Typography, Button, InputNumber } from 'antd';
import { PlusOutlined, DeleteOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useCsvChartStore } from '../../stores/csvChartStore';

const { Text } = Typography;

interface ChartSidebarProps {
  columns: string[];
  totalRows: number;
}

const ChartSidebar: React.FC<ChartSidebarProps> = ({ columns, totalRows }) => {
  const { t } = useTranslation('waveform');

  const {
    chartGroups,
    hiddenLines,
    visiblePoints,
    addChartGroup,
    removeChartGroup,
    updateChartGroup,
    toggleLineVisibility,
    setVisiblePoints,
  } = useCsvChartStore();

  const visibleColumns = useMemo(() => {
    return columns.filter(col => !hiddenLines.includes(col));
  }, [columns, hiddenLines]);

  const handleLineVisibilityChange = (checkedValues: string[]) => {
    const newHidden = columns.filter(col => !checkedValues.includes(col));
    newHidden.forEach(col => {
      if (!hiddenLines.includes(col)) {
        toggleLineVisibility(col);
      }
    });
    hiddenLines.forEach(col => {
      if (!newHidden.includes(col)) {
        toggleLineVisibility(col);
      }
    });
  };

  const handleAddChartGroup = () => {
    const newIndex = chartGroups.length + 1;
    addChartGroup({
      name: `图表${newIndex}`,
      columns: [],
      height: 300,
    });
  };

  return (
    <div style={{ width: 280, height: '100%', overflow: 'auto' }}>
      <Space direction="vertical" style={{ width: '100%' }} size="middle">
        <Card size="small" title={t('sidebar.displaySettings')} styles={{ body: { padding: 12 } }}>
          <Space direction="vertical" style={{ width: '100%' }}>
            <Space>
              <Text>{t('sidebar.visiblePoints')}</Text>
              <InputNumber
                min={100}
                max={10000}
                value={visiblePoints}
                onChange={(v) => setVisiblePoints(v || 1000)}
                style={{ width: 100 }}
              />
            </Space>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t('sidebar.totalRows')}: {totalRows}
            </Text>
          </Space>
        </Card>

        <Card 
          size="small" 
          title={t('sidebar.chartGroups')} 
          styles={{ body: { padding: 12 } }}
          extra={
            <Button 
              type="text" 
              icon={<PlusOutlined />} 
              onClick={handleAddChartGroup}
              size="small"
            />
          }
        >
          <Space direction="vertical" style={{ width: '100%' }} size="small">
            {chartGroups.map((group) => (
              <Card
                key={group.name}
                size="small"
                styles={{ body: { padding: 8 } }}
                title={
                  <Space>
                    <Text strong style={{ fontSize: 12 }}>{group.name}</Text>
                    {chartGroups.length > 1 && (
                      <Button
                        type="text"
                        icon={<DeleteOutlined />}
                        onClick={() => removeChartGroup(group.name)}
                        size="small"
                        danger
                      />
                    )}
                  </Space>
                }
              >
                <Space direction="vertical" style={{ width: '100%' }} size="small">
                  <Select
                    mode="multiple"
                    allowClear
                    style={{ width: '100%' }}
                    placeholder={t('sidebar.selectColumns')}
                    value={group.columns}
                    onChange={(cols) => updateChartGroup(group.name, { columns: cols })}
                    options={columns.map(col => ({ label: col, value: col }))}
                    maxTagCount="responsive"
                    size="small"
                  />
                  <Space>
                    <Text style={{ fontSize: 11 }}>{t('sidebar.height')}</Text>
                    <InputNumber
                      min={150}
                      max={600}
                      value={group.height}
                      onChange={(v) => updateChartGroup(group.name, { height: v || 300 })}
                      style={{ width: 70 }}
                      size="small"
                    />
                  </Space>
                </Space>
              </Card>
            ))}
          </Space>
        </Card>

        <Card size="small" title={t('sidebar.lineVisibility')} styles={{ body: { padding: 12 } }}>
          <Checkbox.Group
            value={visibleColumns}
            onChange={handleLineVisibilityChange as (checkedValue: (string | number | boolean)[]) => void}
            style={{ width: '100%' }}
          >
            <Space direction="vertical" style={{ width: '100%' }}>
              {columns.map(col => (
                <Checkbox key={col} value={col}>
                  {col}
                </Checkbox>
              ))}
            </Space>
          </Checkbox.Group>
          {columns.length === 0 && (
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t('sidebar.noColumns')}
            </Text>
          )}
        </Card>
      </Space>
    </div>
  );
};

export default ChartSidebar;
