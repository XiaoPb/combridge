import React from 'react';
import { Empty, theme } from 'antd';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import WidgetRenderer from './widgets/WidgetRenderer';

const DashboardCanvas: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { token } = theme.useToken();
  const { currentDashboard, isEditMode, selectedWidget, setSelectedWidget } =
    useDashboardStore();

  if (!currentDashboard || currentDashboard.widgets.length === 0) {
    return (
      <div
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: token.colorFillSecondary,
        }}
      >
        <Empty description={t('noWidgets')} />
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
            onClick={() => isEditMode && setSelectedWidget(widget.id)}
            style={{
              border:
                selectedWidget === widget.id
                  ? `2px solid ${token.colorPrimary}`
                  : '1px solid transparent',
              borderRadius: 8,
              cursor: isEditMode ? 'pointer' : 'default',
              transition: 'border-color 0.2s',
            }}
          >
            <WidgetRenderer config={widget} />
          </div>
        ))}
      </div>
    </div>
  );
};

export default DashboardCanvas;
