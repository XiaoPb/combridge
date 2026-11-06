import React, { useRef, useState } from 'react';
import {
  Alert,
  Button,
  Input,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message,
  theme,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { SendOutlined, ClearOutlined, ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { gh3220Api } from '../../api/gh3220';
import { useGh3220Store } from '../../stores/gh3220Store';
import {
  GH3220_WORK_MODE_LIST,
  formatWorkModeLabel,
  parseFunctionMaskText,
  resolveGh3220Lang,
  type Gh3220Lang,
} from '../../constants/gh3220Functions';
import type { Gh3220CaptureConfig } from '../../types/gh3220';
import { formatErrorMessage } from '../../utils/errorMessage';

const { Text } = Typography;

type CommandValues = Record<string, string>;

interface FieldOption {
  value: string;
  label: string;
}

interface CommandField {
  key: string;
  kind: 'text' | 'select';
  defaultValue: string;
  options?: FieldOption[] | ((lang: Gh3220Lang) => FieldOption[]);
}

interface CommandDef {
  key: string;
  /** UProtocol 命令号，`--` 表示复合命令或纯状态/配置命令 */
  cmd: string;
  fields: CommandField[];
  /** 下发前需要二次确认（真机 STOP 后通常需复位才能再次 START） */
  danger?: boolean;
  run: (values: CommandValues) => Promise<unknown>;
}

interface LogEntry {
  id: number;
  time: string;
  key: string;
  cmd: string;
  request: string;
  response: string;
  ok: boolean;
  durationMs: number;
}

const toNumber = (text: string, fallback = 0): number => {
  const value = text.trim();
  if (!value) return fallback;
  const parsed = value.toLowerCase().startsWith('0x')
    ? Number.parseInt(value.slice(2), 16)
    : Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
};

const toValueList = (text: string): number[] =>
  text
    .split(/[,;\s]+/)
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => toNumber(part));

const toBool = (text: string): boolean => text === 'true';

const workModeOptions = (lang: Gh3220Lang): FieldOption[] =>
  GH3220_WORK_MODE_LIST.map((info) => ({
    value: String(info.mode),
    label: formatWorkModeLabel(info.mode, lang),
  }));

const boolOptions = (lang: Gh3220Lang): FieldOption[] => [
  { value: 'true', label: lang === 'zh' ? '是' : 'Yes' },
  { value: 'false', label: lang === 'zh' ? '否' : 'No' },
];

const chipActions: FieldOption[] = [
  { value: 'wakeup', label: 'wakeup' },
  { value: 'sleep', label: 'sleep' },
  { value: 'soft-reset', label: 'soft-reset' },
  { value: 'hard-reset', label: 'hard-reset' },
  { value: 'rx-reset', label: 'rx-reset' },
];

const maskField = (): CommandField => ({
  key: 'funcMask',
  kind: 'text',
  defaultValue: 'hr',
});

/** 后端 `gh3220_*` 命令的完整目录；与 src/api/gh3220.ts 一一对应 */
const COMMANDS: CommandDef[] = [
  // —— 状态与信息 ——
  { key: 'getStatus', cmd: '--', fields: [], run: () => gh3220Api.getStatus() },
  { key: 'getDeviceInfo', cmd: '0x19', fields: [], run: () => gh3220Api.getDeviceInfo() },
  { key: 'getStats', cmd: '--', fields: [], run: () => gh3220Api.getStats() },
  { key: 'getCaptureStatus', cmd: '--', fields: [], run: () => gh3220Api.getCaptureStatus() },
  { key: 'getPreferences', cmd: '--', fields: [], run: () => gh3220Api.getPreferences() },
  { key: 'clearStats', cmd: '--', fields: [], run: () => gh3220Api.clearStats() },
  // —— 采样控制 ——
  {
    key: 'setWorkMode',
    cmd: '0x10',
    fields: [
      { key: 'mode', kind: 'select', defaultValue: '2', options: workModeOptions },
      maskField(),
    ],
    run: (v) => gh3220Api.setWorkMode(toNumber(v.mode), parseFunctionMaskText(v.funcMask)),
  },
  {
    key: 'start',
    cmd: '0x0C',
    fields: [maskField(), { key: 'slotEn', kind: 'text', defaultValue: '0' }],
    run: (v) => gh3220Api.start(parseFunctionMaskText(v.funcMask), toNumber(v.slotEn)),
  },
  {
    key: 'stop',
    cmd: '0x0C',
    danger: true,
    fields: [maskField()],
    run: (v) => gh3220Api.stop(parseFunctionMaskText(v.funcMask)),
  },
  {
    key: 'switchSlotFunc',
    cmd: '0x0F',
    fields: [
      { key: 'slot', kind: 'text', defaultValue: '0' },
      { key: 'extra', kind: 'text', defaultValue: '0' },
      maskField(),
      { key: 'on', kind: 'select', defaultValue: 'true', options: boolOptions },
    ],
    run: (v) =>
      gh3220Api.switchSlotFunc(
        toNumber(v.slot),
        toNumber(v.extra),
        parseFunctionMaskText(v.funcMask),
        toBool(v.on)
      ),
  },
  {
    key: 'captureStart',
    cmd: '--',
    fields: [
      { key: 'autoStart', kind: 'select', defaultValue: 'true', options: boolOptions },
      { key: 'autoStop', kind: 'select', defaultValue: 'false', options: boolOptions },
      { key: 'mode', kind: 'select', defaultValue: '2', options: workModeOptions },
      { key: 'slotEn', kind: 'text', defaultValue: '0' },
      maskField(),
      { key: 'channelsOverride', kind: 'text', defaultValue: '' },
      { key: 'csvEnabled', kind: 'select', defaultValue: 'true', options: boolOptions },
    ],
    run: (v) => {
      const config: Gh3220CaptureConfig = {
        autoStart: toBool(v.autoStart),
        autoStop: toBool(v.autoStop),
        mode: toNumber(v.mode),
        slotEn: toNumber(v.slotEn),
        funcMask: parseFunctionMaskText(v.funcMask),
        channelsOverride: v.channelsOverride ? toNumber(v.channelsOverride) : null,
        csvEnabled: toBool(v.csvEnabled),
      };
      return gh3220Api.captureStart(config);
    },
  },
  {
    key: 'captureStop',
    cmd: '--',
    fields: [],
    run: () => gh3220Api.captureStop(),
  },
  // —— 寄存器与芯片 ——
  {
    key: 'regRead',
    cmd: '0x03',
    fields: [
      { key: 'addr', kind: 'text', defaultValue: '0x00' },
      { key: 'count', kind: 'text', defaultValue: '1' },
    ],
    run: (v) => gh3220Api.regRead(toNumber(v.addr), toNumber(v.count, 1)),
  },
  {
    key: 'regWrite',
    cmd: '0x03',
    fields: [
      { key: 'addr', kind: 'text', defaultValue: '0x00' },
      { key: 'values', kind: 'text', defaultValue: '0x0000' },
    ],
    run: (v) => gh3220Api.regWrite(toNumber(v.addr), toValueList(v.values)),
  },
  {
    key: 'loadRegList',
    cmd: '--',
    fields: [{ key: 'filePath', kind: 'text', defaultValue: '' }],
    run: (v) => gh3220Api.loadRegList(v.filePath.trim()),
  },
  {
    key: 'chipCtrl',
    cmd: '0x17',
    fields: [{ key: 'action', kind: 'select', defaultValue: 'wakeup', options: chipActions }],
    run: (v) => gh3220Api.chipCtrl(v.action),
  },
  {
    key: 'setFifoThr',
    cmd: '--',
    fields: [{ key: 'value', kind: 'text', defaultValue: '0' }],
    run: (v) => gh3220Api.setFifoThr(toNumber(v.value)),
  },
  {
    key: 'setReportEvent',
    cmd: '--',
    fields: [{ key: 'mask', kind: 'text', defaultValue: '0x0000' }],
    run: (v) => gh3220Api.setReportEvent(toNumber(v.mask)),
  },
  // —— 调试与原始帧 ——
  {
    key: 'debugStatus',
    cmd: '0xA2',
    fields: [{ key: 'kind', kind: 'text', defaultValue: '0' }],
    run: (v) => gh3220Api.debugStatus(v.kind.trim()),
  },
  {
    key: 'packageTest',
    cmd: '0x05',
    fields: [{ key: 'hex', kind: 'text', defaultValue: '0102' }],
    run: (v) => gh3220Api.packageTest(v.hex.trim()),
  },
  {
    key: 'sendRaw',
    cmd: '--',
    fields: [{ key: 'hex', kind: 'text', defaultValue: 'AA 11 19 01 01' }],
    run: (v) => gh3220Api.sendRaw(v.hex.trim()),
  },
  // —— 配置 ——
  {
    key: 'updatePreferences',
    cmd: '--',
    fields: [],
    run: async () => {
      const prefs = useGh3220Store.getState().preferences ?? (await gh3220Api.getPreferences());
      await gh3220Api.updatePreferences(prefs);
      return prefs;
    },
  },
];

const QUICK_KEYS = ['getDeviceInfo', 'getStatus', 'getStats', 'getCaptureStatus', 'packageTest'];

function defaultValues(def: CommandDef): CommandValues {
  return def.fields.reduce<CommandValues>((acc, field) => {
    acc[field.key] = field.defaultValue;
    return acc;
  }, {});
}

function initialRowValues(): Record<string, CommandValues> {
  return COMMANDS.reduce<Record<string, CommandValues>>((acc, def) => {
    acc[def.key] = defaultValues(def);
    return acc;
  }, {});
}

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

/**
 * GH3220 命令收发控制台：
 * 命令目录按 GH3036「RPC 命令」页的行式表格排布——每条命令一行，
 * 左为命令名/命令号，中间为参数控件，右侧为行内发送按钮；
 * 下方保留最近一次响应与 TX/RX 收发日志。
 */
const Gh3220CommandConsole: React.FC = () => {
  const { t, i18n } = useTranslation('gh3220');
  const lang = resolveGh3220Lang(i18n.language);
  const { token } = theme.useToken();
  const { status } = useGh3220Store();

  const [rowValues, setRowValues] = useState<Record<string, CommandValues>>(initialRowValues);
  const [sendingKey, setSendingKey] = useState<string | null>(null);
  const [lastResponse, setLastResponse] = useState('');
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const logId = useRef(0);

  const setField = (commandKey: string, fieldKey: string, value: string) =>
    setRowValues((prev) => ({
      ...prev,
      [commandKey]: { ...(prev[commandKey] ?? {}), [fieldKey]: value },
    }));

  const execute = async (def: CommandDef, requestValues: CommandValues) => {
    const started = performance.now();
    logId.current += 1;
    let ok = true;
    let responseText = '';
    try {
      const result = await def.run(requestValues);
      responseText = formatResponse(result);
      message.success(`${t(`commands.list.${def.key}`)} · ${t('commands.success')}`);
    } catch (err) {
      ok = false;
      responseText = formatErrorMessage(err, t('commands.failed'));
      message.error(`${t(`commands.list.${def.key}`)} · ${responseText}`);
    }
    const entry: LogEntry = {
      id: logId.current,
      time: new Date().toLocaleTimeString(),
      key: def.key,
      cmd: def.cmd,
      request: Object.keys(requestValues).length > 0 ? JSON.stringify(requestValues) : '—',
      response: responseText,
      ok,
      durationMs: Math.round(performance.now() - started),
    };
    setLastResponse(responseText);
    setLogs((prev) => [entry, ...prev].slice(0, 100));
    setSendingKey(null);
  };

  const handleSend = (def: CommandDef, requestValues?: CommandValues) => {
    const payload = requestValues ?? rowValues[def.key] ?? defaultValues(def);
    setSendingKey(def.key);
    if (def.danger) {
      Modal.confirm({
        title: t('commands.stopConfirmTitle'),
        content: t('commands.stopConfirmContent'),
        okText: t('commands.confirm'),
        cancelText: t('commands.cancel'),
        okButtonProps: { danger: true },
        onOk: () => execute(def, payload),
        onCancel: () => setSendingKey(null),
      });
      return;
    }
    void execute(def, payload);
  };

  const handleQuick = (key: string) => {
    const def = COMMANDS.find((item) => item.key === key);
    if (def) handleSend(def, defaultValues(def));
  };

  const renderField = (def: CommandDef, field: CommandField) => {
    const options = typeof field.options === 'function' ? field.options(lang) : field.options;
    const value = rowValues[def.key]?.[field.key] ?? field.defaultValue;
    const wide = field.key === 'mode' || field.key === 'filePath' || field.key === 'hex';
    const controlStyle: React.CSSProperties = { width: wide ? 230 : 100 };
    return (
      <span
        key={field.key}
        style={{ display: 'inline-flex', alignItems: 'center', gap: 4, whiteSpace: 'nowrap' }}
      >
        <Text style={{ fontSize: 11, color: token.colorTextTertiary }}>
          {t(`commands.fields.${field.key}`)}
        </Text>
        {field.kind === 'select' && options ? (
          <Select
            size="small"
            style={controlStyle}
            value={value}
            onChange={(next: string) => setField(def.key, field.key, next)}
            options={options}
          />
        ) : (
          <Input
            size="small"
            style={controlStyle}
            value={value}
            onChange={(e) => setField(def.key, field.key, e.target.value)}
          />
        )}
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
        def.fields.length === 0 ? (
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t('commands.noParams')}
          </Text>
        ) : (
          <Space size={[8, 4]} wrap>
            {def.fields.map((field) => renderField(def, field))}
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
      width: 190,
      render: (_, row) => `${row.cmd === '--' ? '—' : row.cmd} · ${t(`commands.list.${row.key}`)}`,
    },
    { title: t('commands.request'), dataIndex: 'request', ellipsis: true, width: 180 },
    {
      title: t('commands.result'),
      dataIndex: 'response',
      ellipsis: true,
      render: (text: string) => <Text style={{ fontSize: 12 }}>{text.replace(/\s+/g, ' ')}</Text>,
    },
    {
      title: t('commands.duration'),
      dataIndex: 'durationMs',
      width: 80,
      render: (ms: number) => `${ms} ms`,
    },
    {
      title: t('commands.status'),
      dataIndex: 'ok',
      width: 76,
      render: (ok: boolean) => (
        <Tag color={ok ? 'success' : 'error'}>
          {ok ? t('commands.success') : t('commands.failed')}
        </Tag>
      ),
    },
  ];

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

  return (
    <Space direction="vertical" size={10} style={{ width: '100%' }}>
      {status.state === 'disconnected' && (
        <Alert type="warning" showIcon message={t('commands.notConnected')} />
      )}

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
          dataSource={COMMANDS}
          pagination={false}
          scroll={{ y: 460, x: 860 }}
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
          scroll={{ y: 220, x: 900 }}
          locale={{ emptyText: t('commands.empty') }}
        />
      </div>
    </Space>
  );
};

export default Gh3220CommandConsole;
