import React, { useState } from 'react';
import { Button, Input, Select, Typography, message, Row, Col, theme } from 'antd';
import { PlayCircleOutlined, ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';

const { Text } = Typography;
const { useToken } = theme;

const Gh3036RpcList: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { token } = useToken();
  const { executeRpc, txChannel } = useGh3036Store();

  const [workMode, setWorkMode] = useState<string>('0');
  const [version, setVersion] = useState<string>(t('gh3036.versionNone'));
  const [command, setCommand] = useState<string>('idle');
  const [writeRegAddr, setWriteRegAddr] = useState<string>('0000');
  const [writeRegValue, setWriteRegValue] = useState<string>('0000');
  const [readRegAddr, setReadRegAddr] = useState<string>('0000');
  const [readRegValue, setReadRegValue] = useState<string>('0000');
  const [configPath, setConfigPath] = useState<string>('');
  const [selectedFunctions, setSelectedFunctions] = useState<string[]>(['adt', 'hr', 'hrv', 'hsm', 'fpbp']);
  const [extendedCmd, setExtendedCmd] = useState<string>('');
  const [isRunning, setIsRunning] = useState<boolean>(false);

  const workModeOptions = [
    { value: '0', label: t('gh3036.workModes.mcuOnline') },
    { value: '1', label: t('gh3036.workModes.mcuOffline') },
    { value: '2', label: t('gh3036.workModes.mptTest') },
  ];

  const commandOptions = [
    { value: 'idle', label: t('gh3036.commands.idle') },
    { value: 'reset', label: t('gh3036.commands.reset') },
    { value: 'sleep', label: t('gh3036.commands.sleep') },
    { value: 'wakeup', label: t('gh3036.commands.wakeup') },
  ];

  const functionOptions = [
    { value: 'adt', label: t('gh3036.functions.adt') },
    { value: 'hr', label: t('gh3036.functions.hr') },
    { value: 'spo2', label: t('gh3036.functions.spo2') },
    { value: 'hrv', label: t('gh3036.functions.hrv') },
    { value: 'hsm', label: t('gh3036.functions.hsm') },
    { value: 'fpbp', label: t('gh3036.functions.fpbp') },
  ];

  const handleExecuteRpc = async (commandKey: string, params: string[]) => {
    if (!txChannel) {
      message.error(t('gh3036.noTxChannel'));
      return false;
    }
    const success = await executeRpc(commandKey, params);
    if (success) {
      message.success(t('gh3036.commandSent', { name: commandKey }));
    }
    return success;
  };

  const handleRestart = async () => {
    await handleExecuteRpc('M', [workMode]);
  };

  const handleGetVersion = async () => {
    const result = await executeRpc('V', ['1']);
    if (result && result.length > 0) {
      const versionStr = String.fromCharCode(...result.filter(c => c >= 32 && c < 127));
      setVersion(versionStr || '-');
    }
  };

  const handleSendCommand = async () => {
    const ctrlTypeMap: Record<string, string> = {
      idle: '0',
      reset: '194',
      sleep: '196',
      wakeup: '195',
    };
    await handleExecuteRpc('C', [ctrlTypeMap[command] || '0']);
  };

  const handleWriteReg = async () => {
    const addr = parseInt(writeRegAddr, 16);
    const value = parseInt(writeRegValue, 16);
    await handleExecuteRpc('W', [addr.toString(), value.toString()]);
  };

  const handleReadReg = async () => {
    const addr = parseInt(readRegAddr, 16);
    const result = await executeRpc('R', [addr.toString(), '1']);
    if (result && result.length >= 2) {
      const value = (result[1] << 8) | result[0];
      setReadRegValue(value.toString(16).toUpperCase().padStart(4, '0'));
      message.success(`寄存器 0x${readRegAddr.toUpperCase()} = 0x${value.toString(16).toUpperCase().padStart(4, '0')}`);
    }
  };

  const handleLoadConfig = async () => {
    if (!configPath.trim()) {
      message.error(t('gh3036.configPathPlaceholder'));
      return;
    }
    await handleExecuteRpc('D', ['0']);
  };

  const handleStartFunction = async () => {
    if (selectedFunctions.length === 0) {
      message.error(t('gh3036.functionSelectPlaceholder'));
      return;
    }
    const funcBits = selectedFunctions.reduce((acc, func) => {
      const bitMap: Record<string, number> = {
        adt: 1,
        hr: 2,
        spo2: 4,
        hrv: 8,
        hsm: 16,
        fpbp: 32,
      };
      return acc | (bitMap[func] || 0);
    }, 0);
    
    const success = await handleExecuteRpc('S', [funcBits.toString(), '0']);
    if (success) {
      setIsRunning(true);
    }
  };

  const handleStopFunction = async () => {
    const success = await handleExecuteRpc('S', ['0', '1']);
    if (success) {
      setIsRunning(false);
    }
  };

  const handleSendExtended = async () => {
    if (!extendedCmd.trim()) {
      message.error(t('gh3036.extendedCommandPlaceholder'));
      return;
    }
    const parts = extendedCmd.split(',').map(s => s.trim());
    if (parts.length > 0) {
      const cmdKey = parts[0];
      const params = parts.slice(1);
      await handleExecuteRpc(cmdKey, params);
    }
  };

  const rowStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    padding: '8px 0',
    borderBottom: `1px solid ${token.colorBorderSecondary}`,
  };

  const labelStyle: React.CSSProperties = {
    width: 80,
    flexShrink: 0,
    fontSize: 13,
    color: token.colorTextSecondary,
  };

  const controlStyle: React.CSSProperties = {
    flex: 1,
    display: 'flex',
    alignItems: 'center',
    gap: 8,
  };

  const buttonStyle: React.CSSProperties = {
    width: 90,
    flexShrink: 0,
  };

  const inputGroupStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: 4,
    flex: 1,
  };

  const smallInputStyle: React.CSSProperties = {
    width: 100,
  };

  return (
    <div style={{ background: token.colorBgContainer }}>
      <Row gutter={[0, 0]}>
        <Col span={24}>
          <div style={rowStyle}>
            <Text style={labelStyle}>{t('gh3036.modeSelect')}</Text>
            <div style={controlStyle}>
              <Select
                value={workMode}
                onChange={setWorkMode}
                options={workModeOptions}
                style={{ flex: 1 }}
                size="small"
              />
            </div>
            <Button
              size="small"
              icon={<ReloadOutlined />}
              onClick={handleRestart}
              style={buttonStyle}
            >
              {t('gh3036.restart')}
            </Button>
          </div>
        </Col>

        <Col span={24}>
          <div style={rowStyle}>
            <Text style={labelStyle}>{t('gh3036.versionGet')}</Text>
            <div style={controlStyle}>
              <Text style={{ fontSize: 13 }}>{version}</Text>
            </div>
            <Button
              size="small"
              onClick={handleGetVersion}
              style={buttonStyle}
            >
              {t('gh3036.getVersion')}
            </Button>
          </div>
        </Col>

        <Col span={24}>
          <div style={rowStyle}>
            <Text style={labelStyle}>{t('gh3036.sendCommand')}</Text>
            <div style={controlStyle}>
              <Select
                value={command}
                onChange={setCommand}
                options={commandOptions}
                style={{ flex: 1 }}
                size="small"
              />
            </div>
            <Button
              size="small"
              onClick={handleSendCommand}
              style={buttonStyle}
            >
              {t('gh3036.sendCommand')}
            </Button>
          </div>
        </Col>

        <Col span={24}>
          <div style={rowStyle}>
            <Text style={labelStyle}>{t('gh3036.writeRegister')}</Text>
            <div style={controlStyle}>
              <div style={inputGroupStyle}>
                <Text style={{ fontSize: 11, color: token.colorTextTertiary }}>{t('gh3036.regAddr')}:</Text>
                <Input
                  size="small"
                  value={writeRegAddr}
                  onChange={(e) => setWriteRegAddr(e.target.value)}
                  style={smallInputStyle}
                  maxLength={4}
                />
                <Text style={{ fontSize: 11, color: token.colorTextTertiary }}>{t('gh3036.regValue')}:</Text>
                <Input
                  size="small"
                  value={writeRegValue}
                  onChange={(e) => setWriteRegValue(e.target.value)}
                  style={smallInputStyle}
                  maxLength={4}
                />
              </div>
            </div>
            <Button
              size="small"
              onClick={handleWriteReg}
              style={buttonStyle}
            >
              {t('gh3036.writeReg')}
            </Button>
          </div>
        </Col>

        <Col span={24}>
          <div style={rowStyle}>
            <Text style={labelStyle}>{t('gh3036.readRegister')}</Text>
            <div style={controlStyle}>
              <div style={inputGroupStyle}>
                <Text style={{ fontSize: 11, color: token.colorTextTertiary }}>{t('gh3036.regAddr')}:</Text>
                <Input
                  size="small"
                  value={readRegAddr}
                  onChange={(e) => setReadRegAddr(e.target.value)}
                  style={smallInputStyle}
                  maxLength={4}
                />
                <Text style={{ fontSize: 11, color: token.colorTextTertiary }}>{t('gh3036.regValue')}:</Text>
                <Input
                  size="small"
                  value={readRegValue}
                  readOnly
                  style={{ ...smallInputStyle, background: token.colorFillSecondary }}
                  maxLength={4}
                />
              </div>
            </div>
            <Button
              size="small"
              onClick={handleReadReg}
              style={buttonStyle}
            >
              {t('gh3036.readReg')}
            </Button>
          </div>
        </Col>

        <Col span={24}>
          <div style={rowStyle}>
            <Text style={labelStyle}>{t('gh3036.configLoad')}</Text>
            <div style={controlStyle}>
              <Input
                size="small"
                placeholder={t('gh3036.configPathPlaceholder')}
                value={configPath}
                onChange={(e) => setConfigPath(e.target.value)}
                style={{ flex: 1 }}
              />
            </div>
            <Button
              size="small"
              onClick={handleLoadConfig}
              style={buttonStyle}
            >
              {t('gh3036.loadConfig')}
            </Button>
          </div>
        </Col>

        <Col span={24}>
          <div style={rowStyle}>
            <Text style={labelStyle}>{t('gh3036.functionSelect')}</Text>
            <div style={controlStyle}>
              <Select
                mode="multiple"
                value={selectedFunctions}
                onChange={setSelectedFunctions}
                options={functionOptions}
                style={{ flex: 1 }}
                size="small"
                maxTagCount={3}
              />
            </div>
            <Button
              type={isRunning ? 'default' : 'primary'}
              size="small"
              icon={<PlayCircleOutlined />}
              onClick={isRunning ? handleStopFunction : handleStartFunction}
              style={buttonStyle}
              danger={isRunning}
            >
              {isRunning ? t('gh3036.stop') : t('gh3036.start')}
            </Button>
          </div>
        </Col>

        <Col span={24}>
          <div style={{ ...rowStyle, borderBottom: 'none' }}>
            <Text style={labelStyle}>{t('gh3036.extendedCommand')}</Text>
            <div style={controlStyle}>
              <Input
                size="small"
                placeholder={t('gh3036.extendedCommandPlaceholder')}
                value={extendedCmd}
                onChange={(e) => setExtendedCmd(e.target.value)}
                style={{ flex: 1 }}
              />
            </div>
            <Button
              size="small"
              onClick={handleSendExtended}
              style={buttonStyle}
            >
              {t('gh3036.send')}
            </Button>
          </div>
        </Col>
      </Row>
    </div>
  );
};

export default Gh3036RpcList;
