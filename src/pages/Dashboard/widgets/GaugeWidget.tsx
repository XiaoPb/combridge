import React, { useEffect, useRef } from 'react';
import { theme } from 'antd';

interface GaugeWidgetProps {
  title: string;
  value: number;
  unit?: string;
  min?: number;
  max?: number;
  color?: string;
}

const GaugeWidget: React.FC<GaugeWidgetProps> = ({
  title,
  value,
  unit = '',
  min = 0,
  max = 100,
  color,
}) => {
  const { token } = theme.useToken();
  const canvasRef = useRef<HTMLCanvasElement>(null);

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

    const range = max - min || 1;
    const normalizedValue = Math.max(0, Math.min(1, (value - min) / range));

    const startAngle = -Math.PI / 2;
    const endAngle = startAngle + normalizedValue * Math.PI * 2;

    ctx.beginPath();
    ctx.moveTo(centerX, centerY);
    ctx.arc(centerX, centerY, radius - 5, startAngle, endAngle);
    ctx.closePath();
    ctx.fillStyle = color || token.colorPrimary;
    ctx.fill();

    ctx.fillStyle = token.colorText;
    ctx.font = 'bold 16px sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(
      `${value.toFixed(1)}${unit}`,
      centerX,
      centerY
    );
  }, [value, min, max, color, unit, token]);

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ textAlign: 'center', marginBottom: 8, fontWeight: 500 }}>{title}</div>
      <canvas
        ref={canvasRef}
        style={{ flex: 1, width: '100%', display: 'block' }}
      />
    </div>
  );
};

export default GaugeWidget;
