import React from 'react';
import { Button, Empty, Card } from 'antd';
import { ClearOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import WaveformChart from '../Waveform/WaveformChart';

const Gh3036DataView: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { waveformColumns, waveformRows, clearWaveformData } = useGh3036Store();

  if (waveformRows.length === 0) {
    return (
      <Card
        size="small"
        title={t('gh3036.dataView')}
        style={{ height: '100%' }}
        styles={{ body: { padding: 8, height: 'calc(100% - 40px)', display: 'flex', alignItems: 'center', justifyContent: 'center' } }}
      >
        <Empty description={t('gh3036.noData')} />
      </Card>
    );
  }

  return (
    <Card
      size="small"
      title={t('gh3036.dataView')}
      extra={
        <Button
          size="small"
          icon={<ClearOutlined />}
          onClick={clearWaveformData}
        >
          {t('gh3036.clearData')}
        </Button>
      }
      style={{ height: '100%' }}
      styles={{ body: { padding: 8, height: 'calc(100% - 40px)' } }}
    >
      <WaveformChart
        columns={waveformColumns}
        rows={waveformRows}
        displayRows={500}
      />
    </Card>
  );
};

export default Gh3036DataView;
