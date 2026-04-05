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
import { useTranslation } from 'react-i18next';
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

const formatUptime = (secs: number, t: (key: string) => string): string => {
  const days = Math.floor(secs / 86400);
  const hours = Math.floor((secs % 86400) / 3600);
  const minutes = Math.floor((secs % 3600) / 60);
  const parts: string[] = [];
  if (days > 0) parts.push(`${days}${t('time.days')}`);
  if (hours > 0) parts.push(`${hours}${t('time.hours')}`);
  if (minutes > 0) parts.push(`${minutes}${t('time.minutes')}`);
  return parts.join(' ') || t('value.justStarted');
};

const SystemInfo: React.FC = () => {
  const { t } = useTranslation('system');
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
        setError(err instanceof Error ? err.message : t('message.loadFailed'));
      } finally {
        setLoading(false);
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [t]);

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
            {t('message.loading')}
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
            {t('title.about')}
          </span>
        }
      >
        <Descriptions column={2} size="small">
          <Descriptions.Item label={t('label.appName')}>
            <Text strong>ComBridge</Text>
          </Descriptions.Item>
          <Descriptions.Item label={t('label.version')}>
            <Tag color="blue">v{appVersion || systemInfo?.app_version || '未知'}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label={t('label.description')} span={2}>
            {t('value.description')}
          </Descriptions.Item>
          <Descriptions.Item label={t('label.author')} span={2}>
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
            {t('button.documentation')}
          </Button>
          <Button
            type="link"
            icon={<GithubOutlined />}
            onClick={() => systemApi.openUrl('https://github.com/combridge/combridge')}
          >
            {t('button.github')}
          </Button>
          <Button
            type="link"
            icon={<BugOutlined />}
            onClick={() => systemApi.openUrl('https://github.com/combridge/combridge/issues')}
          >
            {t('button.feedback')}
          </Button>
        </Space>
      </Card>

      <Card
        title={
          <span>
            <DesktopOutlined style={{ marginRight: 8 }} />
            {t('title.systemInfo')}
          </span>
        }
      >
        {systemInfo && (
          <Descriptions column={2} bordered size="small">
            <Descriptions.Item label={t('label.os')}>{systemInfo.os_name}</Descriptions.Item>
            <Descriptions.Item label={t('label.osVersion')}>{systemInfo.os_version}</Descriptions.Item>
            <Descriptions.Item label={t('label.arch')}>{systemInfo.arch}</Descriptions.Item>
            <Descriptions.Item label={t('label.hostname')}>{systemInfo.hostname}</Descriptions.Item>
            <Descriptions.Item label={t('label.cpuCores')}>{systemInfo.cpu_count} {t('value.cores')}</Descriptions.Item>
            <Descriptions.Item label={t('label.totalMemory')}>
              {formatBytes(systemInfo.total_memory)}
            </Descriptions.Item>
            <Descriptions.Item label={t('label.appVersion')}>
              <Tag color="blue">v{systemInfo.app_version}</Tag>
            </Descriptions.Item>
          </Descriptions>
        )}
      </Card>

      <Card
        title={
          <span>
            <CloudServerOutlined style={{ marginRight: 8 }} />
            {t('title.runtimeStatus')}
          </span>
        }
      >
        {systemStatus && (
          <div>
            <div style={{ marginBottom: 24 }}>
              <Text strong>{t('label.cpuUsage')}</Text>
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
                {t('label.memoryUsage')} ({formatBytes(systemStatus.used_memory)} /{' '}
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
                {t('label.systemUptime')}
              </Text>
              <Text style={{ marginLeft: 8 }}>{formatUptime(systemStatus.uptime_secs, t)}</Text>
            </div>
          </div>
        )}
      </Card>

      <Card
        title={
          <span>
            <DatabaseOutlined style={{ marginRight: 8 }} />
            {t('title.diskUsage')}
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
