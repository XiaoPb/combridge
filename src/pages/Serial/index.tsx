import React, { useEffect } from 'react';
import { Row, Col, Alert } from 'antd';
import { useSerial } from '../../hooks/useSerial';
import SerialToolbar from './SerialToolbar';
import SerialDataView from './SerialDataView';
import SerialSendPanel from './SerialSendPanel';
import SerialSettings from './SerialSettings';

const SerialPage: React.FC = () => {
  const {
    ports,
    currentPort,
    config,
    receivedData,
    sentData,
    isScanning,
    error,
    scanPorts,
    openPort,
    closePort,
    sendData,
    clearAllData,
    updatePortConfig,
    setCurrentPort,
    isConnected,
  } = useSerial();

  useEffect(() => {
    scanPorts();
  }, []);

  const handleOpenPort = async () => {
    if (currentPort) {
      await openPort(currentPort, config);
    }
  };

  const handleClosePort = async () => {
    if (currentPort) {
      await closePort(currentPort);
    }
  };

  const handleSendData = async (data: string, format: 'hex' | 'text') => {
    await sendData(data, format);
  };

  const connected = currentPort ? isConnected(currentPort) : false;

  return (
    <div>
      {error && (
        <Alert
          title="错误"
          description={error}
          type="error"
          closable
          style={{ marginBottom: 16 }}
        />
      )}

      <SerialToolbar
        ports={ports}
        currentPort={currentPort}
        config={config}
        isScanning={isScanning}
        isConnected={connected}
        onScan={scanPorts}
        onSelectPort={setCurrentPort}
        onOpen={handleOpenPort}
        onClose={handleClosePort}
        onUpdateConfig={updatePortConfig}
      />

      <Row gutter={16} style={{ marginTop: 16 }}>
        <Col xs={24} lg={18}>
          <SerialDataView
            receivedData={receivedData}
            sentData={sentData}
            onClear={clearAllData}
          />
        </Col>
        <Col xs={24} lg={6}>
          <SerialSettings
            config={config}
            isConnected={connected}
            onUpdateConfig={updatePortConfig}
          />
        </Col>
      </Row>

      <div style={{ marginTop: 16 }}>
        <SerialSendPanel
          isConnected={connected}
          onSend={handleSendData}
        />
      </div>
    </div>
  );
};

export default SerialPage;
