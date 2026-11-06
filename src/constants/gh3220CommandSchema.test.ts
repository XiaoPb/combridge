import { describe, expect, it, vi } from 'vitest';

vi.mock('../api/tauri', () => ({ invoke: vi.fn() }));

import { gh3220Api } from '../api/gh3220';
import {
  boundedOptions,
  channelOptions,
  customOption,
  decodeEventMask,
  decodeFunctionMask,
  encodeEventMask,
  encodeFunctionMask,
  eventMaskOptions,
  formatEventMaskPreview,
  formatFunctionMaskPreview,
  functionOptions,
  GH3220_CHIP_ACTIONS,
  GH3220_COMMAND_CODES,
  GH3220_DEBUG_STATUS_KINDS,
  GH3220_EVENT_LIST,
  GH3220_FUNCTION_LIST,
  GH3220_VERSION_TYPES,
  maxChannelsForFunctions,
  parseFunctionMaskText,
  workModeOptions,
} from './gh3220Functions';
import {
  DEFAULT_COMMAND_VALUES,
  describeCommand,
  encodeBoundedNumber,
  encodeCommand,
  encodeHex,
  encodeNumberList,
  GH3220_COMMAND_API_KEYS,
  GH3220_COMMAND_CATALOG,
  needsConfirm,
  validateCommand,
  type OptionContext,
} from './gh3220CommandSchema';

const ctx: OptionContext = { lang: 'zh', selectedFunctions: ['hr'] };
const find = (key: string) => {
  const def = GH3220_COMMAND_CATALOG.find((item) => item.key === key);
  if (!def) throw new Error(`missing command ${key}`);
  return def;
};
const row = (key: string) => ({ ...DEFAULT_COMMAND_VALUES[key] });

describe('V3 语义层：功能位多选编解码', () => {
  it('单选 / 多选按全表 mask 归约', () => {
    expect(encodeFunctionMask(['hr'])).toBe(0x0002);
    expect(encodeFunctionMask(['hr', 'spo2'])).toBe(0x0042);
    expect(encodeFunctionMask(['adt', 'ecg'])).toBe(0x0081);
  });

  it('15 项全选与文本解析结果一致', () => {
    const all = GH3220_FUNCTION_LIST.map((info) => info.key);
    expect(encodeFunctionMask(all)).toBe(parseFunctionMaskText(all.join(',')));
  });

  it('历史别名兼容（bp / ir / sadt_g / lead_det）', () => {
    expect(encodeFunctionMask(['bp'])).toBe(1 << 4);
    expect(encodeFunctionMask(['ir', 'sadt_g', 'lead_det'])).toBe(
      (1 << 15) | (1 << 9) | (1 << 19)
    );
  });

  it('空数组返回 0，未知文本抛错', () => {
    expect(encodeFunctionMask([])).toBe(0);
    expect(() => encodeFunctionMask(['not_a_function'])).toThrow(/未知功能/);
  });

  it('未知 mask 位以 0x 占位项保留，decode(encode(x)) 往返一致', () => {
    const keys = ['hr', '0x2000'];
    const mask = encodeFunctionMask(keys);
    expect(mask).toBe(0x2002);
    expect(decodeFunctionMask(mask)).toEqual(expect.arrayContaining(['hr', '0x2000']));
    expect(decodeFunctionMask(0x2)).toEqual(['hr']);
    expect(encodeFunctionMask(decodeFunctionMask(0x42))).toBe(0x42);
  });

  it('预览包含十六进制掩码与语义名', () => {
    expect(formatFunctionMaskPreview(0x42, 'zh')).toContain('0x0042');
    expect(formatFunctionMaskPreview(0x42, 'zh')).toContain('心率 HR');
    expect(formatFunctionMaskPreview(0x42, 'en')).toContain('Heart Rate HR');
  });

  it('功能选项 15 项且双语 label 非空', () => {
    expect(functionOptions('zh')).toHaveLength(15);
    expect(functionOptions('zh').every((item) => item.label.length > 0)).toBe(true);
    expect(functionOptions('en').every((item) => item.label.length > 0)).toBe(true);
    expect(GH3220_FUNCTION_LIST.length).toBe(15);
  });
});

describe('V3 语义层：事件位与枚举表', () => {
  it('14 项事件位 OR 正确，空掩码为 0', () => {
    expect(GH3220_EVENT_LIST).toHaveLength(14);
    expect(encodeEventMask(GH3220_EVENT_LIST.map((item) => String(item.bit)))).toBe(0x7f7f);
    expect(encodeEventMask([])).toBe(0);
    expect(encodeEventMask(['1', '2', '64'])).toBe(0x43);
  });

  it('事件语义键与未知数值位都支持，u16 溢出拒绝', () => {
    expect(encodeEventMask(['chip_wakeup', 'wear_on'])).toBe(0x0401);
    expect(encodeEventMask(['0x8000'])).toBe(0x8000);
    expect(() => encodeEventMask(['0x10000'])).toThrow(/u16/);
    expect(() => encodeEventMask(['nope'])).toThrow(/未知事件位/);
  });

  it('事件掩码预览与 decode 保留未知位', () => {
    expect(formatEventMaskPreview(0x0401, 'zh')).toContain('佩戴上');
    expect(decodeEventMask(0x8001)).toEqual(expect.arrayContaining(['1', '0x8000']));
    expect(eventMaskOptions('en')).toHaveLength(14);
  });

  it('调试类型 / 芯片动作 / 版本类型 / 工作模式表完整', () => {
    expect(GH3220_DEBUG_STATUS_KINDS).toHaveLength(6);
    expect(GH3220_CHIP_ACTIONS).toHaveLength(5);
    expect(GH3220_VERSION_TYPES).toHaveLength(9);
    expect(workModeOptions('zh')).toHaveLength(8);
    expect(GH3220_COMMAND_CODES.SWITCH_SLOT_FUNC).toBe(0x1c);
    expect(GH3220_COMMAND_CODES.FIFO_THR_CONFIG).toBe(0x12);
    expect(GH3220_COMMAND_CODES.REPORT_EVENT_SET).toBe(0x13);
    expect(GH3220_COMMAND_CODES.LOAD_REG_LIST).toBe(0xa1);
  });
});

describe('V3 语义层：动态选项与自定义值', () => {
  it('通道覆盖上限随所选功能变化', () => {
    expect(maxChannelsForFunctions(['adt'])).toBe(8);
    expect(maxChannelsForFunctions(['hr'])).toBe(32);
    expect(maxChannelsForFunctions([])).toBe(32);
    expect(channelOptions(['adt'], 'zh')).toHaveLength(10); // auto + 8 + custom
    expect(channelOptions(['hr'], 'zh')).toHaveLength(34);
  });

  it('boundedNumber 档位与 custom 编码', () => {
    expect(boundedOptions('slot', 'zh')).toHaveLength(9); // 0..7 + custom
    expect(encodeBoundedNumber('auto')).toBeNull();
    expect(encodeBoundedNumber('4')).toBe(4);
    expect(encodeBoundedNumber(customOption(42))).toBe(42);
    expect(encodeHex('AA 11 19 01 01')).toBe('AA11190101');
    expect(encodeNumberList('0x1234, 0xabcd')).toEqual([0x1234, 0xabcd]);
  });
});

describe('V3 命令目录一致性', () => {
  it('22 条命令 key 唯一，且与 gh3220Api 方法一一对应', () => {
    expect(GH3220_COMMAND_CATALOG).toHaveLength(22);
    expect(new Set(GH3220_COMMAND_API_KEYS).size).toBe(22);
    for (const key of GH3220_COMMAND_API_KEYS) {
      expect(typeof (gh3220Api as unknown as Record<string, unknown>)[key]).toBe('function');
    }
    expect(Object.keys(gh3220Api).filter((key) => !GH3220_COMMAND_API_KEYS.includes(key))).toEqual([
      'connect',
      'disconnect',
    ]);
  });

  it('CMD 号按协议修正，危险命令集合准确', () => {
    expect(find('switchSlotFunc').cmd).toBe('0x1C');
    expect(find('setFifoThr').cmd).toBe('0x12');
    expect(find('setReportEvent').cmd).toBe('0x13');
    expect(find('loadRegList').cmd).toBe('0xA1');
    expect(find('start').cmd).toBe('0x0C');
    expect(find('captureStart').cmd).toBe('0x10+0x0C');
    const dangerous = GH3220_COMMAND_CATALOG.filter((def) => def.danger).map((def) => def.key);
    expect(dangerous.sort()).toEqual(['captureStop', 'stop']);
    expect(needsConfirm(find('chipCtrl'), { action: 'hard-reset' })).toBe(true);
    expect(needsConfirm(find('chipCtrl'), { action: 'wakeup' })).toBe(false);
  });

  it('每条命令的 defaultValue 覆盖全部参数', () => {
    for (const def of GH3220_COMMAND_CATALOG) {
      expect(DEFAULT_COMMAND_VALUES[def.key]).toBeDefined();
      for (const param of def.params) {
        expect(DEFAULT_COMMAND_VALUES[def.key][param.key]).toBeDefined();
      }
    }
  });

  it('表格内不存在 functionMask 文本输入（kind 全部为选择类）', () => {
    const textKinds = GH3220_COMMAND_CATALOG.flatMap((def) => def.params.map((p) => p.kind));
    expect(textKinds).not.toContain('text');
    expect(GH3220_COMMAND_CATALOG.find((d) => d.key === 'start')?.params[0].kind).toBe('functionMask');
  });
});

describe('V3 编码 / 校验 / 日志双列', () => {
  it('功能多选编码成 u32，日志 wire 显示实际数值', () => {
    const def = find('setWorkMode');
    const values = { ...row('setWorkMode'), funcMask: ['hr', 'spo2'], mode: '2' };
    expect(validateCommand(def, values, ctx)).toBeNull();
    expect(encodeCommand(def, values, ctx)).toEqual({ mode: 2, funcMask: 0x42 });
    const desc = describeCommand(def, values, ctx);
    expect(desc.semantic).toContain('心率 HR');
    expect(desc.wire).toContain('funcMask:0x42');
  });

  it('必选功能为空 / 掩码为 0 时校验失败', () => {
    const def = find('start');
    expect(validateCommand(def, { ...row('start'), funcMask: [] }, ctx)).toMatch(/至少选择/);
  });

  it('bool / boundedNumber / bitmask / enum 编码正确', () => {
    const start = find('start');
    expect(encodeCommand(start, row('start'), ctx)).toEqual({ funcMask: 0x2, slotEn: 0 });

    const capture = find('captureStart');
    expect(encodeCommand(capture, row('captureStart'), ctx)).toMatchObject({
      autoStart: true,
      autoStop: false,
      mode: 2,
      funcMask: 0x2,
      channelsOverride: null,
      csvEnabled: true,
    });

    const events = find('setReportEvent');
    expect(encodeCommand(events, { mask: ['chip_wakeup', 'wear_on'] }, ctx)).toEqual({ mask: 0x401 });

    const debug = find('debugStatus');
    expect(encodeCommand(debug, row('debugStatus'), ctx)).toEqual({ kind: 'status' });
    const chip = find('chipCtrl');
    expect(encodeCommand(chip, row('chipCtrl'), ctx)).toEqual({ action: 'wakeup' });
  });

  it('HEX 格式、文件路径、数值列表校验', () => {
    const pkg = find('packageTest');
    expect(validateCommand(pkg, { hex: '0102' }, ctx)).toBeNull();
    expect(validateCommand(pkg, { hex: '0' }, ctx)).toMatch(/偶数位/);
    expect(validateCommand(pkg, { hex: 'ZZ' }, ctx)).toMatch(/偶数位|十六进制/);

    const load = find('loadRegList');
    expect(validateCommand(load, { filePath: '' }, ctx)).toMatch(/选择文件/);
    expect(validateCommand(load, { filePath: 'C:/regs.csv' }, ctx)).toBeNull();

    const write = find('regWrite');
    expect(encodeCommand(write, { addr: '0x0010', values: '0x1234, 0xabcd' }, ctx)).toEqual({
      addr: 16,
      values: [0x1234, 0xabcd],
    });
    expect(validateCommand(write, { addr: '0x0010', values: '' }, ctx)).toMatch(/数值列表/);
  });

  it('boundedNumber 越界与 custom 值校验', () => {
    const fifo = find('setFifoThr');
    expect(validateCommand(fifo, { value: customOption(70000) }, ctx)).toMatch(/不能大于/);
    expect(validateCommand(fifo, { value: customOption(256) }, ctx)).toBeNull();
    expect(encodeCommand(fifo, { value: customOption(256) }, ctx)).toEqual({ value: 256 });
  });
});
