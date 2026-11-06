import React, { useEffect, useRef, useState } from 'react';
import {
  Alert,
  Button,
  Collapse,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
  message,
  theme,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { SendOutlined, ClearOutlined, ReloadOutlined, FolderOpenOutlined } from '@ant-design/icons';
import { open } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';
import { gh3220Api } from '../../api/gh3220';
import { useGh3220Store } from '../../stores/gh3220Store';
import { useGh3220CommandConfig } from '../../stores/gh3220CommandConfig';
import {
  DEFAULT_COMMAND_VALUES,
  describeCommand,
  encodeBoundedNumber,
  encodeCommand,
  encodeHex,
  GH3220_COMMAND_CATALOG,
  needsConfirm,
  resolveOptions,
  validateCommand,
  type CommandDef,
  type OptionContext,
  type ParamDef,
  type ParamRaw,
} from '../../constants/gh3220CommandSchema';
import {
  customOption,
  customValue,
  encodeFunctionMask,
  formatEventMaskPreview,
  formatFunctionMaskPreview,
  functionOptions,
  GH3220_HEX_TEMPLATES,
  isCustom,
  resolveGh3220Lang,
  workModeOptions,
  channelOptions,
  boundedOptions,
  boolOptions,
  encodeEventMask,
  type Gh3220Lang,
  type Gh3220Option,
} from '../../constants/gh3220Functions';
import type { Gh3220CaptureConfig } from '../../types/gh3220';
import { formatErrorMessage } from '../../utils/errorMessage';

const { Text } = Typography;

/** 组合启动行与参数预览共用的功能位编码器 */
const maskOf = encodeFunctionMask;

interface LogEntry {
  id: number;
  time: string;
  key: string;
  cmd: string;
  semantic: string;
  wire: string;
  response: string;
  ok: boolean;
  durationMs: number;
}

const QUICK_KEYS = ['getDeviceInfo', 'getStatus', 'getStats', 'getCaptureStatus', 'packageTest'];

function formatResponse(value: unknown): string {
  if (value === undefined) return '(void)';
  if (typeof value === 'string') return value;
  if (Array.isArray(value) && value.every((item) => typeof item === 'number')) {
    const numbers = value as number[];
    const hex =
      numbers.length > 0 && numbers.every((n) => n >= 0 && n <= 255)
        ? `\nHEX: ${numbers.map((n) => n.toString(16).padStart(2, '0')).join(' ')}`
        : '';
    return `${JSON.stringify(value)}${hex}`;
  }
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

const asArray = (raw: ParamRaw | undefined): string[] =>
  Array.isArray(raw) ? (raw as string[]) : raw === undefined ? [] : [String(raw)];

/**
 * GH3220 命令收发控制台（V3）：
 * - 参数全部由 `GH3220_COMMAND_CATALOG` 元数据驱动，可枚举项一律下拉 / 多选；
 * - 顶部「功能组合启动」行对齐 GH3036：只勾选功能名，掩码与细节由代码层补完；
 * - 日志同时显示语义值与实际下发数值。
 */
const Gh3220CommandConsole: React.FC = () => {
  const { t, i18n } = useTranslation('gh3220');
  const lang: Gh3220Lang = resolveGh3220Lang(i18n.language);
  const { token } = theme.useToken();
  const { status, deviceInfo, preferences, capture } = useGh3220Store();
  const config = useGh3220CommandConfig();
  const syncFromDevice = useGh3220CommandConfig((state) => state.syncFromDevice);

  const [sendingKey, setSendingKey] = useState<string | null>(null);
  const [lastResponse, setLastResponse] = useState('');
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [advancedHex, setAdvancedHex] = useState(GH3220_HEX_TEMPLATES[0].hex);
  const logId = useRef(0);

  // 真机 / 偏好反向回填（用户手动改过的字段不会被覆盖）
  useEffect(() => {
    syncFromDevice(deviceInfo, preferences, status);
  }, [deviceInfo, preferences, status, syncFromDevice]);

  const buildCtx = (values: Record<string, ParamRaw>): OptionContext => {
    const rawFunc = values.funcMask;
    return {
      lang,
      selectedFunctions: Array.isArray(rawFunc) ? (rawFunc as string[]) : config.selectedFunctions,
    };
  };

  const pushLog = (entry: Omit<LogEntry, 'id' | 'time'>) => {
    logId.current += 1;
    setLogs((prev) =>
      [{ ...entry, id: logId.current, time: new Date().toLocaleTimeString() }, ...prev].slice(0, 100)
    );
  };

  const execute = async (
    def: CommandDef,
    encoded: Record<string, unknown>,
    semantic: string,
    wire: string
  ) => {
    const started = performance.now();
    let ok = true;
    let responseText = '';
    try {
      const result = await def.invoke(encoded);
      responseText = formatResponse(result);
      message.success(`${t(`commands.list.${def.key}`)} · ${t('commands.success')}`);
    } catch (err) {
      ok = false;
      responseText = formatErrorMessage(err, t('commands.failed'));
      message.error(`${t(`commands.list.${def.key}`)} · ${responseText}`);
    }
    setLastResponse(responseText);
    pushLog({
      key: def.key,
      cmd: def.cmd,
      semantic,
      wire,
      response: responseText,
      ok,
      durationMs: Math.round(performance.now() - started),
    });
    setSendingKey(null);
  };

  const confirmAndExecute = (
    def: CommandDef,
    encoded: Record<string, unknown>,
    semantic: string,
    wire: string,
    confirmKey: string
  ) => {
    Modal.confirm({
      title: t(`commands.confirmTitles.${confirmKey}`),
      content: t(`commands.confirmContents.${confirmKey}`),
      okText: t('commands.confirm'),
      cancelText: t('commands.cancel'),
      okButtonProps: { danger: true },
      onOk: () => void execute(def, encoded, semantic, wire),
      onCancel: () => setSendingKey(null),
    });
  };

  const handleSend = (def: CommandDef, overrides?: Record<string, ParamRaw>) => {
    const values = overrides ?? config.rowValues[def.key] ?? DEFAULT_COMMAND_VALUES[def.key];
    const ctx = buildCtx(values);
    const error = validateCommand(def, values, ctx);
    if (error) {
      message.error(error);
      return;
    }
    const encoded = encodeCommand(def, values, ctx);
    const { semantic, wire } = describeCommand(def, values, ctx);
    setSendingKey(def.key);
    if (needsConfirm(def, encoded)) {
      const confirmKey = def.confirmKey ?? (def.danger ? 'stop' : 'chipReset');
      confirmAndExecute(def, encoded, semantic, wire, confirmKey);
      return;
    }
    void execute(def, encoded, semantic, wire);
  };

  const handleQuick = (key: string) => {
    const def = GH3220_COMMAND_CATALOG.find((item) => item.key === key);
    if (def) handleSend(def);
  };

  const setField = (def: CommandDef, param: ParamDef, value: ParamRaw) => {
    config.setRowValue(def.key, param.key, value);
    if (param.kind === 'functionMask') {
      config.setSelectedFunctions(asArray(value));
    }
    if (param.kind === 'bitmask') {
      config.setEventMaskKeys(asArray(value));
    }
  };

  /** 未知位（如设备回读的 0x…）补成可显示的选项，避免丢数据 */
  const withUnknownOptions = (options: Gh3220Option[], selected: string[]): Gh3220Option[] => {
    const known = new Set(options.map((item) => item.value));
    return [
      ...options,
      ...selected.filter((value) => !known.has(value)).map((value) => ({ value, label: value })),
    ];
  };

  const customEditor = (param: ParamDef, value: ParamRaw, onChange: (next: ParamRaw) => void) => {
    if (!isCustom(value)) return null;
    return (
      <InputNumber
        size="small"
        min={param.min ?? 0}
        max={param.max ?? 65535}
        value={Number(customValue(value, '0'))}
        onChange={(next) => onChange(customOption(next ?? 0))}
        style={{ width: 92 }}
      />
    );
  };

  const renderField = (def: CommandDef, param: ParamDef) => {
    const raw = config.rowValues[def.key]?.[param.key] ?? param.defaultValue;
    const ctx = buildCtx(config.rowValues[def.key] ?? {});
    const options = resolveOptions(param, ctx);
    const label = t(`commands.fields.${param.key}`);

    const common = (
      <Text key={`${param.key}-label`} style={{ fontSize: 11, color: token.colorTextTertiary }}>
        {label}
      </Text>
    );

    if (param.kind === 'functionMask' || param.kind === 'bitmask') {
      const selected = asArray(raw);
      const mask =
        param.kind === 'functionMask' ? maskOf(selected) : encodeEventMask(selected);
      const preview =
        param.kind === 'functionMask'
          ? formatFunctionMaskPreview(mask, lang)
          : formatEventMaskPreview(mask, lang);
      return (
        <span key={param.key} style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
          {common}
          <Select
            size="small"
            mode="multiple"
            maxTagCount={1}
            allowClear
            style={{ minWidth: 190 }}
            value={selected}
            onChange={(next: string[]) => setField(def, param, next)}
            options={withUnknownOptions(options, selected)}
            placeholder={t(
              param.kind === 'bitmask'
                ? 'commands.preview.selectEvents'
                : 'commands.preview.selectFunctions'
            )}
          />
          <Tooltip title={preview}>
            <Tag color="blue" style={{ margin: 0, maxWidth: 170, overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {preview}
            </Tag>
          </Tooltip>
        </span>
      );
    }

    if (param.kind === 'file') {
      return (
        <span key={param.key} style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
          {common}
          <Input
            size="small"
            readOnly
            style={{ width: 200 }}
            value={String(raw)}
            placeholder={t('commands.preview.selectFile')}
          />
          <Button
            size="small"
            icon={<FolderOpenOutlined />}
            onClick={async () => {
              const selected = await open({
                multiple: false,
                filters: [{ name: 'Register list', extensions: ['csv', 'txt'] }],
              });
              if (selected && typeof selected === 'string') setField(def, param, selected);
            }}
          />
        </span>
      );
    }

    if (param.kind === 'hex') {
      return (
        <span key={param.key} style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
          {common}
          <Input
            size="small"
            style={{ width: 170 }}
            value={String(raw)}
            onChange={(e) => setField(def, param, e.target.value)}
          />
          <Select
            size="small"
            style={{ width: 140 }}
            value={undefined}
            placeholder={t('commands.advanced.template')}
            options={GH3220_HEX_TEMPLATES.map((item) => ({
              value: item.key,
              label: lang === 'zh' ? item.nameZh : item.nameEn,
            }))}
            onChange={(key: string) => {
              const template = GH3220_HEX_TEMPLATES.find((item) => item.key === key);
              if (template) setField(def, param, template.hex);
            }}
          />
        </span>
      );
    }

    if (param.kind === 'numberList') {
      return (
        <span key={param.key} style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
          {common}
          <Input
            size="small"
            style={{ width: 140 }}
            value={String(raw)}
            onChange={(e) => setField(def, param, e.target.value)}
          />
        </span>
      );
    }

    // enum / bool / boundedNumber：一律 Select（可枚举即下拉）
    const text = String(raw);
    return (
      <span key={param.key} style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
        {common}
        <Select
          size="small"
          style={{ width: param.key === 'mode' ? 210 : 132 }}
          value={isCustom(raw) ? 'custom' : text}
          options={withUnknownOptions(options, [text])}
          onChange={(next: string) => {
            if (next === 'custom') {
              const fallback = customValue(raw, '') || String(param.defaultValue) || '0';
              setField(def, param, customOption(fallback));
            } else {
              setField(def, param, next);
            }
          }}
        />
        {customEditor(param, raw, (next) => setField(def, param, next))}
      </span>
    );
  };

  const commandColumns: ColumnsType<CommandDef> = [
    {
      title: t('commands.command'),
      key: 'command',
      width: 200,
      render: (_, def) => (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2, padding: '2px 0' }}>
          <Text style={{ fontSize: 13 }}>{t(`commands.list.${def.key}`)}</Text>
          <Tag
            color={def.cmd === '--' ? 'default' : 'blue'}
            style={{ width: 'fit-content', margin: 0, fontSize: 10, lineHeight: '16px' }}
          >
            {def.cmd === '--' ? '—' : `CMD ${def.cmd}`}
          </Tag>
        </div>
      ),
    },
    {
      title: t('commands.params'),
      key: 'params',
      render: (_, def) =>
        def.params.length === 0 ? (
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t('commands.noParams')}
          </Text>
        ) : (
          <Space size={[8, 4]} wrap>
            {def.params.map((param) => renderField(def, param))}
          </Space>
        ),
    },
    {
      title: t('commands.action'),
      key: 'action',
      width: 100,
      render: (_, def) => (
        <Button
          size="small"
          type={def.danger ? 'default' : 'primary'}
          danger={def.danger}
          icon={<SendOutlined />}
          loading={sendingKey === def.key}
          disabled={sendingKey !== null && sendingKey !== def.key}
          onClick={() => handleSend(def)}
          style={{ width: 90 }}
        >
          {t('commands.send')}
        </Button>
      ),
    },
  ];

  const logColumns: ColumnsType<LogEntry> = [
    { title: t('commands.time'), dataIndex: 'time', width: 92 },
    {
      title: t('commands.command'),
      key: 'command',
      width: 180,
      render: (_, row) => `${row.cmd === '--' ? '—' : row.cmd} · ${t(`commands.list.${row.key}`)}`,
    },
    {
      title: t('commands.semanticColumn'),
      dataIndex: 'semantic',
      ellipsis: true,
      width: 230,
      render: (text: string) => <Text style={{ fontSize: 12 }}>{text}</Text>,
    },
    {
      title: t('commands.wireColumn'),
      dataIndex: 'wire',
      ellipsis: true,
      width: 230,
      render: (text: string) => <Text code style={{ fontSize: 11 }}>{text}</Text>,
    },
    {
      title: t('commands.result'),
      dataIndex: 'response',
      ellipsis: true,
      width: 180,
      render: (text: string) => <Text style={{ fontSize: 12 }}>{text.replace(/\s+/g, ' ')}</Text>,
    },
    { title: t('commands.duration'), dataIndex: 'durationMs', width: 80, render: (ms: number) => `${ms} ms` },
    {
      title: t('commands.status'),
      dataIndex: 'ok',
      width: 76,
      render: (ok: boolean) => (
        <Tag color={ok ? 'success' : 'error'}>{ok ? t('commands.success') : t('commands.failed')}</Tag>
      ),
    },
  ];

  const running = status.capturing || capture.active || config.isRunning;
  const selectedMask = maskOf(config.selectedFunctions);
  const maskPreview = formatFunctionMaskPreview(selectedMask, lang);

  /** 组合启动：只勾选功能，其余细节由代码层补完（对齐 GH3036 bitMap.reduce） */
  const handleComboStart = async () => {
    if (config.selectedFunctions.length === 0) {
      message.warning(t('commands.preview.functionRequired'));
      return;
    }
    const encoded: Gh3220CaptureConfig = {
      autoStart: true,
      autoStop: false,
      mode: Number(config.workMode),
      slotEn: encodeBoundedNumber(config.slotEn) ?? 0,
      funcMask: selectedMask,
      channelsOverride: encodeBoundedNumber(config.channelsOverride),
      csvEnabled: config.csvEnabled === 'true',
    };
    const semantic = describeCommand(
      GH3220_COMMAND_CATALOG.find((def) => def.key === 'captureStart')!,
      {
        autoStart: 'true',
        autoStop: 'false',
        mode: config.workMode,
        slotEn: config.slotEn,
        funcMask: config.selectedFunctions,
        channelsOverride: config.channelsOverride,
        csvEnabled: config.csvEnabled,
      },
      { lang, selectedFunctions: config.selectedFunctions }
    );
    setSendingKey('combo');
    try {
      await gh3220Api.captureStart(encoded);
      config.setConfig({ isRunning: true });
      setLastResponse(t('commands.combo.startSuccess'));
      pushLog({
        key: 'captureStart',
        cmd: '0x10+0x0C',
        semantic: semantic.semantic,
        wire: `captureStart(funcMask:0x${selectedMask.toString(16).toUpperCase()}, mode:${encoded.mode}, slotEn:${encoded.slotEn})`,
        response: t('commands.combo.startSuccess'),
        ok: true,
        durationMs: 0,
      });
      message.success(t('commands.combo.startSuccess'));
    } catch (err) {
      const text = formatErrorMessage(err, t('commands.failed'));
      setLastResponse(text);
      pushLog({
        key: 'captureStart',
        cmd: '0x10+0x0C',
        semantic: semantic.semantic,
        wire: `captureStart(funcMask:0x${selectedMask.toString(16).toUpperCase()})`,
        response: text,
        ok: false,
        durationMs: 0,
      });
      message.error(text);
    } finally {
      setSendingKey(null);
    }
  };

  const handleComboStop = () => {
    Modal.confirm({
      title: t('commands.confirmTitles.captureStop'),
      content: t('commands.confirmContents.captureStop'),
      okText: t('commands.confirm'),
      cancelText: t('commands.cancel'),
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          await gh3220Api.captureStop();
          config.setConfig({ isRunning: false });
          setLastResponse(t('commands.combo.stopSuccess'));
          pushLog({
            key: 'captureStop',
            cmd: '--',
            semantic: t('commands.combo.stop'),
            wire: 'captureStop()',
            response: t('commands.combo.stopSuccess'),
            ok: true,
            durationMs: 0,
          });
          message.success(t('commands.combo.stopSuccess'));
        } catch (err) {
          message.error(formatErrorMessage(err, t('commands.failed')));
        }
      },
    });
  };

  const sectionStyle: React.CSSProperties = {
    padding: '12px 16px',
    background: token.colorFillSecondary,
    borderRadius: 8,
  };
  const labelStyle: React.CSSProperties = {
    fontSize: 13,
    fontWeight: 500,
    marginBottom: 8,
    color: token.colorTextSecondary,
  };
  const rowLabel: React.CSSProperties = {
    width: 82,
    fontSize: 12,
    color: token.colorTextSecondary,
    flex: '0 0 auto',
  };

  const comboRow = (label: string, control: React.ReactNode, action?: React.ReactNode) => (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
      <span style={rowLabel}>{label}</span>
      {control}
      {action}
    </div>
  );

  const advancedItems = [
    {
      key: 'advanced',
      label: t('commands.advanced.title'),
      children: (
        <Space direction="vertical" size={8} style={{ width: '100%' }}>
          <Space wrap>
            <Text style={{ fontSize: 12 }}>{t('commands.fields.slotEn')}</Text>
            <Select
              size="small"
              style={{ width: 120 }}
              value={isCustom(config.slotEn) ? 'custom' : config.slotEn}
              options={boundedOptions('slotEn', lang)}
              onChange={(next: string) => config.setConfig({ slotEn: next })}
            />
            {customEditor({ key: 'slotEn', kind: 'boundedNumber', defaultValue: '0', min: 0, max: 255, encode: () => 0 } as ParamDef, config.slotEn, (next) =>
              config.setConfig({ slotEn: String(next) })
            )}
            <Text style={{ fontSize: 12 }}>{t('commands.fields.channelsOverride')}</Text>
            <Select
              size="small"
              style={{ width: 160 }}
              value={isCustom(config.channelsOverride) ? 'custom' : config.channelsOverride}
              options={channelOptions(config.selectedFunctions, lang)}
              onChange={(next: string) => config.setConfig({ channelsOverride: next })}
            />
            <Text style={{ fontSize: 12 }}>{t('commands.fields.csvEnabled')}</Text>
            <Select
              size="small"
              style={{ width: 90 }}
              value={config.csvEnabled}
              options={boolOptions(lang)}
              onChange={(next: string) => config.setConfig({ csvEnabled: next })}
            />
          </Space>
          <Space wrap>
            <Text style={{ fontSize: 12 }}>{t('commands.advanced.template')}</Text>
            <Select
              size="small"
              style={{ width: 170 }}
              value={undefined}
              placeholder={t('commands.advanced.template')}
              options={GH3220_HEX_TEMPLATES.map((item) => ({
                value: item.key,
                label: lang === 'zh' ? item.nameZh : item.nameEn,
              }))}
              onChange={(key: string) => {
                const template = GH3220_HEX_TEMPLATES.find((item) => item.key === key);
                if (template) setAdvancedHex(template.hex);
              }}
            />
            <Input
              size="small"
              style={{ width: 220 }}
              value={advancedHex}
              onChange={(e) => setAdvancedHex(e.target.value)}
            />
            <Button
              size="small"
              icon={<SendOutlined />}
              onClick={() => {
                const def = GH3220_COMMAND_CATALOG.find((item) => item.key === 'sendRaw')!;
                handleSend(def, { hex: encodeHex(advancedHex) });
              }}
            >
              {t('commands.advanced.rawSend')}
            </Button>
          </Space>
        </Space>
      ),
    },
  ];

  return (
    <Space direction="vertical" size={10} style={{ width: '100%' }}>
      {status.state === 'disconnected' && (
        <Alert type="warning" showIcon message={t('commands.notConnected')} />
      )}

      {/* 功能组合启动行：对齐 GH3036 的 functionSelect 单行 */}
      <div style={sectionStyle}>
        <div style={labelStyle}>{t('commands.combo.title')}</div>
        <Space direction="vertical" size={8} style={{ width: '100%' }}>
          {comboRow(
            t('commands.combo.functions'),
            <>
              <Select
                size="small"
                mode="multiple"
                maxTagCount={2}
                allowClear
                style={{ minWidth: 330, flex: 1 }}
                value={config.selectedFunctions}
                options={withUnknownOptions(functionOptions(lang), config.selectedFunctions)}
                onChange={(keys: string[]) => config.setSelectedFunctions(keys)}
                placeholder={t('commands.preview.selectFunctions')}
              />
              <Tooltip title={maskPreview}>
                <Tag color="blue" style={{ margin: 0 }}>{maskPreview}</Tag>
              </Tooltip>
            </>,
            <Button
              size="small"
              type={running ? 'default' : 'primary'}
              danger={running}
              loading={sendingKey === 'combo'}
              style={{ width: 90 }}
              onClick={() => (running ? handleComboStop() : void handleComboStart())}
            >
              {running ? t('commands.combo.stop') : t('commands.combo.start')}
            </Button>
          )}
          {comboRow(
            t('commands.combo.mode'),
            <Select
              size="small"
              style={{ width: 300 }}
              value={config.workMode}
              options={workModeOptions(lang)}
              onChange={(next: string) => config.setConfig({ workMode: next })}
            />
          )}
          {config.lastUpdatedFromDevice && !config.manualEdited.selectedFunctions ? (
            <Text type="secondary" style={{ fontSize: 11 }}>
              {t('commands.preview.fromDevice')}
            </Text>
          ) : null}
          <Collapse size="small" items={advancedItems} />
        </Space>
      </div>

      <div style={sectionStyle}>
        <div style={labelStyle}>{t('commands.quick')}</div>
        <Space wrap size={8}>
          {QUICK_KEYS.map((key) => (
            <Button key={key} size="small" icon={<ReloadOutlined />} onClick={() => handleQuick(key)}>
              {t(`commands.list.${key}`)}
            </Button>
          ))}
          <Button
            size="small"
            icon={<ClearOutlined />}
            onClick={() => {
              setLogs([]);
              setLastResponse('');
            }}
          >
            {t('commands.clear')}
          </Button>
        </Space>
      </div>

      <div style={sectionStyle}>
        <div style={labelStyle}>{t('commands.catalogTitle')}</div>
        <Table<CommandDef>
          rowKey="key"
          size="small"
          bordered
          columns={commandColumns}
          dataSource={GH3220_COMMAND_CATALOG}
          pagination={false}
          scroll={{ y: 460, x: 1180 }}
        />
        <Text type="secondary" style={{ fontSize: 12, display: 'block', marginTop: 8 }}>
          {t('commands.hint')}
        </Text>
      </div>

      <div style={sectionStyle}>
        <div style={labelStyle}>{t('commands.responseTitle')}</div>
        <pre
          style={{
            margin: 0,
            padding: 8,
            maxHeight: 160,
            overflow: 'auto',
            background: token.colorBgContainer,
            borderRadius: 6,
            fontSize: 12,
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-all',
          }}
        >
          {lastResponse || t('commands.empty')}
        </pre>
      </div>

      <div style={sectionStyle}>
        <div style={labelStyle}>{t('commands.logTitle', { count: logs.length })}</div>
        <Table<LogEntry>
          rowKey="id"
          size="small"
          columns={logColumns}
          dataSource={logs}
          pagination={false}
          scroll={{ y: 220, x: 1150 }}
          locale={{ emptyText: t('commands.empty') }}
        />
      </div>
    </Space>
  );
};

export default Gh3220CommandConsole;
