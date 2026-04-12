import React, { useEffect, useRef } from 'react';
import { theme } from 'antd';

interface CompassWidgetProps {
  value: number;
  color?: string;
}

const CompassWidget: React.FC<CompassWidgetProps> = ({
  value,
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

    const headingRad = (value * Math.PI) / 180 - Math.PI / 2;

    ctx.save();
    ctx.translate(centerX, centerY);
    ctx.rotate(headingRad);

    ctx.beginPath();
    ctx.moveTo(0, -radius + 25);
    ctx.lineTo(-8, 10);
    ctx.lineTo(0, 0);
    ctx.lineTo(8, 10);
    ctx.closePath();
    ctx.fillStyle = color || token.colorError;
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
    ctx.fillText(`${value.toFixed(0)}°`, centerX, centerY);
  }, [value, color, token]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: '100%', height: 150, display: 'block' }}
    />
  );
};

export default CompassWidget;
