import React, { useState } from 'react';
import {
  Button,
  Card,
  Divider,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Switch,
  Tag,
  Tooltip,
  Typography,
  message,
} from 'antd';
import { open } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';
import { gh3220Api } from '../../../api/gh3220';
import { useGh3220CommandConfig } from '../../../stores/gh3220CommandConfig';
import { formatErrorMessage } from '../../../utils/errorMessage';
import { encodeBoundedNumber } from '../../../constants/gh3220CommandSchema';
import {
  boundedOptions,
  chipActionOptions,
  customOption,
  customValue,
  encodeEventMask,
  encodeFunctionMask,
  eventMaskOptions,
  formatEventMaskPreview,
  formatFunctionMaskPreview,
  formatWorkModeLabel,
  functionOptions,
  GH3220_WORK_MODE_LIST,
  isCustom,
  resolveGh3220Lang,
  type Gh3220Option,
} from '../../../constants/gh3220Functions';

/** 调试页分区②：采样命令控制（V3：参数全部下拉/多选，功能选择与配置页共享） */
const ControlSection: React.FC = () => {
  const { t, i18n } = useTranslation('gh3220');
  const lang = resolveGh3220Lang(i18n.language);
  const cfg = useGh3220CommandConfig();

  const [fifoThr, setFifoThr] = useState('0');
  const [regAddr, setRegAddr] = useState('0x0000');
  const [regCount, setRegCount] = useState('1');
  const [regValues, setRegValues] = useState('');
  const [regRead, setRegRead] = useState<number[]>([]);

  const mask = encodeFunctionMask(cfg.selectedFunctions);
  const eventMask = encodeEventMask(cfg.eventMaskKeys);

  const codeText = (code: number) => {
    if (code === 0) return t('common.ok');
    if (code === 1) return t('common.fail');
    if (code === 0xff) return t('common.unsupported');
    return `0x${code.toString(16)}`;
  };

  const run = async (label: string, action: () => Promise<number>) => {
    try {
      const code = await action();
      message.success(`${label}: ${codeText(code)}`);
    } catch (error) {
      message.error(formatErrorMessage(error, label));
    }
  };

  const confirmRun = (
    title: string,
    content: string,
    label: string,
    action: () => Promise<number>
  ) => {
    Modal.confirm({
      title,
      content,
      okText: t('commands.confirm'),
      cancelText: t('commands.cancel'),
      okButtonProps: { danger: true },
      onOk: () => void run(label, action),
    });
  };

  /** 数值档位下拉 + 自定义值编辑 */
  const numericSelect = (
    value: string,
    options: Gh3220Option[],
    onChange: (next: string) => void,
    min = 0,
    max = 255,
    width = 120
  ) => (
    <>
      <Select
        size="small"
        style={{ width }}
        value={isCustom(value) ? 'custom' : value}
        options={options}
        onChange={(next: string) =>
          onChange(next === 'custom' ? customOption(customValue(value) || '0') : next)
        }
      />
      {isCustom(value) ? (
        <InputNumber
          size="small"
          min={min}
          max={max}
          value={Number(customValue(value, '0'))}
          onChange={(next) => onChange(customOption(next ?? 0))}
          style={{ width: 96 }}
        />
      ) : null}
    </>
  );

  const numberOrZero = (value: string) => encodeBoundedNumber(value) ?? 0;

  const handleWorkMode = () =>
    void run(t('control.workMode'), () =>
      gh3220Api.setWorkMode(Number(cfg.workMode), mask)
    );

  const handleStart = () =>
    void run(t('control.start'), () => gh3220Api.start(mask, numberOrZero(cfg.slotEn)));

  const handleStop = () =>
    confirmRun(
      t('commands.confirmTitles.stop'),
      t('commands.confirmContents.stop'),
      t('control.stop'),
      () => gh3220Api.stop(mask)
    );

  const handleSwitch = () =>
    void run(t('control.switchFunc'), () =>
      gh3220Api.switchSlotFunc(
        numberOrZero(cfg.slot),
        numberOrZero(cfg.extra),
        mask,
        cfg.slotOn === 'true'
      )
    );

  const handleChip = () => {
    const action = () => gh3220Api.chipCtrl(cfg.chipAction);
    if (cfg.chipAction.includes('reset')) {
      confirmRun(
        t('commands.confirmTitles.chipReset'),
        t('commands.confirmContents.chipReset'),
        t('control.chipAction'),
        action
      );
    } else {
      void run(t('control.chipAction'), action);
    }
  };

  const parseNumber = (text: string): number | null => {
    const parsed = /^0x[0-9a-f]+$/i.test(text.trim())
      ? Number.parseInt(text.trim().slice(2), 16)
      : Number(text);
    if (Number.isNaN(parsed)) {
      message.error(t('control.invalidNumber', { value: text }));
      return null;
    }
    return parsed;
  };

  const handleRegRead = async () => {
    const addr = parseNumber(regAddr);
    if (addr === null) return;
    try {
      const values = await gh3220Api.regRead(addr, Number(regCount));
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
      .map((part) => parseNumber(part))
      .filter((value): value is number => value !== null);
    if (values.length === 0) {
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

  const workModes: Gh3220Option[] = GH3220_WORK_MODE_LIST.map((info) => ({
    value: String(info.mode),
    label: formatWorkModeLabel(info.mode, lang),
  }));

  return (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
      <Card title={t('control.samplingTitle')}>
        <Space wrap size="middle">
          <Space>
            <Typography.Text>{t('control.mode')}</Typography.Text>
            <Select
              size="small"
              style={{ width: 240 }}
              value={cfg.workMode}
              onChange={(next: string) => cfg.setConfig({ workMode: next })}
              options={workModes}
            />
          </Space>
          <Space>
            <Typography.Text>{t('control.funcMask')}</Typography.Text>
            <Select
              size="small"
              mode="multiple"
              maxTagCount={2}
              style={{ minWidth: 220 }}
              value={cfg.selectedFunctions}
              onChange={(keys: string[]) => cfg.setSelectedFunctions(keys)}
              options={functionOptions(lang)}
              placeholder={t('commands.preview.selectFunctions')}
            />
            <Tooltip title={formatFunctionMaskPreview(mask, lang)}>
              <Tag color="blue">{formatFunctionMaskPreview(mask, lang)}</Tag>
            </Tooltip>
          </Space>
          <Space>
            <Typography.Text>{t('control.slotEn')}</Typography.Text>
            {numericSelect(cfg.slotEn, boundedOptions('slotEn', lang), (next) =>
              cfg.setConfig({ slotEn: next })
            )}
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
            {numericSelect(cfg.slot, boundedOptions('slot', lang), (next) =>
              cfg.setConfig({ slot: next })
            )}
          </Space>
          <Space>
            <Typography.Text>{t('control.extra')}</Typography.Text>
            {numericSelect(cfg.extra, boundedOptions('extra', lang), (next) =>
              cfg.setConfig({ extra: next })
            )}
          </Space>
          <Space>
            <Typography.Text>{t('control.enable')}</Typography.Text>
            <Switch
              checked={cfg.slotOn === 'true'}
              onChange={(checked) => cfg.setConfig({ slotOn: String(checked) })}
            />
          </Space>
          <Button onClick={handleSwitch}>{t('control.switchFunc')}</Button>
        </Space>
      </Card>

      <Card title={t('control.chipTitle')}>
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Space wrap size="middle">
            <Space>
              <Typography.Text>{t('control.chipAction')}</Typography.Text>
              <Select
                size="small"
                style={{ width: 160 }}
                value={cfg.chipAction}
                onChange={(next: string) => cfg.setConfig({ chipAction: next })}
                options={chipActionOptions(lang)}
              />
              <Button onClick={handleChip}>{t('control.apply')}</Button>
            </Space>
            <Space>
              <Typography.Text>{t('control.fifoThr')}</Typography.Text>
              {numericSelect(fifoThr, boundedOptions('fifoThr', lang), setFifoThr, 0, 65535)}
              <Button
                onClick={() =>
                  void run(t('control.fifoThr'), () =>
                    gh3220Api.setFifoThr(numberOrZero(fifoThr))
                  )
                }
              >
                {t('control.apply')}
              </Button>
            </Space>
          </Space>
          <Space wrap size="middle">
            <Typography.Text>{t('control.eventMask')}</Typography.Text>
            <Select
              size="small"
              mode="multiple"
              maxTagCount={2}
              style={{ minWidth: 260 }}
              value={cfg.eventMaskKeys}
              onChange={(keys: string[]) => cfg.setEventMaskKeys(keys)}
              options={eventMaskOptions(lang)}
              placeholder={t('commands.preview.selectEvents')}
            />
            <Tag color="blue">{formatEventMaskPreview(eventMask, lang)}</Tag>
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
            {numericSelect(regCount, boundedOptions('count', lang), setRegCount, 1, 64)}
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
