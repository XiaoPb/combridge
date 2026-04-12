import React, { useEffect, useRef } from 'react';
import { theme } from 'antd';
import type { WidgetConfig } from '../../../types/dashboard';
import { useDashboardStore } from '../../../stores/dashboardStore';

interface GaugeWidgetProps {
  config: WidgetConfig;
}

const GaugeWidget: React.FC<GaugeWidgetProps> = ({ config }) => {
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

    const centerX = rect.width / 2;
    const centerY = rect.height / 2;
    const radius = Math.min(centerX, centerY) - 10;

    ctx.clearRect(0, 0, rect.width, rect.height);

    ctx.beginPath();
    ctx.arc(centerX, centerY, radius, 0, Math.PI * 2);
    ctx.fillStyle = token.colorFillSecondary;
    ctx.fill();

    const min = config.min ?? 0;
    const max = config.max ?? 100;
    const range = max - min || 1;

    const lastData = dataBuffer[dataBuffer.length - 1];
    const value = lastData?.values[config.dataKey] ?? min;
    const normalizedValue = Math.max(0, Math.min(1, (value - min) / range));

    const startAngle = -Math.PI / 2;
    const endAngle = startAngle + normalizedValue * Math.PI * 2;

    ctx.beginPath();
    ctx.moveTo(centerX, centerY);
    ctx.arc(centerX, centerY, radius - 5, startAngle, endAngle);
    ctx.closePath();
    ctx.fillStyle = config.color || token.colorPrimary;
    ctx.fill();

    ctx.fillStyle = token.colorText;
    ctx.font = 'bold 16px sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(
      `${value.toFixed(1)}${config.unit || ''}`,
      centerX,
      centerY
    );
  }, [dataBuffer, config, token]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: '100%', height: '100%', display: 'block' }}
    />
  );
};

export default GaugeWidget;
