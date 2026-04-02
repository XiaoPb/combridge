import React from 'react';
import { Spin } from 'antd';
import { LoadingOutlined } from '@ant-design/icons';

interface LoadingSpinnerProps {
  size?: 'small' | 'default' | 'large';
  tip?: string;
  fullscreen?: boolean;
}

const LoadingSpinner: React.FC<LoadingSpinnerProps> = ({
  size = 'default',
  tip = '加载中...',
  fullscreen = false,
}) => {
  const spinIcon = <LoadingOutlined style={{ fontSize: size === 'large' ? 48 : size === 'small' ? 16 : 24 }} spin />;

  if (fullscreen) {
    return (
      <div
        style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          backgroundColor: 'rgba(255, 255, 255, 0.8)',
          zIndex: 9999,
        }}
      >
        <Spin indicator={spinIcon} tip={tip} size={size} />
      </div>
    );
  }

  return (
    <div
      style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        padding: '40px 0',
      }}
    >
      <Spin indicator={spinIcon} tip={tip} size={size} />
    </div>
  );
};

export default LoadingSpinner;
