import React, { useState } from 'react';
import { Select, Space, Input, Button, Modal, message } from 'antd';
import { FolderOpenOutlined, PlayCircleOutlined, PauseCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { useDashboardStore } from '../../stores/dashboardStore';
import { useSerialStore } from '../../stores/serialStore';
import { useBleStore } from '../../stores/bleStore';
import type { DataSourceType } from '../../types/dashboard';

const DataSourceSelector: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const {
    dataSourceType,
    setDataSourceType,
    connectedDevice,
    setConnectedDevice,
    currentDashboard,
    setCurrentDashboard,
    isRunning,
    setIsRunning,
    addDataPoint,
    parserScript,
  } = useDashboardStore();
  const { ports } = useSerialStore();
  const { connections } = useBleStore();

  const [filePath, setFilePath] = useState<string>('');
  const [fileContent, setFileContent] = useState<string[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [playbackInterval, setPlaybackInterval] = useState<ReturnType<typeof setInterval> | null>(null);

  const dataSourceOptions = [
    { label: t('dataSource.serial'), value: 'serial' as DataSourceType },
    { label: t('dataSource.ble'), value: 'ble' as DataSourceType },
    { label: t('dataSource.file'), value: 'file' as DataSourceType },
    { label: t('dataSource.manual'), value: 'manual' as DataSourceType },
  ];

  const deviceOptions =
    dataSourceType === 'serial'
      ? ports.map((p) => ({ label: p.name, value: p.name }))
      : connections.map((c) => ({ label: c.name || c.address, value: c.address }));

  const handleSelectFile = async () => {
    const selected = await open({
      multiple: false,
      filters: [
        { name: 'Data Files', extensions: ['txt', 'json', 'csv', 'log'] },
      ],
    });

    if (selected) {
      const path = typeof selected === 'string' ? selected : selected.path;
      setFilePath(path);
      if (currentDashboard) {
        setCurrentDashboard({
          ...currentDashboard,
          dataSource: { ...currentDashboard.dataSource, filePath: path },
        });
      }
      message.success(t('fileSelected') || 'File selected');
    }
  };

  const handleStartPlayback = async () => {
    if (!filePath) {
      message.warning(t('selectFileFirst') || 'Please select a file first');
      return;
    }

    try {
      const { readFile } = await import('@tauri-apps/plugin-fs');
      const content = await readFile(filePath);
      const text = new TextDecoder().decode(content);
      const lines = text.split('\n').filter((line) => line.trim());
      setFileContent(lines);
      setCurrentIndex(0);
      setIsRunning(true);

      const interval = setInterval(() => {
        setCurrentIndex((prev) => {
          if (prev >= lines.length - 1) {
            clearInterval(interval);
            setIsRunning(false);
            setPlaybackInterval(null);
            return prev;
          }
          return prev + 1;
        });
      }, currentDashboard?.refreshRate || 100);

      setPlaybackInterval(interval);
    } catch (error) {
      message.error(t('fileReadError') || 'Failed to read file');
    }
  };

  const handleStopPlayback = () => {
    if (playbackInterval) {
      clearInterval(playbackInterval);
      setPlaybackInterval(null);
    }
    setIsRunning(false);
  };

  React.useEffect(() => {
    if (dataSourceType === 'file' && fileContent.length > 0 && currentIndex < fileContent.length) {
      const line = fileContent[currentIndex];
      if (line) {
        addDataPoint({
          timestamp: Date.now(),
          values: { raw: parseFloat(line) || 0 },
        });
      }
    }
  }, [currentIndex, fileContent, dataSourceType]);

  React.useEffect(() => {
    return () => {
      if (playbackInterval) {
        clearInterval(playbackInterval);
      }
    };
  }, [playbackInterval]);

  return (
    <Space>
      <Select
        value={dataSourceType}
        onChange={(type) => {
          setDataSourceType(type);
          if (type !== 'file' && playbackInterval) {
            clearInterval(playbackInterval);
            setPlaybackInterval(null);
            setIsRunning(false);
          }
        }}
        options={dataSourceOptions}
        style={{ width: 120 }}
      />
      {dataSourceType === 'serial' && (
        <Select
          value={connectedDevice}
          onChange={setConnectedDevice}
          options={deviceOptions}
          placeholder={t('selectDevice')}
          style={{ width: 150 }}
        />
      )}
      {dataSourceType === 'ble' && (
        <Select
          value={connectedDevice}
          onChange={setConnectedDevice}
          options={deviceOptions}
          placeholder={t('selectDevice')}
          style={{ width: 150 }}
        />
      )}
      {dataSourceType === 'file' && (
        <>
          <Input
            value={filePath}
            placeholder={t('selectFile')}
            style={{ width: 200 }}
            readOnly
            onClick={handleSelectFile}
          />
          <Button
            icon={<FolderOpenOutlined />}
            onClick={handleSelectFile}
          />
          {isRunning ? (
            <Button
              icon={<PauseCircleOutlined />}
              onClick={handleStopPlayback}
              danger
            />
          ) : (
            <Button
              icon={<PlayCircleOutlined />}
              onClick={handleStartPlayback}
              type="primary"
            />
          )}
        </>
      )}
    </Space>
  );
};

export default DataSourceSelector;
