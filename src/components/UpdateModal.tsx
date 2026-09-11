import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Alert, Button, Modal, Progress, Space, Typography } from 'antd';
import { DownloadOutlined, FolderOpenOutlined, ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import {
  updateApi,
  type DownloadProgress,
  type DownloadResult,
  type UpdateInfo,
} from '../api/tauri';

const { Paragraph, Text } = Typography;

interface UpdateModalProps {
  update: UpdateInfo | null;
  open: boolean;
  onClose: () => void;
}

type DownloadState = 'available' | 'downloading' | 'downloaded' | 'error';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function UpdateModal({ update, open, onClose }: UpdateModalProps) {
  const { t } = useTranslation();
  const [state, setState] = useState<DownloadState>('available');
  const [progress, setProgress] = useState<DownloadProgress>({ downloaded: 0 });
  const [result, setResult] = useState<DownloadResult | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen<DownloadProgress>('update-download-progress', (event) => {
      if (!disposed) setProgress(event.payload);
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    setState('available');
    setProgress({ downloaded: 0 });
    setResult(null);
    setError('');
  }, [update?.latest_version]);

  if (!update) return null;

  const startDownload = async () => {
    setState('downloading');
    setError('');
    try {
      const downloaded = await updateApi.download(update.asset);
      setResult(downloaded);
      setState('downloaded');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setState('error');
    }
  };

  const openInstaller = async () => {
    if (!result) return;
    try {
      await updateApi.openInstaller(result.path);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setState('error');
    }
  };

  const percent = Math.min(100, Math.max(0, Math.round(progress.percent ?? 0)));
  const progressText = progress.total
    ? `${formatBytes(progress.downloaded)} / ${formatBytes(progress.total)}`
    : formatBytes(progress.downloaded);

  return (
    <Modal
      open={open}
      title={t('update.title')}
      onCancel={state === 'downloading' ? undefined : onClose}
      closable={state !== 'downloading'}
      maskClosable={state !== 'downloading'}
      width={560}
      footer={null}
    >
      <Space direction="vertical" size={16} style={{ width: '100%' }}>
        <div>
          <Text>{t('update.versionChange', {
            current: update.current_version,
            latest: update.latest_version,
          })}</Text>
          {update.published_at && (
            <Text type="secondary" style={{ display: 'block', marginTop: 4 }}>
              {t('update.publishedAt', { date: new Date(update.published_at).toLocaleDateString() })}
            </Text>
          )}
        </div>

        {update.body && (
          <Paragraph
            style={{ maxHeight: 220, overflow: 'auto', whiteSpace: 'pre-wrap', marginBottom: 0 }}
          >
            {update.body}
          </Paragraph>
        )}

        {state === 'downloading' && (
          <div>
            <Progress percent={percent} status="active" />
            <Text type="secondary">{progressText}</Text>
          </div>
        )}

        {state === 'downloaded' && (
          <Alert type="success" showIcon message={t('update.downloadComplete')} />
        )}

        {state === 'error' && (
          <Alert type="error" showIcon message={t('update.failed')} description={error} />
        )}

        <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
          {state !== 'downloading' && state !== 'downloaded' && (
            <Button onClick={onClose}>{t('update.later')}</Button>
          )}
          {state === 'available' && (
            <Button type="primary" icon={<DownloadOutlined />} onClick={startDownload}>
              {t('update.download')}
            </Button>
          )}
          {state === 'error' && (
            <Button type="primary" icon={<ReloadOutlined />} onClick={result ? openInstaller : startDownload}>
              {t('update.retry')}
            </Button>
          )}
          {state === 'downloaded' && (
            <Button type="primary" icon={<FolderOpenOutlined />} onClick={openInstaller}>
              {t('update.openInstaller')}
            </Button>
          )}
        </Space>
      </Space>
    </Modal>
  );
}
