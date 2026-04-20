import React, { useState } from 'react';
import { Button, Input, Select, Typography, message, Row, Col, theme } from 'antd';
import { PlayCircleOutlined, ReloadOutlined, FolderOpenOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { gh3036Api } from '../../api/gh3036';
import { open } from '@tauri-apps/plugin-dialog';

const { Text } = Typography;
const { useToken } = theme;

const Gh3036RpcList: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { token } = useToken();
  const { executeRpc, txChannel, rpcConfig, setRpcConfig } = useGh3036Store();
  const [extendedCmd, setExtendedCmd] = useState<string>('');

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

  const factoryModeOptions = [
    { value: 'chipInit', label: t('gh3036.factoryModes.chipInit') },
    { value: 'chipUid', label: t('gh3036.factoryModes.chipUid') },
    { value: 'baseNoise', label: t('gh3036.factoryModes.baseNoise') },
    { value: 'ppgNoise', label: t('gh3036.factoryModes.ppgNoise') },
    { value: 'lpctr', label: t('gh3036.factoryModes.lpctr') },
    { value: 'lplctr', label: t('gh3036.factoryModes.lplctr') },
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
    await handleExecuteRpc('M', [rpcConfig.workMode]);
  };

  const handleGetVersion = async () => {
    const result = await executeRpc('V', ['1']);
    if (result && result.length > 0) {
      const versionStr = String.fromCharCode(...result.filter(c => c >= 32 && c < 127));
      setRpcConfig({ version: versionStr || '-' });
    }
  };

  const handleSendCommand = async () => {
    const ctrlTypeMap: Record<string, string> = {
      idle: '0',
      reset: '194',
      sleep: '196',
      wakeup: '195',
    };
    await handleExecuteRpc('C', [ctrlTypeMap[rpcConfig.command] || '0']);
  };

  const handleWriteReg = async () => {
    const addr = parseInt(rpcConfig.writeRegAddr, 16);
    const value = parseInt(rpcConfig.writeRegValue, 16);
    await handleExecuteRpc('W', [addr.toString(), value.toString()]);
  };

  const handleReadReg = async () => {
    const addr = parseInt(rpcConfig.readRegAddr, 16);
    const result = await executeRpc('R', [addr.toString(), '1']);
    if (result && result.length >= 2) {
      const value = (result[1] << 8) | result[0];
      setRpcConfig({ readRegValue: value.toString(16).toUpperCase().padStart(4, '0') });
      message.success(`寄存器 0x${rpcConfig.readRegAddr.toUpperCase()} = 0x${value.toString(16).toUpperCase().padStart(4, '0')}`);
    }
  };

  const handleLoadConfig = async () => {
    const filePath = await open({
      filters: [
        { name: 'Config Files', extensions: ['config', 'ini'] },
      ],
    });
    
    if (!filePath) return;
    
    const pathStr = filePath as string;
    setRpcConfig({ configPath: pathStr });
    
    try {
      const regs = await gh3036Api.loadConfigFile(pathStr);
      message.success(t('gh3036.configParsed', { count: regs.length }));
      
      await handleExecuteRpc('L', regs);
    } catch (err) {
      message.error(t('gh3036.configParseFailed', { error: String(err) }));
    }
  };

  const handleStartFunction = async () => {
    if (rpcConfig.selectedFunctions.length === 0) {
      message.error(t('gh3036.functionSelectPlaceholder'));
      return;
    }
    const funcBits = rpcConfig.selectedFunctions.reduce((acc, func) => {
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
      setRpcConfig({ isRunning: true });
    }
  };

  const handleStopFunction = async () => {
    const success = await handleExecuteRpc('S', ['0', '1']);
    if (success) {
      setRpcConfig({ isRunning: false });
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

  const handleFactorySetMode = async () => {
    if (!rpcConfig.factoryMode) {
      message.error(t('gh3036.factoryModeSelectPlaceholder'));
      return;
    }
    const bitMap: Record<string, number> = {
      chipInit: 0x02,
      chipUid: 0x04,
      baseNoise: 0x08,
      ppgNoise: 0x10,
      lpctr: 0x20,
      lplctr: 0x40,
    };
    const modeBits = bitMap[rpcConfig.factoryMode] || 0;
    await handleExecuteRpc('FS', [modeBits.toString()]);
  };

  const handleFactoryGetMode = async () => {
    if (!rpcConfig.factoryMode) {
      message.error(t('gh3036.factoryModeSelectPlaceholder'));
      return;
    }
    const bitMap: Record<string, number> = {
      chipInit: 0x02,
      chipUid: 0x04,
      baseNoise: 0x08,
      ppgNoise: 0x10,
      lpctr: 0x20,
      lplctr: 0x40,
    };
    const modeBits = bitMap[rpcConfig.factoryMode] || 0;
    const result = await executeRpc('FG', [modeBits.toString()]);
    if (result && result.length >= 2) {
      const value = (result[1] << 8) | result[0];
      setRpcConfig({ factoryResult: `0x${value.toString(16).toUpperCase().padStart(4, '0')}` });
      message.success(t('gh3036.factoryResultSuccess', { value: `0x${value.toString(16).toUpperCase().padStart(4, '0')}` }));
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
                value={rpcConfig.workMode}
                onChange={(value) => setRpcConfig({ workMode: value })}
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
              <Text style={{ fontSize: 13 }}>{rpcConfig.version}</Text>
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
                value={rpcConfig.command}
                onChange={(value) => setRpcConfig({ command: value })}
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
                  value={rpcConfig.writeRegAddr}
                  onChange={(e) => setRpcConfig({ writeRegAddr: e.target.value })}
                  style={smallInputStyle}
                  maxLength={4}
                />
                <Text style={{ fontSize: 11, color: token.colorTextTertiary }}>{t('gh3036.regValue')}:</Text>
                <Input
                  size="small"
                  value={rpcConfig.writeRegValue}
                  onChange={(e) => setRpcConfig({ writeRegValue: e.target.value })}
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
                  value={rpcConfig.readRegAddr}
                  onChange={(e) => setRpcConfig({ readRegAddr: e.target.value })}
                  style={smallInputStyle}
                  maxLength={4}
                />
                <Text style={{ fontSize: 11, color: token.colorTextTertiary }}>{t('gh3036.regValue')}:</Text>
                <Input
                  size="small"
                  value={rpcConfig.readRegValue}
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
                value={rpcConfig.configPath}
                readOnly
                style={{ flex: 1, background: token.colorFillSecondary }}
              />
            </div>
            <Button
              size="small"
              icon={<FolderOpenOutlined />}
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
                value={rpcConfig.selectedFunctions}
                onChange={(value) => setRpcConfig({ selectedFunctions: value })}
                options={functionOptions}
                style={{ flex: 1 }}
                size="small"
                maxTagCount={3}
              />
            </div>
            <Button
              type={rpcConfig.isRunning ? 'default' : 'primary'}
              size="small"
              icon={<PlayCircleOutlined />}
              onClick={rpcConfig.isRunning ? handleStopFunction : handleStartFunction}
              style={buttonStyle}
              danger={rpcConfig.isRunning}
            >
              {rpcConfig.isRunning ? t('gh3036.stop') : t('gh3036.start')}
            </Button>
          </div>
        </Col>

        <Col span={24}>
          <div style={rowStyle}>
            <Text style={labelStyle}>{t('gh3036.factoryMode')}</Text>
            <div style={controlStyle}>
              <Select
                value={rpcConfig.factoryMode}
                onChange={(value) => setRpcConfig({ factoryMode: value })}
                options={factoryModeOptions}
                style={{ flex: 1 }}
                size="small"
                placeholder={t('gh3036.factoryModeSelectPlaceholder')}
              />
            </div>
            <Button
              size="small"
              onClick={handleFactorySetMode}
              style={{ ...buttonStyle, marginRight: 4 }}
            >
              {t('gh3036.factorySet')}
            </Button>
          </div>
        </Col>

        <Col span={24}>
          <div style={rowStyle}>
            <Text style={labelStyle}>{t('gh3036.factoryResult')}</Text>
            <div style={controlStyle}>
              <Text style={{ fontSize: 13 }}>{rpcConfig.factoryResult}</Text>
            </div>
            <Button
              size="small"
              onClick={handleFactoryGetMode}
              style={buttonStyle}
            >
              {t('gh3036.factoryGet')}
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
