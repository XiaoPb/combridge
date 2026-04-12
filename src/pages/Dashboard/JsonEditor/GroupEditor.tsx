import React from 'react';
import { Card, Button, Space, Input, Select, List, Empty, Tag } from 'antd';
import { PlusOutlined, DeleteOutlined, DragOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../../stores/dashboardStore';
import type { WidgetGroup, DatasetConfig } from '../../../types/dashboard';
import { DEFAULT_DATASET_CONFIG } from '../../../types/dashboard';
import DatasetEditor from './DatasetEditor';

const GroupEditor: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { jsonConfig, setJsonConfig } = useDashboardStore();

  const handleAddGroup = () => {
    const newGroup: WidgetGroup = {
      title: `Group ${(jsonConfig.groups?.length || 0) + 1}`,
      widget: '',
      datasets: [{ ...DEFAULT_DATASET_CONFIG }],
    };

    setJsonConfig({
      ...jsonConfig,
      groups: [...(jsonConfig.groups || []), newGroup],
    });
  };

  const handleRemoveGroup = (index: number) => {
    const newGroups = [...(jsonConfig.groups || [])];
    newGroups.splice(index, 1);
    setJsonConfig({
      ...jsonConfig,
      groups: newGroups,
    });
  };

  const handleUpdateGroup = (index: number, updates: Partial<WidgetGroup>) => {
    const newGroups = [...(jsonConfig.groups || [])];
    newGroups[index] = { ...newGroups[index], ...updates };
    setJsonConfig({
      ...jsonConfig,
      groups: newGroups,
    });
  };

  const handleAddDataset = (groupIndex: number) => {
    const newGroups = [...(jsonConfig.groups || [])];
    const group = newGroups[groupIndex];
    const newIndex = group.datasets.length;
    newGroups[groupIndex] = {
      ...group,
      datasets: [...group.datasets, { ...DEFAULT_DATASET_CONFIG, index: newIndex }],
    };
    setJsonConfig({
      ...jsonConfig,
      groups: newGroups,
    });
  };

  const handleUpdateDataset = (groupIndex: number, datasetIndex: number, updates: Partial<DatasetConfig>) => {
    const newGroups = [...(jsonConfig.groups || [])];
    const group = newGroups[groupIndex];
    newGroups[groupIndex] = {
      ...group,
      datasets: group.datasets.map((ds, i) =>
        i === datasetIndex ? { ...ds, ...updates } : ds
      ),
    };
    setJsonConfig({
      ...jsonConfig,
      groups: newGroups,
    });
  };

  const handleRemoveDataset = (groupIndex: number, datasetIndex: number) => {
    const newGroups = [...(jsonConfig.groups || [])];
    const group = newGroups[groupIndex];
    newGroups[groupIndex] = {
      ...group,
      datasets: group.datasets.filter((_, i) => i !== datasetIndex),
    };
    setJsonConfig({
      ...jsonConfig,
      groups: newGroups,
    });
  };

  const widgetTypeOptions = [
    { value: 'accelerometer', label: t('widgetTypes.accelerometer') || '加速度计' },
    { value: 'gyro', label: t('widgetTypes.gyro') || '陀螺仪' },
    { value: 'compass', label: t('widgetTypes.compass') || '指南针' },
    { value: '', label: t('widgetTypes.custom') || '自定义' },
  ];

  return (
    <div>
      <div style={{ marginBottom: 16, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span style={{ fontWeight: 500 }}>{t('jsonEditor.groups') || '组件组'}</span>
        <Button type="primary" icon={<PlusOutlined />} onClick={handleAddGroup} size="small">
          {t('jsonEditor.addGroup') || '添加组件组'}
        </Button>
      </div>

      {(!jsonConfig.groups || jsonConfig.groups.length === 0) ? (
        <Empty description={t('jsonEditor.noGroups') || '暂无组件组，点击上方按钮添加'} />
      ) : (
        <List
          dataSource={jsonConfig.groups}
          renderItem={(group, groupIndex) => (
            <Card
              key={groupIndex}
              size="small"
              style={{ marginBottom: 12 }}
              title={
                <Space>
                  <DragOutlined style={{ cursor: 'move', color: '#999' }} />
                  <Input
                    value={group.title}
                    onChange={(e) => handleUpdateGroup(groupIndex, { title: e.target.value })}
                    style={{ width: 150 }}
                    size="small"
                  />
                  <Select
                    value={group.widget}
                    onChange={(value) => handleUpdateGroup(groupIndex, { widget: value })}
                    options={widgetTypeOptions}
                    style={{ width: 120 }}
                    size="small"
                  />
                  <Tag color="blue">{group.datasets.length} {t('jsonEditor.datasets') || '数据集'}</Tag>
                </Space>
              }
              extra={
                <Button
                  type="text"
                  danger
                  icon={<DeleteOutlined />}
                  onClick={() => handleRemoveGroup(groupIndex)}
                  size="small"
                />
              }
            >
              <List
                dataSource={group.datasets}
                renderItem={(dataset, datasetIndex) => (
                  <DatasetEditor
                    key={datasetIndex}
                    dataset={dataset}
                    onChange={(updates) => handleUpdateDataset(groupIndex, datasetIndex, updates)}
                    onRemove={() => handleRemoveDataset(groupIndex, datasetIndex)}
                  />
                )}
              />
              <Button
                type="dashed"
                icon={<PlusOutlined />}
                onClick={() => handleAddDataset(groupIndex)}
                size="small"
                style={{ marginTop: 8, width: '100%' }}
              >
                {t('jsonEditor.addDataset') || '添加数据集'}
              </Button>
            </Card>
          )}
        />
      )}
    </div>
  );
};

export default GroupEditor;
