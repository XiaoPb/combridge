import React, { useEffect, useRef } from 'react';
import { theme } from 'antd';
import type { WidgetConfig } from '../../../types/dashboard';
import { useDashboardStore } from '../../../stores/dashboardStore';

interface CompassWidgetProps {
  config: WidgetConfig;
}

const CompassWidget: React.FC<CompassWidgetProps> = ({ config }) => {
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
    ctx.strokeStyle = token.colorBorder;
    ctx.lineWidth = 2;
    ctx.stroke();

    const directions = ['N', 'E', 'S', 'W'];
    ctx.fillStyle = token.colorText;
    ctx.font = 'bold 12px sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    directions.forEach((dir, i) => {
      const angle = (i * Math.PI) / 2 - Math.PI / 2;
      const x = centerX + Math.cos(angle) * (radius - 15);
      const y = centerY + Math.sin(angle) * (radius - 15);
      ctx.fillText(dir, x, y);
    });

    const lastData = dataBuffer[dataBuffer.length - 1];
    const heading = lastData?.values[config.dataKey] ?? 0;
    const headingRad = (heading * Math.PI) / 180 - Math.PI / 2;

    ctx.save();
    ctx.translate(centerX, centerY);
    ctx.rotate(headingRad);

    ctx.beginPath();
    ctx.moveTo(0, -radius + 25);
    ctx.lineTo(-8, 10);
    ctx.lineTo(0, 0);
    ctx.lineTo(8, 10);
    ctx.closePath();
    ctx.fillStyle = token.colorError;
    ctx.fill();

    ctx.beginPath();
    ctx.moveTo(0, radius - 25);
    ctx.lineTo(-8, -10);
    ctx.lineTo(0, 0);
    ctx.lineTo(8, -10);
    ctx.closePath();
    ctx.fillStyle = token.colorTextSecondary;
    ctx.fill();

    ctx.restore();

    ctx.fillStyle = token.colorText;
    ctx.font = 'bold 14px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(`${heading.toFixed(0)}°`, centerX, centerY);
  }, [dataBuffer, config, token]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: '100%', height: '100%', display: 'block' }}
    />
  );
};

export default CompassWidget;
