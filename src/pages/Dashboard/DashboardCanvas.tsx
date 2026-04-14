import React from 'react';
import { Card, Row, Col, Empty, Statistic, Progress, Tag } from 'antd';
import { useDashboardStore } from '../../stores/dashboardStore';
import type { WidgetGroup, DatasetConfig, WidgetConfig } from '../../types/dashboard';
import GaugeWidget from './widgets/GaugeWidget';
import TextWidget from './widgets/TextWidget';
import LedWidget from './widgets/LedWidget';
import CompassWidget from './widgets/CompassWidget';
import AccelerometerWidget from './widgets/AccelerometerWidget';

const DashboardCanvas: React.FC = () => {
  const { jsonConfig, parsedDataBuffer, selectedJsonFile } = useDashboardStore();

  const getLatestValue = (index: number): number | null => {
    if (parsedDataBuffer.length === 0) return null;
    const latest = parsedDataBuffer[parsedDataBuffer.length - 1];
    const keys = Object.keys(latest.values);
    if (index < keys.length) {
      return latest.values[keys[index]];
    }
    return null;
  };

  const datasetToWidgetConfig = (dataset: DatasetConfig, dataKey: string): WidgetConfig => ({
    id: `dataset-${dataset.index}`,
    type: dataset.widget as WidgetConfig['type'],
    title: dataset.title,
    x: 0,
    y: 0,
    width: 1,
    height: 1,
    dataKey,
    min: dataset.min,
    max: dataset.max,
    unit: dataset.units,
    color: dataset.color,
  });

  const renderDatasetWidget = (dataset: DatasetConfig, groupIndex: number, datasetIndex: number) => {
    const value = getLatestValue(dataset.index);
    const key = `${groupIndex}-${datasetIndex}`;
    const dataKey = `field_${dataset.index}`;
    const config = datasetToWidgetConfig(dataset, dataKey);

    switch (dataset.widget) {
      case 'x':
      case 'y':
      case 'z':
        return (
          <Card key={key} size="small" style={{ marginBottom: 8 }}>
            <Statistic
              title={`${dataset.title} (${dataset.widget.toUpperCase()})`}
              value={value?.toFixed(2) ?? '--'}
              suffix={dataset.units}
            />
          </Card>
        );
      case 'bar':
        return (
          <Card key={key} size="small" style={{ marginBottom: 8 }}>
            <div style={{ marginBottom: 8 }}>{dataset.title}</div>
            <Progress
              percent={((value ?? 0 - dataset.min) / (dataset.max - dataset.min)) * 100}
              format={() => `${value?.toFixed(1) ?? '--'} ${dataset.units}`}
            />
          </Card>
        );
      case 'gauge':
        return (
          <Card key={key} size="small" style={{ marginBottom: 8 }}>
            <GaugeWidget config={config} />
          </Card>
        );
      case 'text':
        return (
          <Card key={key} size="small" style={{ marginBottom: 8 }}>
            <TextWidget config={config} />
          </Card>
        );
      case 'led':
        return (
          <Card key={key} size="small" style={{ marginBottom: 8 }}>
            <LedWidget config={config} />
          </Card>
        );
      default:
        return (
          <Card key={key} size="small" style={{ marginBottom: 8 }}>
            <Statistic
              title={dataset.title}
              value={value?.toFixed(2) ?? '--'}
              suffix={dataset.units}
            />
          </Card>
        );
    }
  };

  const renderGroup = (group: WidgetGroup, groupIndex: number) => {
    const isAccelerometer = group.widget === 'accelerometer';
    const isCompass = group.widget === 'compass';

    if (isAccelerometer && group.datasets.length >= 3) {
      const config: WidgetConfig = {
        id: `accelerometer-${groupIndex}`,
        type: 'accelerometer',
        title: group.title,
        x: 0,
        y: 0,
        width: 1,
        height: 1,
        dataKey: 'accel',
        min: group.datasets[0]?.min ?? -10,
        max: group.datasets[0]?.max ?? 10,
        color: group.datasets[0]?.color,
      };
      return (
        <Card
          key={groupIndex}
          title={group.title}
          size="small"
          style={{ marginBottom: 16 }}
        >
          <AccelerometerWidget config={config} />
        </Card>
      );
    }

    if (isCompass && group.datasets.length >= 1) {
      const config: WidgetConfig = {
        id: `compass-${groupIndex}`,
        type: 'compass',
        title: group.title,
        x: 0,
        y: 0,
        width: 1,
        height: 1,
        dataKey: `field_${group.datasets[0].index}`,
        color: group.datasets[0]?.color,
      };
      return (
        <Card
          key={groupIndex}
          title={group.title}
          size="small"
          style={{ marginBottom: 16 }}
        >
          <CompassWidget config={config} />
        </Card>
      );
    }

    return (
      <Card
        key={groupIndex}
        title={group.title}
        size="small"
        style={{ marginBottom: 16 }}
      >
        <Row gutter={[8, 8]}>
          {group.datasets.map((dataset, datasetIndex) => (
            <Col key={datasetIndex} span={24 / Math.min(group.datasets.length, 4)}>
              {renderDatasetWidget(dataset, groupIndex, datasetIndex)}
            </Col>
          ))}
        </Row>
      </Card>
    );
  };

  if (!selectedJsonFile) {
    return (
      <div style={{ 
        height: '100%', 
        display: 'flex', 
        alignItems: 'center', 
        justifyContent: 'center',
        flexDirection: 'column',
        gap: 16
      }}>
        <Empty
          description="请先在设置面板中选择配置文件"
        />
        <Tag color="blue">提示：选择JSON配置文件后，仪表盘将显示配置的组件</Tag>
      </div>
    );
  }

  if (!jsonConfig.groups || jsonConfig.groups.length === 0) {
    return (
      <div style={{ 
        height: '100%', 
        display: 'flex', 
        alignItems: 'center', 
        justifyContent: 'center' 
      }}>
        <Empty
          description="配置文件中没有组件组，请在JSON编辑器中添加"
        />
      </div>
    );
  }

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: 16 }}>
      <Row gutter={[16, 16]}>
        {jsonConfig.groups.map((group, groupIndex) => (
          <Col 
            key={groupIndex} 
            xs={24} 
            sm={12} 
            md={8} 
            lg={6}
          >
            {renderGroup(group, groupIndex)}
          </Col>
        ))}
      </Row>
    </div>
  );
};

export default DashboardCanvas;
