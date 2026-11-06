/**
 * GH3220 命令参数 schema（V3：`docs/gh3220-ui-alignment-v3-plan.md` P0）
 *
 * 目标：UI 上只出现「选择」，所有数值编码、默认值、校验、选项生成集中在这里。
 * 组件只渲染控件、只保存语义值；`encodeCommand()` 是唯一的数值出口。
 *
 * 与 GH3036 的差异：GH3036 是字符串 RPC（`executeRpc(key, string[])`），
 * GH3220 保留 24 个类型化的 `gh3220Api.*` invoke，schema 负责把语义值编码成
 * 这些 invoke 需要的 number / boolean / string / number[]。后端不改。
 *
 * 数据出处：
 * - CMD 号：`libs/gh-uprotocol/src/lib.rs:18-38`（本文件 §3.6 已修正历史错误）
 * - 参数类型：`src-tauri/src/commands/gh3220.rs:44-176`
 * - debugStatus.kind / chipCtrl 动作：`src-tauri/src/gh3220/manager.rs:487-521`
 * - 版本类型：`libs/gh-uprotocol/src/lib.rs:42-52`
 */

import { gh3220Api } from '../api/gh3220';
import type { Gh3220CaptureConfig } from '../types/gh3220';
import {
  asText,
  boolOptions,
  boundedOptions,
  channelOptions,
  chipActionOptions,
  CUSTOM_PREFIX,
  customValue,
  debugKindOptions,
  encodeEventMask,
  encodeFunctionMask,
  eventMaskOptions,
  formatEventMaskPreview,
  formatFunctionMaskPreview,
  functionOptions,
  GH3220_COMMAND_CODES,
  isCustom,
  parseNumericText,
  versionTypeOptions,
  workModeOptions,
  type Gh3220Lang,
  type Gh3220Option,
} from './gh3220Functions';

export type ParamKind =
  | 'functionMask'
  | 'bitmask'
  | 'enum'
  | 'bool'
  | 'boundedNumber'
  | 'file'
  | 'hex'
  | 'numberList';

export type FieldOption = Gh3220Option;
export type ParamRaw = string | string[];
export type CommandGroup = 'status' | 'sampling' | 'register' | 'debug' | 'config';

/** 动态选项的上下文：语言、已选功能（决定通道数上限）、设备发现的特征 UUID */
export interface OptionContext {
  lang: Gh3220Lang;
  selectedFunctions: string[];
  discoveredWriteUuids?: string[];
  discoveredNotifyUuids?: string[];
}

export interface ParamDef {
  /** 同时是 i18n key：`commands.fields.<key>` */
  key: string;
  kind: ParamKind;
  /** 语义值：多选为 string[]，其余为 string；`custom:<value>` 表示自定义值 */
  defaultValue: ParamRaw;
  options?: FieldOption[] | ((ctx: OptionContext) => FieldOption[]);
  min?: number;
  max?: number;
  /** 多选类：至少选 1 项 */
  required?: boolean;
  /** 语义值 → 该命令所需的实际参数 */
  encode: (raw: ParamRaw, ctx: OptionContext) => unknown;
  validate?: (raw: ParamRaw, ctx: OptionContext) => string | null;
}

export interface CommandDef {
  /** 与 `gh3220Api` 的方法名一一对应，同时是 i18n key `commands.list.<key>` */
  key: string;
  /** UProtocol CMD 号；`--` 表示复合/纯状态命令 */
  cmd: string;
  group: CommandGroup;
  params: ParamDef[];
  /** 固定需要二次确认 */
  danger?: boolean;
  /** 二次确认文案的 i18n 后缀：`commands.<confirmKey>Title/Content` */
  confirmKey?: 'stop' | 'captureStop' | 'chipReset';
  /** 按编码后的实际值动态决定是否需要确认（如 reset 类芯片动作） */
  confirmWhen?: (encoded: Record<string, unknown>) => boolean;
  invoke: (encoded: Record<string, unknown>) => Promise<unknown>;
}

/* -------------------------------------------------------------------------- */
/* 通用编码与校验                                                              */
/* -------------------------------------------------------------------------- */

const msg = (ctx: OptionContext, zh: string, en: string): string => (ctx.lang === 'zh' ? zh : en);

const asArray = (raw: ParamRaw): string[] =>
  (Array.isArray(raw) ? raw : [raw]).map((item) => String(item)).filter(Boolean);

/** `auto` → null；`custom:<n>` / 普通档位 → number */
export function encodeBoundedNumber(raw: ParamRaw): number | null {
  const text = asText(raw);
  if (text === 'auto' || text === '') return null;
  const value = parseNumericText(isCustom(raw) ? customValue(raw) : text, Number.NaN);
  return Number.isFinite(value) ? value : Number.NaN;
}

export function encodeNumberList(raw: ParamRaw): number[] {
  return asArray(raw)
    .flatMap((item) => item.split(/[,;\s]+/))
    .filter(Boolean)
    .map((item) => parseNumericText(item, Number.NaN));
}

export function encodeHex(raw: ParamRaw): string {
  return asText(raw).replace(/0x/gi, '').replace(/[\s,;_-]/g, '');
}

function defaultEncoder(kind: ParamKind): ParamDef['encode'] {
  switch (kind) {
    case 'functionMask':
      return (raw) => encodeFunctionMask(asArray(raw));
    case 'bitmask':
      return (raw) => encodeEventMask(asArray(raw));
    case 'bool':
      return (raw) => asText(raw) === 'true';
    case 'boundedNumber':
      return (raw) => encodeBoundedNumber(raw);
    case 'file':
      return (raw) => asText(raw).trim();
    case 'hex':
      return (raw) => encodeHex(raw);
    case 'numberList':
      return (raw) => encodeNumberList(raw);
    case 'enum':
    default:
      return (raw) => (isCustom(raw) ? customValue(raw) : asText(raw));
  }
}

function defaultValidator(param: ParamDef): ParamDef['validate'] {
  return (raw, ctx) => {
    if (param.kind === 'functionMask' || param.kind === 'bitmask') {
      const keys = asArray(raw);
      if (param.required && keys.length === 0) {
        return msg(ctx, '请至少选择一个功能', 'Select at least one function');
      }
      if (param.kind === 'functionMask' && keys.length > 0 && encodeFunctionMask(keys) === 0) {
        return msg(ctx, '功能位掩码不能为 0', 'Function mask must not be 0');
      }
      return null;
    }
    if (param.kind === 'hex') {
      const text = encodeHex(raw);
      if (!text) return msg(ctx, 'HEX 数据不能为空', 'HEX data is required');
      if (!/^[0-9a-f]+$/i.test(text) || text.length % 2 !== 0) {
        return msg(ctx, 'HEX 需为偶数位十六进制字节', 'HEX must contain an even number of hex digits');
      }
      return null;
    }
    if (param.kind === 'file') {
      return asText(raw).trim()
        ? null
        : msg(ctx, '请先选择文件', 'Select a file first');
    }
    if (param.kind === 'boundedNumber') {
      const value = encodeBoundedNumber(raw);
      if (value === null) return null;
      if (!Number.isFinite(value)) return msg(ctx, '数值格式无效', 'Invalid number');
      if (param.min !== undefined && value < param.min) {
        return msg(ctx, `不能小于 ${param.min}`, `Must be >= ${param.min}`);
      }
      if (param.max !== undefined && value > param.max) {
        return msg(ctx, `不能大于 ${param.max}`, `Must be <= ${param.max}`);
      }
      return null;
    }
    if (param.kind === 'numberList') {
      const values = encodeNumberList(raw);
      if (values.length === 0 || values.some((v) => !Number.isFinite(v))) {
        return msg(ctx, '数值列表格式无效', 'Invalid value list');
      }
      if (param.max !== undefined && values.some((v) => v > param.max!)) {
        return msg(ctx, `每项不能大于 ${param.max}`, `Each value must be <= ${param.max}`);
      }
      return null;
    }
    if (isCustom(raw) && param.kind === 'enum' && !customValue(raw).trim()) {
      return msg(ctx, '请输入自定义值', 'Enter a custom value');
    }
    return null;
  };
}

function makeParam(
  key: string,
  kind: ParamKind,
  defaultValue: ParamRaw,
  extra: Partial<ParamDef> = {}
): ParamDef {
  const base: ParamDef = { key, kind, defaultValue, encode: defaultEncoder(kind) };
  const param = { ...base, ...extra };
  if (!extra.validate) param.validate = defaultValidator(param);
  return param;
}

/** 解析参数的动态选项 */
export function resolveOptions(param: ParamDef, ctx: OptionContext): FieldOption[] {
  if (!param.options) return [];
  return typeof param.options === 'function' ? param.options(ctx) : param.options;
}

/* -------------------------------------------------------------------------- */
/* 参数定义（§7.2 参数字典）                                                   */
/* -------------------------------------------------------------------------- */

const funcMask = (): ParamDef =>
  makeParam('funcMask', 'functionMask', ['hr'], {
    required: true,
    options: (ctx) => functionOptions(ctx.lang),
  });

const mode = (): ParamDef =>
  makeParam('mode', 'enum', '2', {
    options: (ctx) => workModeOptions(ctx.lang),
    encode: (raw) => parseNumericText(isCustom(raw) ? customValue(raw) : asText(raw)),
  });

const bounded = (
  key: string,
  defaultValue: string,
  options: FieldOption[] | ((ctx: OptionContext) => FieldOption[]),
  min: number,
  max: number
): ParamDef => makeParam(key, 'boundedNumber', defaultValue, { options, min, max });

const slotEn = (): ParamDef =>
  bounded('slotEn', '0', (ctx) => boundedOptions('slotEn', ctx.lang), 0, 255);
const slot = (): ParamDef =>
  bounded('slot', '0', (ctx) => boundedOptions('slot', ctx.lang), 0, 255);
const extra = (): ParamDef =>
  bounded('extra', '0', (ctx) => boundedOptions('extra', ctx.lang), 0, 255);
const bool = (key: string, defaultValue: 'true' | 'false'): ParamDef =>
  makeParam(key, 'bool', defaultValue, { options: (ctx) => boolOptions(ctx.lang) });
const channelsOverride = (): ParamDef =>
  bounded('channelsOverride', 'auto', (ctx) => channelOptions(ctx.selectedFunctions, ctx.lang), 1, 32);

const addr = (): ParamDef =>
  makeParam('addr', 'numberList', '0x0000', {
    min: 0,
    max: 65535,
    encode: (raw) => parseNumericText(asText(raw), Number.NaN),
    validate: (raw, ctx) => {
      const value = parseNumericText(asText(raw), Number.NaN);
      if (!Number.isFinite(value) || value < 0 || value > 65535) {
        return msg(ctx, '寄存器地址需为 0..65535（支持 0x 前缀）', 'Register address must be 0..65535');
      }
      return null;
    },
  });

const values = (): ParamDef =>
  makeParam('values', 'numberList', '0x0000', { max: 65535 });

const hex = (defaultValue = '0102'): ParamDef =>
  makeParam('hex', 'hex', defaultValue);

/* -------------------------------------------------------------------------- */
/* 命令目录（22 条，与 gh3220Api 除 connect/disconnect 外一一对应）            */
/* -------------------------------------------------------------------------- */

const hexCode = (code: number): string => `0x${code.toString(16).toUpperCase().padStart(2, '0')}`;
const C = GH3220_COMMAND_CODES;

const n = (e: Record<string, unknown>, key: string): number => e[key] as number;
const s = (e: Record<string, unknown>, key: string): string => e[key] as string;
const b = (e: Record<string, unknown>, key: string): boolean => e[key] as boolean;
const list = (e: Record<string, unknown>, key: string): number[] => e[key] as number[];

export const GH3220_COMMAND_CATALOG: CommandDef[] = [
  // —— 状态与信息 ——
  { key: 'getStatus', cmd: '--', group: 'status', params: [], invoke: () => gh3220Api.getStatus() },
  {
    key: 'getDeviceInfo',
    cmd: hexCode(C.GET_VER),
    group: 'status',
    params: [],
    invoke: () => gh3220Api.getDeviceInfo(),
  },
  { key: 'getStats', cmd: '--', group: 'status', params: [], invoke: () => gh3220Api.getStats() },
  {
    key: 'getCaptureStatus',
    cmd: '--',
    group: 'status',
    params: [],
    invoke: () => gh3220Api.getCaptureStatus(),
  },
  {
    key: 'getPreferences',
    cmd: '--',
    group: 'config',
    params: [],
    invoke: () => gh3220Api.getPreferences(),
  },
  { key: 'clearStats', cmd: '--', group: 'status', params: [], invoke: () => gh3220Api.clearStats() },

  // —— 采样控制 ——
  {
    key: 'setWorkMode',
    cmd: hexCode(C.WORK_MODE),
    group: 'sampling',
    params: [mode(), funcMask()],
    invoke: (e) => gh3220Api.setWorkMode(n(e, 'mode'), n(e, 'funcMask')),
  },
  {
    key: 'start',
    cmd: hexCode(C.START_CTRL),
    group: 'sampling',
    params: [funcMask(), slotEn()],
    invoke: (e) => gh3220Api.start(n(e, 'funcMask'), n(e, 'slotEn') ?? 0),
  },
  {
    key: 'stop',
    cmd: hexCode(C.START_CTRL),
    group: 'sampling',
    params: [funcMask()],
    danger: true,
    confirmKey: 'stop',
    invoke: (e) => gh3220Api.stop(n(e, 'funcMask')),
  },
  {
    key: 'switchSlotFunc',
    cmd: hexCode(C.SWITCH_SLOT_FUNC),
    group: 'sampling',
    params: [slot(), extra(), funcMask(), bool('on', 'true')],
    invoke: (e) =>
      gh3220Api.switchSlotFunc(n(e, 'slot'), n(e, 'extra'), n(e, 'funcMask'), b(e, 'on')),
  },
  {
    key: 'captureStart',
    cmd: `${hexCode(C.WORK_MODE)}+${hexCode(C.START_CTRL)}`,
    group: 'sampling',
    params: [
      bool('autoStart', 'true'),
      bool('autoStop', 'false'),
      mode(),
      slotEn(),
      funcMask(),
      channelsOverride(),
      bool('csvEnabled', 'true'),
    ],
    invoke: (e) => {
      const config: Gh3220CaptureConfig = {
        autoStart: b(e, 'autoStart'),
        autoStop: b(e, 'autoStop'),
        mode: n(e, 'mode'),
        slotEn: n(e, 'slotEn') ?? 0,
        funcMask: n(e, 'funcMask'),
        channelsOverride: (e.channelsOverride as number | null) ?? null,
        csvEnabled: b(e, 'csvEnabled'),
      };
      return gh3220Api.captureStart(config);
    },
  },
  {
    key: 'captureStop',
    cmd: '--',
    group: 'sampling',
    params: [],
    danger: true,
    confirmKey: 'captureStop',
    invoke: () => gh3220Api.captureStop(),
  },

  // —— 寄存器与芯片 ——
  {
    key: 'regRead',
    cmd: hexCode(C.REG_RW),
    group: 'register',
    params: [addr(), bounded('count', '1', (ctx) => boundedOptions('count', ctx.lang), 1, 64)],
    invoke: (e) => gh3220Api.regRead(n(e, 'addr'), n(e, 'count')),
  },
  {
    key: 'regWrite',
    cmd: hexCode(C.REG_RW),
    group: 'register',
    params: [addr(), values()],
    invoke: (e) => gh3220Api.regWrite(n(e, 'addr'), list(e, 'values')),
  },
  {
    key: 'loadRegList',
    cmd: hexCode(C.LOAD_REG_LIST),
    group: 'register',
    params: [makeParam('filePath', 'file', '')],
    invoke: (e) => gh3220Api.loadRegList(s(e, 'filePath')),
  },
  {
    key: 'chipCtrl',
    cmd: hexCode(C.CHIP_CTRL),
    group: 'register',
    params: [
      makeParam('action', 'enum', 'wakeup', { options: (ctx) => chipActionOptions(ctx.lang) }),
    ],
    confirmKey: 'chipReset',
    confirmWhen: (e) => String(e.action ?? '').includes('reset'),
    invoke: (e) => gh3220Api.chipCtrl(s(e, 'action')),
  },
  {
    key: 'setFifoThr',
    cmd: hexCode(C.FIFO_THR_CONFIG),
    group: 'register',
    params: [
      bounded('value', '0', (ctx) => boundedOptions('fifoThr', ctx.lang), 0, 65535),
    ],
    invoke: (e) => gh3220Api.setFifoThr(n(e, 'value') ?? 0),
  },
  {
    key: 'setReportEvent',
    cmd: hexCode(C.REPORT_EVENT_SET),
    group: 'register',
    params: [
      makeParam('mask', 'bitmask', [], {
        options: (ctx) => eventMaskOptions(ctx.lang),
      }),
    ],
    invoke: (e) => gh3220Api.setReportEvent(n(e, 'mask')),
  },

  // —— 调试与原始帧 ——
  {
    key: 'debugStatus',
    cmd: hexCode(C.DEBUG_STATUS),
    group: 'debug',
    params: [
      makeParam('kind', 'enum', 'status', { options: (ctx) => debugKindOptions(ctx.lang) }),
    ],
    invoke: (e) => gh3220Api.debugStatus(s(e, 'kind')),
  },
  {
    key: 'packageTest',
    cmd: hexCode(C.PACKAGE_TEST),
    group: 'debug',
    params: [hex()],
    invoke: (e) => gh3220Api.packageTest(s(e, 'hex')),
  },
  {
    key: 'sendRaw',
    cmd: hexCode(C.NEW_STRUCT_RAWDATA),
    group: 'debug',
    params: [hex('AA 11 19 01 01')],
    invoke: (e) => gh3220Api.sendRaw(s(e, 'hex')),
  },

  // —— 配置 ——
  {
    key: 'updatePreferences',
    cmd: '--',
    group: 'config',
    params: [],
    invoke: async () => {
      const prefs = await gh3220Api.getPreferences();
      await gh3220Api.updatePreferences(prefs);
      return prefs;
    },
  },
];

/** 目录 key 与 `gh3220Api` 方法名的对照清单（单测断言一致性） */
export const GH3220_COMMAND_API_KEYS: string[] = GH3220_COMMAND_CATALOG.map((def) => def.key);

/** 22 行的默认语义值（组件初始 state） */
export const DEFAULT_COMMAND_VALUES: Record<string, Record<string, ParamRaw>> =
  GH3220_COMMAND_CATALOG.reduce<Record<string, Record<string, ParamRaw>>>((acc, def) => {
    acc[def.key] = def.params.reduce<Record<string, ParamRaw>>((row, param) => {
      row[param.key] = Array.isArray(param.defaultValue)
        ? [...param.defaultValue]
        : param.defaultValue;
      return row;
    }, {});
    return acc;
  }, {});

export function defaultRowValues(): Record<string, Record<string, ParamRaw>> {
  return Object.fromEntries(
    Object.entries(DEFAULT_COMMAND_VALUES).map(([key, row]) => [key, { ...row }])
  );
}

/** 校验整行；返回第一条错误文案（无错误返回 null） */
export function validateCommand(
  def: CommandDef,
  values: Record<string, ParamRaw>,
  ctx: OptionContext
): string | null {
  for (const param of def.params) {
    const raw = values[param.key] ?? param.defaultValue;
    const error = param.validate?.(raw, ctx) ?? null;
    if (error) return `${param.key}: ${error}`;
  }
  return null;
}

/** 语义值 → invoke 参数（唯一的数值出口） */
export function encodeCommand(
  def: CommandDef,
  values: Record<string, ParamRaw>,
  ctx: OptionContext
): Record<string, unknown> {
  return def.params.reduce<Record<string, unknown>>((acc, param) => {
    const raw = values[param.key] ?? param.defaultValue;
    acc[param.key] = param.encode(raw, ctx);
    return acc;
  }, {});
}

/** 是否需要二次确认 */
export function needsConfirm(def: CommandDef, encoded: Record<string, unknown>): boolean {
  return Boolean(def.danger) || Boolean(def.confirmWhen?.(encoded));
}

function formatWireValue(key: string, value: unknown): string {
  if (value === null) return 'null';
  if (key === 'funcMask' || key === 'mask') {
    return `0x${(Number(value) >>> 0).toString(16).toUpperCase()}`;
  }
  if (typeof value === 'number') return String(value);
  if (typeof value === 'boolean') return String(value);
  if (Array.isArray(value)) return `[${value.join(',')}]`;
  return JSON.stringify(value);
}

/** 语义值描述（UI 可读） */
export function describeSemantic(
  def: CommandDef,
  values: Record<string, ParamRaw>,
  ctx: OptionContext
): string {
  if (def.params.length === 0) return '—';
  return def.params
    .map((param) => {
      const raw = values[param.key] ?? param.defaultValue;
      if (param.kind === 'functionMask') {
        return `${param.key}=${formatFunctionMaskPreview(encodeFunctionMask(asArray(raw)), ctx.lang)}`;
      }
      if (param.kind === 'bitmask') {
        return `${param.key}=${formatEventMaskPreview(encodeEventMask(asArray(raw)), ctx.lang)}`;
      }
      if (param.kind === 'bool') {
        return `${param.key}=${asText(raw) === 'true' ? (ctx.lang === 'zh' ? '是' : 'Yes') : ctx.lang === 'zh' ? '否' : 'No'}`;
      }
      const option = resolveOptions(param, ctx).find((item) => item.value === asText(raw));
      const label = option?.label ?? (isCustom(raw) ? customValue(raw) : asText(raw));
      return `${param.key}=${label}`;
    })
    .join(' · ');
}

/** 语义值 + 实际下发数值，供日志双列使用 */
export function describeCommand(
  def: CommandDef,
  values: Record<string, ParamRaw>,
  ctx: OptionContext
): { semantic: string; wire: string } {
  const encoded = encodeCommand(def, values, ctx);
  const args = Object.entries(encoded)
    .map(([key, value]) => `${key}:${formatWireValue(key, value)}`)
    .join(', ');
  return {
    semantic: describeSemantic(def, values, ctx),
    wire: `${def.key}(${args}) · CMD ${def.cmd}`,
  };
}

/** 供通道配置 / 版本页复用的选项集合 */
export const GH3220_SELECT_OPTIONS = {
  mtu: (lang: Gh3220Lang) => [
    { value: 'auto', label: lang === 'zh' ? '自动（后端按 max_write_len）' : 'Auto (backend max_write_len)' },
    ...['20', '23', '64', '128', '244', '517'].map((value) => ({ value, label: `${value} B` })),
    { value: 'custom', label: lang === 'zh' ? '自定义…' : 'Custom…' },
  ],
  tcpAddress: (lang: Gh3220Lang) => [
    ...['127.0.0.1:9000', 'localhost:9000', '[::1]:9000'].map((value) => ({ value, label: value })),
    { value: 'custom', label: lang === 'zh' ? '自定义…' : 'Custom…' },
  ],
  idleCloseSecs: (lang: Gh3220Lang) => boundedOptions('idleCloseSecs', lang, ' s'),
  sampleRate: (lang: Gh3220Lang) => boundedOptions('sampleRate', lang, ' Hz'),
  displayDuration: (lang: Gh3220Lang) => boundedOptions('displayDuration', lang, ' s'),
  versionType: versionTypeOptions,
  uuid: (defaults: string[], discovered: string[], lang: Gh3220Lang) => {
    const merged = Array.from(new Set([...defaults, ...discovered]));
    return [
      ...merged.map((value) => ({ value, label: value })),
      { value: 'custom', label: lang === 'zh' ? '自定义…' : 'Custom…' },
    ];
  },
};

export { CUSTOM_PREFIX };
