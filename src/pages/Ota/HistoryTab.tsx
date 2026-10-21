import React from 'react';
import { Button, Card, Empty, List, Tag, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { useOtaStore } from '../../stores/otaStore';

export const HistoryTab: React.FC = () => {
  const { t } = useTranslation('ota');
  const logs = useOtaStore((s) => s.logs);
  const clearLogs = useOtaStore((s) => s.clearLogs);

  return (
    <Card
      title={t('history.title')}
      extra={<Button size="small" onClick={clearLogs}>{t('history.clear')}</Button>}
      style={{ flex: 1, overflow: 'auto' }}
    >
      {logs.length === 0 ? (
        <Empty description={t('history.empty')} />
      ) : (
        <List
          size="small"
          dataSource={logs}
          renderItem={(item) => (
            <List.Item>
              <Typography.Text type="secondary">{item.time}</Typography.Text>
              <Tag color={item.level === 'error' ? 'red' : 'blue'}>{item.level}</Tag>
              <Typography.Text>{item.message}</Typography.Text>
            </List.Item>
          )}
        />
      )}
    </Card>
  );
};
