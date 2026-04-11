import React, { useState, useCallback, useRef, useEffect } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';

interface WaveformChartProps {
  columns: string[];
  rows: number[][];
  displayRows?: number;
}

const COLORS = [
  '#1890ff',
  '#52c41a',
  '#faad14',
  '#f5222d',
  '#722ed1',
  '#13c2c2',
  '#eb2f96',
  '#fa8c16',
];

interface ContextMenuPosition {
  x: number;
  y: number;
}

const WaveformChart: React.FC<WaveformChartProps> = ({
  columns,
  rows,
  displayRows = 500,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuPosition | null>(null);

  const displayData = rows.slice(-displayRows).map((row, index) => {
    const point: Record<string, number | string> = { index };
    columns.forEach((col, colIndex) => {
      if (colIndex < row.length) {
        point[col] = row[colIndex];
      }
    });
    return point;
  });

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  }, []);

  const handleCloseContextMenu = useCallback(() => {
    setContextMenu(null);
  }, []);

  const handleSaveAsImage = useCallback(() => {
    if (!containerRef.current) return;

    const svgElement = containerRef.current.querySelector('svg');
    if (!svgElement) return;

    const svgData = new XMLSerializer().serializeToString(svgElement);
    const svgBlob = new Blob([svgData], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(svgBlob);

    const link = document.createElement('a');
    link.href = url;
    link.download = `waveform_${Date.now()}.svg`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);

    setContextMenu(null);
  }, []);

  const handleSaveAsPNG = useCallback(() => {
    if (!containerRef.current) return;

    const svgElement = containerRef.current.querySelector('svg');
    if (!svgElement) return;

    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    const svgData = new XMLSerializer().serializeToString(svgElement);
    const img = new Image();

    img.onload = () => {
      canvas.width = img.width;
      canvas.height = img.height;
      ctx?.drawImage(img, 0, 0);

      canvas.toBlob((blob) => {
        if (blob) {
          const url = URL.createObjectURL(blob);
          const link = document.createElement('a');
          link.href = url;
          link.download = `waveform_${Date.now()}.png`;
          document.body.appendChild(link);
          link.click();
          document.body.removeChild(link);
          URL.revokeObjectURL(url);
        }
      });
    };

    img.src = 'data:image/svg+xml;base64,' + btoa(unescape(encodeURIComponent(svgData)));
    setContextMenu(null);
  }, []);

  useEffect(() => {
    const handleClickOutside = () => {
      setContextMenu(null);
    };
    if (contextMenu) {
      document.addEventListener('click', handleClickOutside);
      return () => {
        document.removeEventListener('click', handleClickOutside);
      };
    }
  }, [contextMenu]);

  if (columns.length === 0 || rows.length === 0) {
    return (
      <div
        style={{
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--text-secondary)',
        }}
      >
        暂无数据
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      className="chart-container"
      style={{ width: '100%', height: '100%' }}
      onContextMenu={handleContextMenu}
    >
      <ResponsiveContainer width="100%" height="100%">
        <LineChart
          data={displayData}
          margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
        >
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border-color)" />
          <XAxis
            dataKey="index"
            stroke="var(--text-secondary)"
            tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
          />
          <YAxis
            stroke="var(--text-secondary)"
            tick={{ fill: 'var(--text-secondary)', fontSize: 12 }}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: 'var(--bg-secondary)',
              border: '1px solid var(--border-color)',
              borderRadius: 4,
            }}
            labelStyle={{ color: 'var(--text-primary)' }}
          />
          <Legend />
          {columns.map((col, index) => (
            <Line
              key={col}
              type="monotone"
              dataKey={col}
              stroke={COLORS[index % COLORS.length]}
              dot={false}
              strokeWidth={1.5}
              isAnimationActive={false}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>

      {contextMenu && (
        <div
          className="chart-context-menu"
          style={{
            left: contextMenu.x,
            top: contextMenu.y,
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <div
            className="chart-context-menu-item"
            onClick={(e) => {
              e.stopPropagation();
              handleSaveAsImage();
            }}
          >
            保存为 SVG
          </div>
          <div
            className="chart-context-menu-item"
            onClick={(e) => {
              e.stopPropagation();
              handleSaveAsPNG();
            }}
          >
            保存为 PNG
          </div>
        </div>
      )}
    </div>
  );
};

export default WaveformChart;
