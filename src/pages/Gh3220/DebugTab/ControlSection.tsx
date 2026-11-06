import React, { useState } from 'react';
import {
  Button,
  Card,
  Divider,
  Input,
  InputNumber,
  Select,
  Space,
  Switch,
  Typography,
  message,
} from 'antd';
import { open } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';
import { gh3220Api } from '../../../api/gh3220';
import { parseFuncMask } from '../../../types/gh3220';
import { formatErrorMessage } from '../../../utils/errorMessage';

/** 调试页分区②：采样命令控制（原 ControlTab 主体） */
const ControlSection: React.FC = () => {
  const { t } = useTranslation('gh3220');

  const [mode, setMode] = useState(0);
  const [funcMaskText, setFuncMaskText] = useState('hr');
  const [slotEn, setSlotEn] = useState(0);
  const [slot, setSlot] = useState(0);
  const [extra, setExtra] = useState(0);
  const [slotOn, setSlotOn] = useState(true);

  const [chipAction, setChipAction] = useState('wakeup');
  const [fifoThr, setFifoThr] = useState(0);
  const [eventMask, setEventMask] = useState(0);

  const [regAddr, setRegAddr] = useState('0x0000');
  const [regCount, setRegCount] = useState(1);
  const [regValues, setRegValues] = useState('');
  const [regRead, setRegRead] = useState<number[]>([]);

  const codeText = (code: number) => {
    if (code === 0) return t('common.ok');
    if (code === 1) return t('common.fail');
    if (code === 0xff) return t('common.unsupported');
    return `0x${code.toString(16)}`;
  };

  const readMask = (): number | null => {
    try {
      return parseFuncMask(funcMaskText);
    } catch (error) {
      message.error((error as Error).message);
      return null;
    }
  };

  const run = async (label: string, action: () => Promise<number>) => {
    try {
      const code = await action();
      message.success(`${label}: ${codeText(code)}`);
    } catch (error) {
      message.error(formatErrorMessage(error, label));
    }
  };

  const handleWorkMode = () => {
    const mask = readMask();
    if (mask === null) return;
    void run(t('control.workMode'), () => gh3220Api.setWorkMode(mode, mask));
  };

  const handleStart = () => {
    const mask = readMask();
    if (mask === null) return;
    void run(t('control.start'), () => gh3220Api.start(mask, slotEn));
  };

  const handleStop = () => {
    const mask = readMask();
    if (mask === null) return;
    void run(t('control.stop'), () => gh3220Api.stop(mask));
  };

  const handleSwitch = () => {
    const mask = readMask();
    if (mask === null) return;
    void run(t('control.switchFunc'), () =>
      gh3220Api.switchSlotFunc(slot, extra, mask, slotOn)
    );
  };

  const parseNumber = (text: string): number | null => {
    const value = Number(text);
    if (Number.isNaN(value)) {
      message.error(t('control.invalidNumber', { value: text }));
      return null;
    }
    return value;
  };

  const handleRegRead = async () => {
    const addr = parseNumber(regAddr);
    if (addr === null) return;
    try {
      const values = await gh3220Api.regRead(addr, regCount);
      setRegRead(values);
      message.success(
        `${t('control.regRead')}: ${values.map((v) => `0x${v.toString(16)}`).join(', ')}`
      );
    } catch (error) {
      message.error(formatErrorMessage(error, t('control.regRead')));
    }
  };

  const handleRegWrite = async () => {
    const addr = parseNumber(regAddr);
    if (addr === null) return;
    const values = regValues
      .split(/[,\s]+/)
      .filter(Boolean)
      .map((part) => Number(part));
    if (values.length === 0 || values.some((v) => Number.isNaN(v))) {
      message.error(t('control.invalidValues'));
      return;
    }
    try {
      await gh3220Api.regWrite(addr, values);
      message.success(t('control.regWrite'));
    } catch (error) {
      message.error(formatErrorMessage(error, t('control.regWrite')));
    }
  };

  const handleLoadRegList = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Register list', extensions: ['csv', 'txt'] }],
      });
      if (!selected || Array.isArray(selected)) return;
      const code = await gh3220Api.loadRegList(selected);
      message.success(`${t('control.loadRegList')}: ${codeText(code)}`);
    } catch (error) {
      message.error(formatErrorMessage(error, t('control.loadRegList')));
    }
  };

  return (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
      <Card title={t('control.samplingTitle')}>
        <Space wrap size="middle">
          <Space>
            <Typography.Text>{t('control.mode')}</Typography.Text>
            <InputNumber min={0} max={255} value={mode} onChange={(v) => setMode(v ?? 0)} />
          </Space>
          <Space>
            <Typography.Text>{t('control.funcMask')}</Typography.Text>
            <Input
              style={{ width: 180 }}
              value={funcMaskText}
              onChange={(e) => setFuncMaskText(e.target.value)}
              placeholder="hr,spo2 或 0x6"
            />
          </Space>
          <Space>
            <Typography.Text>{t('control.slotEn')}</Typography.Text>
            <InputNumber min={0} max={255} value={slotEn} onChange={(v) => setSlotEn(v ?? 0)} />
          </Space>
        </Space>
        <Divider />
        <Space wrap>
          <Button onClick={handleWorkMode}>{t('control.workMode')}</Button>
          <Button type="primary" onClick={handleStart}>
            {t('control.start')}
          </Button>
          <Button danger onClick={handleStop}>
            {t('control.stop')}
          </Button>
        </Space>
        <Typography.Paragraph type="secondary" style={{ marginTop: 12, marginBottom: 0 }}>
          {t('control.startHint')}
        </Typography.Paragraph>
      </Card>

      <Card title={t('control.switchTitle')}>
        <Space wrap size="middle">
          <Space>
            <Typography.Text>{t('control.slot')}</Typography.Text>
            <InputNumber min={0} max={255} value={slot} onChange={(v) => setSlot(v ?? 0)} />
          </Space>
          <Space>
            <Typography.Text>{t('control.extra')}</Typography.Text>
            <InputNumber min={0} max={255} value={extra} onChange={(v) => setExtra(v ?? 0)} />
          </Space>
          <Space>
            <Typography.Text>{t('control.enable')}</Typography.Text>
            <Switch checked={slotOn} onChange={setSlotOn} />
          </Space>
          <Button onClick={handleSwitch}>{t('control.switchFunc')}</Button>
        </Space>
      </Card>

      <Card title={t('control.chipTitle')}>
        <Space wrap size="middle">
          <Space>
            <Typography.Text>{t('control.chipAction')}</Typography.Text>
            <Select
              style={{ width: 160 }}
              value={chipAction}
              onChange={setChipAction}
              options={[
                { value: 'wakeup', label: 'wakeup' },
                { value: 'sleep', label: 'sleep' },
                { value: 'soft-reset', label: 'soft-reset' },
                { value: 'hard-reset', label: 'hard-reset' },
                { value: 'rx-reset', label: 'rx-reset' },
              ]}
            />
            <Button
              onClick={() => void run(t('control.chipAction'), () => gh3220Api.chipCtrl(chipAction))}
            >
              {t('control.apply')}
            </Button>
          </Space>
          <Space>
            <Typography.Text>{t('control.fifoThr')}</Typography.Text>
            <InputNumber min={0} max={65535} value={fifoThr} onChange={(v) => setFifoThr(v ?? 0)} />
            <Button
              onClick={() => void run(t('control.fifoThr'), () => gh3220Api.setFifoThr(fifoThr))}
            >
              {t('control.apply')}
            </Button>
          </Space>
          <Space>
            <Typography.Text>{t('control.eventMask')}</Typography.Text>
            <InputNumber min={0} max={65535} value={eventMask} onChange={(v) => setEventMask(v ?? 0)} />
            <Button
              onClick={() =>
                void run(t('control.eventMask'), () => gh3220Api.setReportEvent(eventMask))
              }
            >
              {t('control.apply')}
            </Button>
          </Space>
        </Space>
        <Typography.Paragraph type="secondary" style={{ marginTop: 12, marginBottom: 0 }}>
          {t('control.wakeupHint')}
        </Typography.Paragraph>
      </Card>

      <Card title={t('control.regTitle')}>
        <Space wrap size="middle">
          <Space>
            <Typography.Text>{t('control.addr')}</Typography.Text>
            <Input
              style={{ width: 120 }}
              value={regAddr}
              onChange={(e) => setRegAddr(e.target.value)}
              placeholder="0x0000"
            />
          </Space>
          <Space>
            <Typography.Text>{t('control.count')}</Typography.Text>
            <InputNumber min={1} max={64} value={regCount} onChange={(v) => setRegCount(v ?? 1)} />
            <Button onClick={handleRegRead}>{t('control.regRead')}</Button>
          </Space>
        </Space>
        <Divider />
        <Space wrap size="middle">
          <Space>
            <Typography.Text>{t('control.values')}</Typography.Text>
            <Input
              style={{ width: 260 }}
              value={regValues}
              onChange={(e) => setRegValues(e.target.value)}
              placeholder="0x1234, 0xabcd"
            />
            <Button onClick={handleRegWrite}>{t('control.regWrite')}</Button>
          </Space>
          <Button onClick={handleLoadRegList}>{t('control.loadRegList')}</Button>
        </Space>
        <Divider />
        <Typography.Text type="secondary">
          {regRead.length > 0
            ? regRead.map((v, i) => `0x${v.toString(16)}${i < regRead.length - 1 ? ', ' : ''}`)
            : t('control.noRegRead')}
        </Typography.Text>
      </Card>
    </Space>
  );
};

export default ControlSection;
