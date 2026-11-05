import React from 'react';
import { Button, Card, Space, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useTranslation } from 'react-i18next';
import { useGh3220Store } from '../../stores/gh3220Store';
import type { Gh3220EventPayload } from '../../types/gh3220';

const EventTab: React.FC = () => {
  const { t } = useTranslation('gh3220');
  const { events, clearEvents } = useGh3220Store();

  const columns: ColumnsType<Gh3220EventPayload> = [
    {
      title: t('events.time'),
      dataIndex: 'tsMs',
      width: 180,
      render: (value: number) => new Date(value).toLocaleString(),
    },
    {
      title: t('events.eventId'),
      dataIndex: 'eventId',
      width: 140,
      render: (value: number) => `0x${value.toString(16).padStart(4, '0')}`,
    },
    {
      title: t('events.reportId'),
      dataIndex: 'reportId',
      width: 100,
    },
    {
      title: t('events.ack'),
      dataIndex: 'acked',
      width: 100,
      render: (value: boolean) => (
        <Tag color={value ? 'success' : 'error'}>
          {value ? t('common.yes') : t('common.no')}
        </Tag>
      ),
    },
  ];

  return (
    <Card
      title={t('events.title')}
      extra={
        <Space>
          <Typography.Text type="secondary">{t('events.count', { count: events.length })}</Typography.Text>
          <Button size="small" onClick={clearEvents}>
            {t('events.clear')}
          </Button>
        </Space>
      }
    >
      <Table<Gh3220EventPayload>
        size="small"
        rowKey={(row) => `${row.tsMs}-${row.eventId}-${row.reportId}`}
        columns={columns}
        dataSource={events}
        pagination={{ pageSize: 20, size: 'small' }}
        locale={{ emptyText: t('events.empty') }}
      />
    </Card>
  );
};

export default EventTab;
