import React from 'react';
import { Button, Modal, Space, Steps, Typography } from 'antd';
import {
  FolderOpenOutlined,
  LineChartOutlined,
  QuestionCircleOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';

const { Text, Title } = Typography;

const bulletListStyle: React.CSSProperties = {
  margin: '4px 0 0',
  paddingLeft: 18,
  color: 'var(--text-secondary)',
  fontSize: 12,
  lineHeight: 1.9,
};

const numberBadgeStyle: React.CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: 20,
  height: 20,
  borderRadius: '50%',
  background: 'var(--primary-color)',
  color: '#fff',
  fontSize: 12,
  flexShrink: 0,
};

const stepCardStyle: React.CSSProperties = {
  flex: '1 1 200px',
  maxWidth: 240,
  padding: '12px 16px',
  borderRadius: 8,
  background: 'var(--bg-secondary)',
  textAlign: 'left',
};

interface CsvUsageGuideModalProps {
  open: boolean;
  onClose: () => void;
}

/** CSV 加载页的完整使用引导弹窗 */
export const CsvUsageGuideModal: React.FC<CsvUsageGuideModalProps> = ({ open, onClose }) => {
  const { t } = useTranslation('waveform');

  return (
    <Modal
      open={open}
      onCancel={onClose}
      centered
      width={640}
      title={t('csvLoader.guide.title')}
      footer={
        <Button type="primary" onClick={onClose}>
          {t('csvLoader.guide.gotIt')}
        </Button>
      }
    >
      <Steps
        orientation="vertical"
        current={-1}
        items={[
          {
            title: t('csvLoader.guide.loadTitle'),
            icon: <FolderOpenOutlined />,
            content: (
              <ul style={bulletListStyle}>
                <li>{t('csvLoader.guide.loadPoint1')}</li>
                <li>{t('csvLoader.guide.loadPoint2')}</li>
                <li>{t('csvLoader.guide.loadPoint3')}</li>
              </ul>
            ),
          },
          {
            title: t('csvLoader.guide.configTitle'),
            icon: <SettingOutlined />,
            content: (
              <ul style={bulletListStyle}>
                <li>{t('csvLoader.guide.configPoint1')}</li>
                <li>{t('csvLoader.guide.configPoint2')}</li>
                <li>{t('csvLoader.guide.configPoint3')}</li>
                <li>{t('csvLoader.guide.configPoint4')}</li>
              </ul>
            ),
          },
          {
            title: t('csvLoader.guide.channelTitle'),
            icon: <LineChartOutlined />,
            content: (
              <ul style={bulletListStyle}>
                <li>{t('csvLoader.guide.channelPoint1')}</li>
                <li>{t('csvLoader.guide.channelPoint2')}</li>
                <li>{t('csvLoader.guide.channelPoint3')}</li>
                <li>{t('csvLoader.guide.channelPoint4')}</li>
              </ul>
            ),
          },
        ]}
      />
    </Modal>
  );
};

interface CsvEmptyGuideProps {
  onSelectFile: () => void;
  onOpenGuide: () => void;
}

/** 未加载 CSV 时的三步上手引导 */
export const CsvEmptyGuide: React.FC<CsvEmptyGuideProps> = ({ onSelectFile, onOpenGuide }) => {
  const { t } = useTranslation('waveform');

  return (
    <div
      style={{
        height: '100%',
        overflow: 'auto',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 24,
        textAlign: 'center',
      }}
    >
      <QuestionCircleOutlined
        style={{ fontSize: 44, marginBottom: 16, color: 'var(--primary-color)' }}
      />
      <Title level={5} style={{ margin: '0 0 4px' }}>
        {t('csvLoader.guide.emptyTitle')}
      </Title>
      <Text type="secondary" style={{ marginBottom: 20 }}>
        {t('csvLoader.guide.emptySubtitle')}
      </Text>

      <div
        style={{
          display: 'flex',
          flexWrap: 'wrap',
          justifyContent: 'center',
          gap: 12,
          width: '100%',
          maxWidth: 780,
          marginBottom: 20,
        }}
      >
        {[1, 2, 3].map((index) => (
          <div key={index} style={stepCardStyle}>
            <Space size={8} style={{ marginBottom: 6 }}>
              <span style={numberBadgeStyle}>{index}</span>
              <Text strong>{t(`csvLoader.guide.step${index}Title`)}</Text>
            </Space>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t(`csvLoader.guide.step${index}Desc`)}
              </Text>
            </div>
          </div>
        ))}
      </div>

      <Space wrap>
        <Button type="primary" icon={<FolderOpenOutlined />} onClick={onSelectFile}>
          {t('csvLoader.guide.emptyAction')}
        </Button>
        <Button type="link" onClick={onOpenGuide}>
          {t('csvLoader.guide.detailAction')}
        </Button>
      </Space>
    </div>
  );
};
