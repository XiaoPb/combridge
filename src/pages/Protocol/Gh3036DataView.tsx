import React from 'react';
import { Table, Button, Empty, Typography, Tag } from 'antd';
import { ClearOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import type { Gh3036FrameData } from '../../api/types';

const { Text } = Typography;

const Gh3036DataView: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { frameData, clearFrameData } = useGh3036Store();

  const columns = [
    {
      title: t('gh3036.timestamp'),
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 120,
      render: (ts: number) => {
        const date = new Date(Number(ts));
        return date.toLocaleTimeString();
      },
    },
    {
      title: t('gh3036.functionId'),
      dataIndex: 'function_id',
      key: 'function_id',
      width: 80,
      render: (_id: number, record: Gh3036FrameData) => (
        <Tag color="blue">{record.function_name}</Tag>
      ),
    },
    {
      title: t('gh3036.frameId'),
      dataIndex: 'frame_id',
      key: 'frame_id',
      width: 60,
    },
    {
      title: t('gh3036.gsData'),
      dataIndex: 'gs_data',
      key: 'gs_data',
      width: 150,
      render: (data: number[]) => (
        <Text style={{ fontSize: 11, fontFamily: 'monospace' }}>
          X:{data[0] ?? '-'} Y:{data[1] ?? '-'} Z:{data[2] ?? '-'}
        </Text>
      ),
    },
    {
      title: t('gh3036.rawdata'),
      dataIndex: 'rawdata',
      key: 'rawdata',
      ellipsis: true,
      render: (data: number[]) => (
        <Text style={{ fontSize: 11, fontFamily: 'monospace' }}>
          {data.slice(0, 8).map((v) => v.toString()).join(', ')}
          {data.length > 8 && '...'}
        </Text>
      ),
    },
    {
      title: t('gh3036.algoData'),
      dataIndex: 'algo_data',
      key: 'algo_data',
      width: 120,
      ellipsis: true,
      render: (data: number[]) => (
        <Text style={{ fontSize: 11, fontFamily: 'monospace' }}>
          {data.slice(0, 4).map((v) => v.toString()).join(', ')}
          {data.length > 4 && '...'}
        </Text>
      ),
    },
  ];

  if (frameData.length === 0) {
    return (
      <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
        <div style={{ flex: '1 1 0', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Empty description={t('gh3036.noData')} />
        </div>
      </div>
    );
  }

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ marginBottom: 8, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {t('gh3036.frameCount', { count: frameData.length })}
        </Text>
        <Button
          size="small"
          icon={<ClearOutlined />}
          onClick={clearFrameData}
        >
          {t('gh3036.clearData')}
        </Button>
      </div>
      <div style={{ flex: '1 1 0', overflow: 'auto' }}>
        <Table
          size="small"
          dataSource={frameData}
          columns={columns}
          rowKey={(record, index) => `${record.timestamp}-${index}`}
          pagination={false}
          scroll={{ y: 'calc(100% - 40px)' }}
        />
      </div>
    </div>
  );
};

export default Gh3036DataView;
