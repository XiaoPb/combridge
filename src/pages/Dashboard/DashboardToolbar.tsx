import React, { useState } from 'react';
import { Space, Button, Select, Dropdown, message, Popconfirm } from 'antd';
import {
  PlayCircleOutlined,
  PauseCircleOutlined,
  PlusOutlined,
  SaveOutlined,
  SettingOutlined,
  DownloadOutlined,
  UploadOutlined,
  DeleteOutlined,
  EditOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';
import { useDashboardStore } from '../../stores/dashboardStore';
import DataSourceSelector from './DataSourceSelector';
import ParserSelector from './ParserSelector';
import ParserScriptManager from './ParserScriptManager';
import type { DashboardConfig } from '../../types/dashboard';

const DashboardToolbar: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const {
    isRunning,
    setIsRunning,
    currentDashboard,
    saveDashboard,
    createNewDashboard,
    savedDashboards,
    setCurrentDashboard,
    deleteDashboard,
    renameDashboard,
    isEditMode,
    setIsEditMode,
  } = useDashboardStore();
  const [showScriptManager, setShowScriptManager] = useState(false);
  const [newName, setNewName] = useState('');

  const handleToggleRun = () => {
    setIsRunning(!isRunning);
  };

  const handleSave = () => {
    if (currentDashboard) {
      saveDashboard(currentDashboard);
      message.success(t('dashboardSaved') || 'Dashboard saved');
    }
  };

  const handleNew = () => {
    createNewDashboard();
  };

  const handleDelete = () => {
    if (currentDashboard) {
      deleteDashboard(currentDashboard.id);
      message.success(t('dashboardDeleted') || 'Dashboard deleted');
    }
  };

  const handleExport = () => {
    if (currentDashboard) {
      const json = JSON.stringify(currentDashboard, null, 2);
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${currentDashboard.name}.json`;
      a.click();
      URL.revokeObjectURL(url);
      message.success(t('dashboardExported') || 'Dashboard exported');
    }
  };

  const handleImport = async () => {
    const selected = await open({
      multiple: false,
      filters: [
        { name: 'Dashboard Files', extensions: ['json'] },
      ],
    });

    if (selected && typeof selected === 'string') {
      try {
        const content = await readFile(selected);
        const text = new TextDecoder().decode(content);
        const dashboard = JSON.parse(text) as DashboardConfig;

        if (!dashboard.id || !dashboard.name || !dashboard.widgets) {
          message.error(t('invalidDashboard') || 'Invalid dashboard file');
          return;
        }

        dashboard.id = `imported_${Date.now()}`;
        saveDashboard(dashboard);
        message.success(t('dashboardImported') || 'Dashboard imported');
      } catch (error) {
        message.error(t('importError') || 'Failed to import dashboard');
      }
    }
  };

  const dashboardOptions = savedDashboards.map((d) => ({
    label: d.name,
    value: d.id,
  }));

  return (
    <div
      style={{
        padding: '8px 16px',
        borderBottom: '1px solid #f0f0f0',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        background: '#fff',
      }}
    >
      <Space>
        <Select
          value={currentDashboard?.id}
          onChange={(id) => {
            const dashboard = savedDashboards.find((d) => d.id === id);
            if (dashboard) {
              setCurrentDashboard(dashboard);
            }
          }}
          options={dashboardOptions}
          style={{ width: 200 }}
          placeholder={t('selectDashboard')}
        />
        <Button icon={<PlusOutlined />} onClick={handleNew}>
          {t('new')}
        </Button>
        <Button icon={<SaveOutlined />} onClick={handleSave}>
          {t('save')}
        </Button>
        <Button
          icon={<EditOutlined />}
          onClick={() => {
            setNewName(currentDashboard?.name || '');
            setEditingName(true);
          }}
        >
          {t('rename')}
        </Button>
        <Popconfirm
          title={t('deleteConfirm')}
          onConfirm={handleDelete}
          disabled={savedDashboards.length <= 1}
        >
          <Button
            icon={<DeleteOutlined />}
            danger
            disabled={savedDashboards.length <= 1}
          >
            {t('delete')}
          </Button>
        </Popconfirm>
      </Space>

      <DataSourceSelector />

      <ParserSelector onOpenManager={() => setShowScriptManager(true)} />

      <Space>
        <Button
          type={isRunning ? 'default' : 'primary'}
          icon={isRunning ? <PauseCircleOutlined /> : <PlayCircleOutlined />}
          onClick={handleToggleRun}
          danger={isRunning}
        >
          {isRunning ? t('stop') : t('start')}
        </Button>
        <Button
          type={isEditMode ? 'primary' : 'default'}
          icon={<SettingOutlined />}
          onClick={() => setIsEditMode(!isEditMode)}
        >
          {t('edit')}
        </Button>
        <Dropdown
          menu={{
            items: [
              {
                key: 'export',
                icon: <DownloadOutlined />,
                label: t('export'),
                onClick: handleExport,
              },
              {
                key: 'import',
                icon: <UploadOutlined />,
                label: t('import'),
                onClick: handleImport,
              },
            ],
          }}
        >
          <Button icon={<DownloadOutlined />} />
        </Dropdown>
      </Space>

      <ParserScriptManager
        open={showScriptManager}
        onClose={() => setShowScriptManager(false)}
      />
    </div>
  );
};

export default DashboardToolbar;
