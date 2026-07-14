import React, { useState, useEffect } from 'react';
import { Button, Input, Select, Typography, message, Row, Col, theme, Modal, Table } from 'antd';
import { PlayCircleOutlined, ReloadOutlined, FolderOpenOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { gh3036Api } from '../../api/gh3036';
import { open } from '@tauri-apps/plugin-dialog';
import type { Gh3036ConfigRegisterPreview, Gh3036VersionTypeConfig } from '../../api/types';
import { formatErrorMessage } from '../../utils/errorMessage';

const { Text } = Typography;
const { useToken } = theme;

const Gh3036RpcList: React.FC = () => {
  const { t } = useTranslation('protocol');
  const { token } = useToken();
  const { executeRpc, txChannel, rpcConfig, setRpcConfig } = useGh3036Store();
  const [extendedCmd, setExtendedCmd] = useState<string>('');
  const [versionTypes, setVersionTypes] = useState<Gh3036VersionTypeConfig[]>([]);

  useEffect(() => {
    const loadVersionTypes = async () => {
      try {
        const types = await gh3036Api.getVersionTypes();
        setVersionTypes(types);
      } catch (err) {
        console.error('加载版本类型失败:', err);
      }
    };
    loadVersionTypes();
  }, []);

  const versionTypeOptions = versionTypes.map(vt => ({
    value: vt.type_value,
    label: vt.name,
  }));

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
    { value: 'gnadt', label: t('gh3036.functions.gnadt') },
    { value: 'irnadt', label: t('gh3036.functions.irnadt') },
    { value: 'test1', label: t('gh3036.functions.test1') },
    { value: 'test2', label: t('gh3036.functions.test2') },
    { value: 'slot', label: t('gh3036.functions.slot') },
  ];

  const factoryModeOptions = [
    { value: 'chipInit', label: t('gh3036.factoryModes.chipInit') },
    { value: 'chipUid', label: t('gh3036.factoryModes.chipUid') },
    { value: 'baseNoise', label: t('gh3036.factoryModes.baseNoise') },
    { value: 'ppgNoise', label: t('gh3036.factoryModes.ppgNoise') },
    { value: 'lpctr', label: t('gh3036.factoryModes.lpctr') },
    { value: 'lplctr', label: t('gh3036.factoryModes.lplctr') },
  ];

  const bitMap: Record<string, number> = {
        adt: 0x01,
        hr: 0x02,
        spo2: 0x04,
        hrv: 0x08,
        gnadt: 0x10,
        irnadt: 0x20,
        test1: 0x40,
        test2: 0x80,
        slot: 0x100,
      };

  const handleExecuteRpc = async (commandKey: string, params: string[]) => {
    if (!txChannel) {
      message.error(t('gh3036.noTxChannel'));
      return false;
    }
    const result = await executeRpc(commandKey, params);
    const success = result !== null;
    if (success) {
      message.success(t('gh3036.commandSent', { name: commandKey }));
    }
    return success;
  };

  const handleRestart = async () => {
    await handleExecuteRpc('M', [rpcConfig.workMode]);
  };

  const handleGetVersion = async () => {
    const result = await executeRpc('V', [rpcConfig.versionType.toString()]);
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
    await handleExecuteRpc('W', ['0x' + addr.toString(16), '0x' + value.toString(16)]);
  };

  const handleReadReg = async () => {
    const addr = parseInt(rpcConfig.readRegAddr, 16);
    const result = await executeRpc('R', ['0x' + addr.toString(16), '1']);
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
      const preview = await gh3036Api.loadConfigFile(pathStr);
      const fileName = pathStr.split(/[\\/]/).pop() || pathStr;
      const dataSource = preview.registers.map((reg, index) => ({
        ...reg,
        key: `${reg.addr}-${index}`,
        index: index + 1,
      }));

      Modal.confirm({
        title: t('gh3036.configLoad'),
        width: 680,
        okText: t('gh3036.loadConfig'),
        cancelText: '取消',
        content: (
          <div style={{ marginTop: 12 }}>
            <div style={{ marginBottom: 12, lineHeight: 1.8 }}>
              <div>
                <Text strong>配置文件：</Text>
                <Text>{fileName}</Text>
              </div>
              <div>
                <Text strong>寄存器数量：</Text>
                <Text>{preview.registerCount}</Text>
              </div>
            </div>
            <Table<Gh3036ConfigRegisterPreview & { key: string; index: number }>
              size="small"
              pagination={false}
              scroll={{ y: 360 }}
              dataSource={dataSource}
              columns={[
                {
                  title: '#',
                  dataIndex: 'index',
                  width: 64,
                },
                {
                  title: '地址',
                  dataIndex: 'addr',
                  width: 160,
                },
                {
                  title: '值',
                  dataIndex: 'value',
                  width: 160,
                },
              ]}
            />
          </div>
        ),
        onOk: async () => {
          try {
            if (!txChannel) {
              throw new Error(t('gh3036.noTxChannel'));
            }

            await gh3036Api.downloadConfigFile(pathStr);
            message.success(`配置下载完成，共 ${preview.registerCount} 个寄存器`);
          } catch (err) {
            message.error(formatErrorMessage(err, t('gh3036.configDownloadFailed')));
            throw err;
          }
        },
      });
    } catch (err) {
      message.error(formatErrorMessage(err, t('gh3036.configParseFailed')));
    }
  };

  const handleStartFunction = async () => {
    if (rpcConfig.selectedFunctions.length === 0) {
      message.error(t('gh3036.functionSelectPlaceholder'));
      return;
    }
    const funcBits = rpcConfig.selectedFunctions.reduce((acc, func) => {
      return acc | (bitMap[func] || 0);
    }, 0);
    
    const success = await handleExecuteRpc('S', [funcBits.toString(), '0']);
    if (success) {
      setRpcConfig({ isRunning: true });
    }
  };

  const handleStopFunction = async () => {
    if (rpcConfig.selectedFunctions.length === 0) {
      message.error(t('gh3036.functionSelectPlaceholder'));
      return;
    }
    const funcBits = rpcConfig.selectedFunctions.reduce((acc, func) => {
      return acc | (bitMap[func] || 0);
    }, 0);
    const success = await handleExecuteRpc('S', [funcBits.toString(), '1']);
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
      chipInit: 0x01,
      chipUid: 0x02,
      baseNoise: 0x04,
      ppgNoise: 0x08,
      lpctr: 0x10,
      lplctr: 0x20,
    };
    const modeBits = bitMap[rpcConfig.factoryMode] || 0;
    await handleExecuteRpc('FS', ['0x' + modeBits.toString(16)]);
  };

  const handleFactoryGetMode = async () => {
    if (!rpcConfig.factoryMode) {
      message.error(t('gh3036.factoryModeSelectPlaceholder'));
      return;
    }
    const bitMap: Record<string, number> = {
      chipInit: 0x01,
      chipUid: 0x02,
      baseNoise: 0x04,
      ppgNoise: 0x08,
      lpctr: 0x10,
      lplctr: 0x20,
    };
    const modeBits = bitMap[rpcConfig.factoryMode] || 0;
    const result = await executeRpc('FG', ['0x' + modeBits.toString(16)]);
    if (result === null) {
      return;
    }
    const values: string[] = [];
    for (let i = 0; i < result.length; i += 2) {
      if (i + 1 < result.length) {
        const value = (result[i + 1] << 8) | result[i];
        values.push(`0x${value.toString(16).toUpperCase().padStart(4, '0')}`);
      }
    }
    const displayValue = values.length > 1 ? values.join(', ') : values[0] || '-';
    setRpcConfig({ factoryResult: displayValue });
    message.success(t('gh3036.factoryResultSuccess', { value: displayValue }));
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
              <Select
                value={rpcConfig.versionType}
                onChange={(value) => setRpcConfig({ versionType: value })}
                options={versionTypeOptions}
                style={{ width: 150, marginRight: 8 }}
                size="small"
              />
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
