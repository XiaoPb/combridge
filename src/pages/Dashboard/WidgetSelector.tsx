import React from 'react';
import { Modal, Card, Row, Col, theme, Typography } from 'antd';
import {
  LineChartOutlined,
  DashboardOutlined,
  FontSizeOutlined,
  BulbOutlined,
  CompassOutlined,
  ControlOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { WidgetType } from '../../types/dashboard';

interface WidgetSelectorProps {
  open: boolean;
  onClose: () => void;
  onSelect: (type: WidgetType) => void;
}

interface WidgetTypeInfo {
  type: WidgetType;
  nameKey: string;
  descKey: string;
  icon: React.ReactNode;
}

const widgetTypes: WidgetTypeInfo[] = [
  {
    type: 'lineChart',
    nameKey: 'lineChart',
    descKey: 'lineChartDesc',
    icon: <LineChartOutlined style={{ fontSize: 32 }} />,
  },
  {
    type: 'gauge',
    nameKey: 'gauge',
    descKey: 'gaugeDesc',
    icon: <DashboardOutlined style={{ fontSize: 32 }} />,
  },
  {
    type: 'text',
    nameKey: 'text',
    descKey: 'textDesc',
    icon: <FontSizeOutlined style={{ fontSize: 32 }} />,
  },
  {
    type: 'led',
    nameKey: 'led',
    descKey: 'ledDesc',
    icon: <BulbOutlined style={{ fontSize: 32 }} />,
  },
  {
    type: 'compass',
    nameKey: 'compass',
    descKey: 'compassDesc',
    icon: <CompassOutlined style={{ fontSize: 32 }} />,
  },
  {
    type: 'accelerometer',
    nameKey: 'accelerometer',
    descKey: 'accelerometerDesc',
    icon: <ControlOutlined style={{ fontSize: 32 }} />,
  },
];

const WidgetSelector: React.FC<WidgetSelectorProps> = ({
  open,
  onClose,
  onSelect,
}) => {
  const { t } = useTranslation('dashboard');
  const { token } = theme.useToken();
  const [hoveredType, setHoveredType] = React.useState<WidgetType | null>(null);

  const handleSelect = (type: WidgetType) => {
    onSelect(type);
    onClose();
  };

  return (
    <Modal
      title={t('addWidget')}
      open={open}
      onCancel={onClose}
      footer={null}
      width={600}
    >
      <Row gutter={[16, 16]}>
        {widgetTypes.map((widget) => (
          <Col span={8} key={widget.type}>
            <Card
              hoverable
              onMouseEnter={() => setHoveredType(widget.type)}
              onMouseLeave={() => setHoveredType(null)}
              onClick={() => handleSelect(widget.type)}
              style={{
                textAlign: 'center',
                cursor: 'pointer',
                borderColor:
                  hoveredType === widget.type
                    ? token.colorPrimary
                    : token.colorBorderSecondary,
                borderWidth: hoveredType === widget.type ? 2 : 1,
                transition: 'all 0.2s',
                background:
                  hoveredType === widget.type
                    ? token.colorPrimaryBg
                    : token.colorBgContainer,
              }}
              styles={{ body: { padding: 16 } }}
            >
              <div
                style={{
                  color:
                    hoveredType === widget.type
                      ? token.colorPrimary
                      : token.colorTextSecondary,
                  marginBottom: 8,
                }}
              >
                {widget.icon}
              </div>
              <Typography.Text
                strong
                style={{
                  display: 'block',
                  color:
                    hoveredType === widget.type
                      ? token.colorPrimary
                      : token.colorText,
                }}
              >
                {t(`widgetTypes.${widget.nameKey}`)}
              </Typography.Text>
              <Typography.Text
                type="secondary"
                style={{ fontSize: 12, display: 'block', marginTop: 4 }}
              >
                {t(`widgetTypes.${widget.descKey}`)}
              </Typography.Text>
            </Card>
          </Col>
        ))}
      </Row>
    </Modal>
  );
};

export default WidgetSelector;
