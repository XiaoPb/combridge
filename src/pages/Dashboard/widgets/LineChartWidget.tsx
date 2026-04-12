import React, { useEffect, useRef } from 'react';
import { theme } from 'antd';
import type { WidgetConfig } from '../../../types/dashboard';
import { useDashboardStore } from '../../../stores/dashboardStore';

interface LineChartWidgetProps {
  config: WidgetConfig;
}

const LineChartWidget: React.FC<LineChartWidgetProps> = ({ config }) => {
  const { token } = theme.useToken();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const { dataBuffer } = useDashboardStore();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * window.devicePixelRatio;
    canvas.height = rect.height * window.devicePixelRatio;
    ctx.scale(window.devicePixelRatio, window.devicePixelRatio);

    ctx.fillStyle = token.colorFillSecondary;
    ctx.fillRect(0, 0, rect.width, rect.height);

    const data = dataBuffer
      .slice(-100)
      .map((d) => d.values[config.dataKey])
      .filter((v) => v !== undefined);

    if (data.length < 2) {
      ctx.fillStyle = token.colorTextSecondary;
      ctx.font = '12px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText('Waiting for data...', rect.width / 2, rect.height / 2);
      return;
    }

    const min = config.min ?? Math.min(...data);
    const max = config.max ?? Math.max(...data);
    const range = max - min || 1;

    const padding = 20;
    const chartWidth = rect.width - padding * 2;
    const chartHeight = rect.height - padding * 2;

    ctx.strokeStyle = token.colorBorderSecondary;
    ctx.beginPath();
    ctx.moveTo(padding, padding);
    ctx.lineTo(padding, rect.height - padding);
    ctx.lineTo(rect.width - padding, rect.height - padding);
    ctx.stroke();

    ctx.strokeStyle = config.color || token.colorPrimary;
    ctx.lineWidth = 2;
    ctx.beginPath();

    data.forEach((value, index) => {
      const x = padding + (index / (data.length - 1)) * chartWidth;
      const y = rect.height - padding - ((value - min) / range) * chartHeight;

      if (index === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });

    ctx.stroke();

    const lastValue = data[data.length - 1];
    ctx.fillStyle = token.colorText;
    ctx.font = 'bold 14px sans-serif';
    ctx.textAlign = 'right';
    ctx.fillText(
      `${lastValue.toFixed(2)}${config.unit || ''}`,
      rect.width - padding,
      padding + 14
    );
  }, [dataBuffer, config, token]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: '100%', height: '100%', display: 'block' }}
    />
  );
};

export default LineChartWidget;
