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

    const lastData = dataBuffer[dataBuffer.length - 1];
    const accX = lastData?.values[`${config.dataKey}_x`] ?? 0;
    const accY = lastData?.values[`${config.dataKey}_y`] ?? 0;
    const accZ = lastData?.values[`${config.dataKey}_z`] ?? 0;

    const scale = radius / 20;
    const ballX = centerX + accX * scale;
    const ballY = centerY - accY * scale;

    ctx.beginPath();
    ctx.arc(ballX, ballY, 8, 0, Math.PI * 2);
    ctx.fillStyle = config.color || token.colorPrimary;
    ctx.fill();

    ctx.fillStyle = token.colorText;
    ctx.font = '10px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(`X: ${accX.toFixed(2)}`, 5, rect.height - 30);
    ctx.fillText(`Y: ${accY.toFixed(2)}`, 5, rect.height - 18);
    ctx.fillText(`Z: ${accZ.toFixed(2)}`, 5, rect.height - 6);
  }, [dataBuffer, config, token]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: '100%', height: '100%', display: 'block' }}
    />
  );
};

export default AccelerometerWidget;
