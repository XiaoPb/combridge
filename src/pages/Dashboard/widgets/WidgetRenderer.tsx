import React from 'react';
import { Card, theme } from 'antd';
import type { WidgetConfig } from '../../../types/dashboard';
import LineChartWidget from './LineChartWidget';
import GaugeWidget from './GaugeWidget';
import TextWidget from './TextWidget';
import LedWidget from './LedWidget';
import CompassWidget from './CompassWidget';
import AccelerometerWidget from './AccelerometerWidget';

interface WidgetRendererProps {
  config: WidgetConfig;
}

const WidgetRenderer: React.FC<WidgetRendererProps> = ({ config }) => {
  const { token } = theme.useToken();

  const renderWidget = () => {
    switch (config.type) {
      case 'lineChart':
        return <LineChartWidget config={config} />;
      case 'gauge':
        return <GaugeWidget config={config} />;
      case 'text':
        return <TextWidget config={config} />;
      case 'led':
        return <LedWidget config={config} />;
      case 'compass':
        return <CompassWidget config={config} />;
      case 'accelerometer':
        return <AccelerometerWidget config={config} />;
      default:
        return <div>Unknown widget type</div>;
    }
  };

  return (
    <Card
      size="small"
      title={config.title}
      style={{
        height: '100%',
        background: token.colorBgContainer,
      }}
      styles={{
        body: { padding: 8, height: 'calc(100% - 40px)' },
      }}
    >
      {renderWidget()}
    </Card>
  );
};

export default WidgetRenderer;
