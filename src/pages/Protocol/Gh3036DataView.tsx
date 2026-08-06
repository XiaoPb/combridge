import React, { useMemo, useCallback } from 'react';
import { Button, Empty, Card, Select, Space, Row, Col, message } from 'antd';
import { ClearOutlined, FileAddOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { gh3036ForceNewCsvFile } from '../../api/gh3036';
import MultiLineChart from '../Waveform/MultiLineChart';

const Gh3036DataView: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { 
    framesData, 
    selectedFunctionId, 
    chartGroups,
    clearWaveformData,
    setSelectedFunctionId,
    setChartGroups
  } = useGh3036Store();

  const functionOptions = useMemo(() => {
    return Array.from(framesData.entries()).map(([id, frames]) => ({
      value: id,
      label: `${frames.function_name} (ID: ${id})`,
    }));
  }, [framesData]);

  const currentFrames = useMemo(() => {
    if (selectedFunctionId === null) return null;
    return framesData.get(selectedFunctionId) ?? null;
  }, [framesData, selectedFunctionId]);

  const availableColumns = useMemo(() => {
    if (!currentFrames) return [];
    return Array.from({ length: currentFrames.channel_count }, (_, i) => `CH${i}`);
  }, [currentFrames]);

  const handleColumnSelect = useCallback((chartIndex: number, columnIndex: number, column: string | null) => {
    const newGroups = [...chartGroups];
    while (newGroups.length <= chartIndex) {
      newGroups.push({ name: `图表 ${newGroups.length + 1}`, columns: [] });
    }
    
    const group = { ...newGroups[chartIndex] };
    const newColumns = [...group.columns];
    
    if (column === null) {
      if (newColumns.length > columnIndex) {
        newColumns.splice(columnIndex, 1);
      }
    } else {
      while (newColumns.length <= columnIndex) {
        newColumns.push('');
      }
      newColumns[columnIndex] = column;
    }
    
    group.columns = newColumns.filter(c => c);
    newGroups[chartIndex] = group;
    
    setChartGroups(newGroups.filter(g => g.columns.length > 0));
  }, [chartGroups, setChartGroups]);

  const handleForceNewCsvFile = useCallback(async () => {
    try {
      await gh3036ForceNewCsvFile();
      message.success(t('gh3036.newCsvFileCreated'));
    } catch (error) {
      message.error(t('gh3036.newCsvFileFailed'));
      console.error('[GH3036] 手动创建新CSV文件失败:', error);
    }
  }, [t]);

  const chartData = useMemo(() => {
    if (!currentFrames || chartGroups.length === 0) {
      return [];
    }

    return chartGroups.map(group => {
      const columns = group.columns.slice(0, 4);
      const rows: number[][] = [];
      
      for (let frameIdx = 0; frameIdx < currentFrames.frame_count; frameIdx++) {
        const row: number[] = [];
        for (const col of columns) {
          const chIdx = parseInt(col.replace('CH', ''), 10);
          row.push(currentFrames.ipd_pa[chIdx]?.[frameIdx] ?? 0);
        }
        rows.push(row);
      }
      
      return { columns, rows };
    });
  }, [currentFrames, chartGroups]);

  if (framesData.size === 0) {
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
        <Space>
          <Select
            size="small"
            style={{ width: 150 }}
            value={selectedFunctionId}
            onChange={setSelectedFunctionId}
            options={functionOptions}
            placeholder={t('gh3036.selectFunction')}
          />
          <Button
            size="small"
            icon={<FileAddOutlined />}
            onClick={handleForceNewCsvFile}
          >
            {t('gh3036.newCsvFile')}
          </Button>
          <Button
            size="small"
            icon={<ClearOutlined />}
            onClick={clearWaveformData}
          >
            {t('gh3036.clearData')}
          </Button>
        </Space>
      }
      style={{ height: '100%' }}
      styles={{ body: { padding: 8, height: 'calc(100% - 40px)', overflow: 'auto' } }}
    >
      <Space orientation="vertical" style={{ width: '100%' }} size="small">
        <Row gutter={8}>
          {[0, 1].map(chartIndex => (
            <Col span={12} key={chartIndex}>
              <Card 
                size="small" 
                title={`图表 ${chartIndex + 1}`}
                style={{ marginBottom: 8 }}
              >
                <Space size="small" wrap>
                  {[0, 1, 2, 3].map(lineIndex => (
                    <Select
                      key={lineIndex}
                      size="small"
                      style={{ width: 80 }}
                      value={chartGroups[chartIndex]?.columns[lineIndex] ?? null}
                      onChange={(value) => handleColumnSelect(chartIndex, lineIndex, value)}
                      options={availableColumns.map(col => ({ value: col, label: col }))}
                      placeholder={`线${lineIndex + 1}`}
                      allowClear
                    />
                  ))}
                </Space>
              </Card>
            </Col>
          ))}
        </Row>
        
        {chartData.map((data, index) => (
          <div 
            key={index} 
            style={{ 
              height: 200, 
              border: '1px solid #d9d9d9', 
              borderRadius: 4,
              marginBottom: 8 
            }}
          >
            {data.columns.length > 0 ? (
              <MultiLineChart
                columns={data.columns}
                rows={data.rows}
                chartGroups={[{ name: `图表 ${index + 1}`, columns: data.columns }]}
              />
            ) : (
              <Empty 
                description={`请选择图表 ${index + 1} 的通道`} 
                style={{ marginTop: 60 }}
              />
            )}
          </div>
        ))}
      </Space>
    </Card>
  );
};

export default Gh3036DataView;
