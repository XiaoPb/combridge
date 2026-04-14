import React, { useEffect, useRef } from 'react';
import { theme } from 'antd';
import type { WidgetConfig } from '../../../types/dashboard';
import { useDashboardStore } from '../../../stores/dashboardStore';

interface AccelerometerWidgetProps {
  config: WidgetConfig;
}

const AccelerometerWidget: React.FC<AccelerometerWidgetProps> = ({ config }) => {
  const { token } = theme.useToken();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const { dataBuffer } = useDashboardStore();

  const latestData = dataBuffer.length > 0 ? dataBuffer[dataBuffer.length - 1].values : {};
  const x = latestData[`${config.dataKey}_x`] ?? latestData['x'] ?? 0;
  const y = latestData[`${config.dataKey}_y`] ?? latestData['y'] ?? 0;
  const z = latestData[`${config.dataKey}_z`] ?? latestData['z'] ?? 0;

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
    const radius = Math.min(centerX, centerY) - 20;

    ctx.clearRect(0, 0, rect.width, rect.height);

    ctx.strokeStyle = token.colorBorderSecondary;
    ctx.lineWidth = 1;

    for (let i = 1; i <= 3; i++) {
      ctx.beginPath();
      ctx.arc(centerX, centerY, (radius * i) / 3, 0, Math.PI * 2);
      ctx.stroke();
    }

    ctx.beginPath();
    ctx.moveTo(centerX - radius, centerY);
    ctx.lineTo(centerX + radius, centerY);
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(centerX, centerY - radius);
    ctx.lineTo(centerX, centerY + radius);
    ctx.stroke();

    const min = config.min ?? -10;
    const max = config.max ?? 10;
    const range = max - min || 1;
    const scale = radius / (range / 2);
    const ballX = centerX + x * scale;
    const ballY = centerY - y * scale;

    ctx.beginPath();
    ctx.arc(ballX, ballY, 8, 0, Math.PI * 2);
    ctx.fillStyle = config.color || token.colorPrimary;
    ctx.fill();

    ctx.fillStyle = token.colorText;
    ctx.font = '10px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(`X: ${x.toFixed(2)}`, 5, rect.height - 30);
    ctx.fillText(`Y: ${y.toFixed(2)}`, 5, rect.height - 18);
    ctx.fillText(`Z: ${z.toFixed(2)}`, 5, rect.height - 6);
  }, [x, y, z, config, token]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: '100%', height: 150, display: 'block' }}
    />
  );
};

export default AccelerometerWidget;
