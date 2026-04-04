import React, { useEffect, useState } from 'react';
import { Card, Descriptions, Progress, Tag, Spin, Typography, Button, Space, Divider } from 'antd';
import {
  DesktopOutlined,
  DatabaseOutlined,
  CloudServerOutlined,
  ClockCircleOutlined,
  InfoCircleOutlined,
  QuestionCircleOutlined,
  GithubOutlined,
  BugOutlined,
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { systemApi } from '../../api/tauri';

const { Text } = Typography;

interface SystemInfo {
  os_name: string;
  os_version: string;
  arch: string;
  hostname: string;
  cpu_count: number;
  total_memory: number;
  app_version: string;
}

interface SystemStatus {
  cpu_usage: number;
  memory_usage: number;
  used_memory: number;
  total_memory: number;
  uptime_secs: number;
  disk_usage: DiskUsage[];
}

interface DiskUsage {
  name: string;
  total_space: number;
  available_space: number;
  used_space: number;
  usage_percent: number;
}

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

const formatUptime = (secs: number): string => {
  const days = Math.floor(secs / 86400);
  const hours = Math.floor((secs % 86400) / 3600);
  const minutes = Math.floor((secs % 3600) / 60);
  const parts: string[] = [];
  if (days > 0) parts.push(`${days}天`);
  if (hours > 0) parts.push(`${hours}小时`);
  if (minutes > 0) parts.push(`${minutes}分钟`);
  return parts.join(' ') || '刚刚启动';
};

const SystemInfo: React.FC = () => {
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [systemStatus, setSystemStatus] = useState<SystemStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [appVersion, setAppVersion] = useState<string>('');

  useEffect(() => {
    const fetchData = async () => {
      setLoading(true);
      setError(null);
      try {
        const [info, status] = await Promise.all([
          invoke<SystemInfo>('get_system_info'),
          invoke<SystemStatus>('get_system_status'),
        ]);
        setSystemInfo(info);
        setSystemStatus(status);
      } catch (err) {
        setError(err instanceof Error ? err.message : '获取系统信息失败');
      } finally {
        setLoading(false);
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    systemApi.getAppVersion().then(setAppVersion).catch(() => {
      console.error('获取应用版本失败');
    });
  }, []);

  if (loading && !systemInfo) {
    return (
      <Card>
        <div style={{ textAlign: 'center', padding: '40px' }}>
          <Spin size="large" />
          <Text type="secondary" style={{ display: 'block', marginTop: 16 }}>
            正在获取系统信息...
          </Text>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <div style={{ textAlign: 'center', padding: '40px', color: '#ff4d4f' }}>
          <Text type="danger">{error}</Text>
        </div>
      </Card>
    );
  }

  return (
    <div style={{ display: 'grid', gap: 16 }}>
      <Card
        title={
          <span>
            <InfoCircleOutlined style={{ marginRight: 8 }} />
            关于 ComBridge
          </span>
        }
      >
        <Descriptions column={2} size="small">
          <Descriptions.Item label="应用名称">
            <Text strong>ComBridge</Text>
          </Descriptions.Item>
          <Descriptions.Item label="版本">
            <Tag color="blue">v{appVersion || systemInfo?.app_version || '未知'}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="描述" span={2}>
            串口与蓝牙调试工具
          </Descriptions.Item>
          <Descriptions.Item label="作者" span={2}>
            ComBridge Team
          </Descriptions.Item>
        </Descriptions>
        <Divider style={{ margin: '16px 0' }} />
        <Space wrap>
          <Button
            type="link"
            icon={<QuestionCircleOutlined />}
            onClick={() => systemApi.openUrl('https://github.com/combridge/combridge/wiki')}
          >
            文档
          </Button>
          <Button
            type="link"
            icon={<GithubOutlined />}
            onClick={() => systemApi.openUrl('https://github.com/combridge/combridge')}
          >
            GitHub
          </Button>
          <Button
            type="link"
            icon={<BugOutlined />}
            onClick={() => systemApi.openUrl('https://github.com/combridge/combridge/issues')}
          >
            问题反馈
          </Button>
        </Space>
      </Card>

      <Card
        title={
          <span>
            <DesktopOutlined style={{ marginRight: 8 }} />
            系统信息
          </span>
        }
      >
        {systemInfo && (
          <Descriptions column={2} bordered size="small">
            <Descriptions.Item label="操作系统">{systemInfo.os_name}</Descriptions.Item>
            <Descriptions.Item label="系统版本">{systemInfo.os_version}</Descriptions.Item>
            <Descriptions.Item label="架构">{systemInfo.arch}</Descriptions.Item>
            <Descriptions.Item label="主机名">{systemInfo.hostname}</Descriptions.Item>
            <Descriptions.Item label="CPU核心数">{systemInfo.cpu_count} 核</Descriptions.Item>
            <Descriptions.Item label="总内存">
              {formatBytes(systemInfo.total_memory)}
            </Descriptions.Item>
            <Descriptions.Item label="应用版本">
              <Tag color="blue">v{systemInfo.app_version}</Tag>
            </Descriptions.Item>
          </Descriptions>
        )}
      </Card>

      <Card
        title={
          <span>
            <CloudServerOutlined style={{ marginRight: 8 }} />
            运行状态
          </span>
        }
      >
        {systemStatus && (
          <div>
            <div style={{ marginBottom: 24 }}>
              <Text strong>CPU 使用率</Text>
              <Progress
                percent={Math.round(systemStatus.cpu_usage)}
                status={systemStatus.cpu_usage > 80 ? 'exception' : 'active'}
                strokeColor={{
                  '0%': '#108ee9',
                  '100%': systemStatus.cpu_usage > 80 ? '#ff4d4f' : '#87d068',
                }}
              />
            </div>

            <div style={{ marginBottom: 24 }}>
              <Text strong>
                内存使用 ({formatBytes(systemStatus.used_memory)} /{' '}
                {formatBytes(systemStatus.total_memory)})
              </Text>
              <Progress
                percent={Math.round(systemStatus.memory_usage)}
                status={systemStatus.memory_usage > 80 ? 'exception' : 'active'}
                strokeColor={{
                  '0%': '#108ee9',
                  '100%': systemStatus.memory_usage > 80 ? '#ff4d4f' : '#87d068',
                }}
              />
            </div>

            <div style={{ marginBottom: 16 }}>
              <Text strong>
                <ClockCircleOutlined style={{ marginRight: 8 }} />
                系统运行时间
              </Text>
              <Text style={{ marginLeft: 8 }}>{formatUptime(systemStatus.uptime_secs)}</Text>
            </div>
          </div>
        )}
      </Card>

      <Card
        title={
          <span>
            <DatabaseOutlined style={{ marginRight: 8 }} />
            磁盘使用
          </span>
        }
      >
        {systemStatus?.disk_usage.map((disk, index) => (
          <div key={index} style={{ marginBottom: 16 }}>
            <Text strong>{disk.name}</Text>
            <Text type="secondary" style={{ marginLeft: 8 }}>
              ({formatBytes(disk.used_space)} / {formatBytes(disk.total_space)})
            </Text>
            <Progress
              percent={Math.round(disk.usage_percent)}
              size="small"
              status={disk.usage_percent > 90 ? 'exception' : 'active'}
            />
          </div>
        ))}
      </Card>
    </div>
  );
};

export default SystemInfo;
