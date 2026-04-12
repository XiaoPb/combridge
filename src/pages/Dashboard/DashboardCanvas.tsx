import React, { useState } from 'react';
import { Empty, Button, theme } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import WidgetRenderer from './widgets/WidgetRenderer';
import WidgetSelector from './WidgetSelector';
import type { WidgetType, WidgetConfig } from '../../types/dashboard';

const generateId = () => Math.random().toString(36).substring(2, 11);

const DashboardCanvas: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { token } = theme.useToken();
  const {
    currentDashboard,
    isEditMode,
    selectedWidget,
    setSelectedWidget,
    addWidget,
  } = useDashboardStore();
  const [showWidgetSelector, setShowWidgetSelector] = useState(false);

  const handleAddWidget = (type: WidgetType) => {
    const newWidget: WidgetConfig = {
      id: generateId(),
      type,
      title: t(`widgetTypes.${type}`),
      x: 0,
      y: 0,
      width: 200,
      height: 150,
      dataKey: '',
    };
    addWidget(newWidget);
  };

  const handleCanvasClick = (e: React.MouseEvent) => {
    if (!isEditMode) return;
    if (e.target === e.currentTarget) {
      setSelectedWidget(null);
      setShowWidgetSelector(true);
    }
  };

  if (!currentDashboard || currentDashboard.widgets.length === 0) {
    return (
      <div
        style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 16,
          background: token.colorFillSecondary,
        }}
      >
        <Empty description={t('noWidgets')} />
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => setShowWidgetSelector(true)}
        >
          {t('addWidget')}
        </Button>
        <WidgetSelector
          open={showWidgetSelector}
          onClose={() => setShowWidgetSelector(false)}
          onSelect={handleAddWidget}
        />
      </div>
    );
  }

  return (
    <div
      style={{
        flex: 1,
        padding: 16,
        overflow: 'auto',
        background: token.colorFillSecondary,
      }}
      onClick={handleCanvasClick}
    >
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
          gap: 16,
          minHeight: '100%',
        }}
      >
        {currentDashboard.widgets.map((widget) => (
          <div
            key={widget.id}
            onClick={(e) => {
              e.stopPropagation();
              if (isEditMode) {
                setSelectedWidget(widget.id);
              }
            }}
            style={{
              border:
                selectedWidget === widget.id
                  ? `2px solid ${token.colorPrimary}`
                  : isEditMode
                    ? '1px dashed transparent'
                    : '1px solid transparent',
              borderRadius: 8,
              cursor: isEditMode ? 'pointer' : 'default',
              transition: 'border-color 0.2s, box-shadow 0.2s',
              boxShadow:
                selectedWidget === widget.id
                  ? `0 0 0 2px ${token.colorPrimaryBg}`
                  : 'none',
            }}
          >
            <WidgetRenderer config={widget} />
          </div>
        ))}
      </div>
      <WidgetSelector
        open={showWidgetSelector}
        onClose={() => setShowWidgetSelector(false)}
        onSelect={handleAddWidget}
      />
    </div>
  );
};

export default DashboardCanvas;
