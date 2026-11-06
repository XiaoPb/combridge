import { describe, expect, it } from 'vitest';
import { parseFuncMask } from '../types/gh3220';
import {
  formatBitNames,
  formatBufferLabel,
  formatEventName,
  formatFunctionLabel,
  formatFunctionMaskNames,
  formatWorkModeLabel,
  gh3220Crc8,
  GH3220_EVENT_MAP,
  GH3220_FUNCTION_MAP,
  GH3220_HEX_TEMPLATES,
  GH3220_RAW_FRAME_TEMPLATES,
  GH3220_WORK_MODE_MAP,
  isValidUProtocolFrame,
  parseDataType,
  parseFrameHeader,
  parseFunctionMaskText,
  parsePackageType,
  resolveGh3220Lang,
} from './gh3220Functions';

describe('GH3220 语义映射', () => {
  it('func id 与 SDK 功能位对齐，真机观察的 func0/func1 有语义名', () => {
    expect(GH3220_FUNCTION_MAP[0].abbr).toBe('ADT');
    expect(GH3220_FUNCTION_MAP[1].abbr).toBe('HR');
    expect(GH3220_FUNCTION_MAP[1].mask).toBe(0x02);
    expect(GH3220_FUNCTION_MAP[19].mask).toBe(0x80000);
    expect(formatFunctionLabel(1, 4, 'zh')).toBe('心率 HR (4 CH)');
    expect(formatFunctionLabel(0, 1, 'zh')).toBe('佩戴检测 ADT (1 CH)');
    expect(formatFunctionLabel(99, 2, 'zh')).toBe('功能 func99 (2 CH)');
    expect(formatFunctionLabel(99, 2, 'en')).toBe('Function func99 (2 CH)');
  });

  it('WORK_MODE / 事件码映射完整，0x0008 标注为固件私有扩展', () => {
    expect(GH3220_WORK_MODE_MAP[2].sdkEnum).toBe('MCU_ONLINE_MODE');
    expect(formatWorkModeLabel(2, 'zh')).toContain('MCU 在线模式');
    expect(formatWorkModeLabel(99, 'zh')).toContain('未知模式');
    expect(GH3220_EVENT_MAP[0x0008].key).toBe('fast_recovery');
    expect(GH3220_EVENT_MAP[0x0008].note).toContain('固件私有扩展');
    expect(formatEventName(0x0008, 'zh')).toBe('快速恢复');
    expect(formatEventName(0x0402, 'zh')).toBe('心电导联接上 / 佩戴上');
    expect(formatEventName(0x8000, 'zh')).toBe('0x8000');
  });

  it('mask → 名称列表，未知位以 0x 兜底', () => {
    expect(formatFunctionMaskNames(0x02, 'zh')).toEqual(['心率 HR']);
    expect(formatFunctionMaskNames(0x80000000, 'zh')).toEqual(['0x80000000']);
  });
});

describe('GH3220 帧头位解析（F4）', () => {
  it('data_type / package_type 位返回可读名称', () => {
    expect(formatBitNames(parseDataType(0x03), 'zh')).toBe('GS 加速度 / 算法结果');
    expect(formatBitNames(parsePackageType(0x80), 'zh')).toBe('新数据/算法结果');
    expect(formatBitNames(parseDataType(0x00), 'en')).toBe('None');
  });

  it('解析 8 字节包头，支持以 0B CMD 开头与空格分隔', () => {
    const header = parseFrameHeader('01 03 00 00 00 0F 80 10');
    expect(header).not.toBeNull();
    expect(header?.functionId).toBe(1);
    expect(header?.dataType).toBe(0x03);
    expect(header?.channelMask).toBe(0x0f);
    expect(header?.channelCount).toBe(4);
    expect(header?.packageType).toBe(0x80);
    expect(header?.dataLen).toBe(0x10);
    expect(header?.dataTypeItems.map((item) => item.key)).toEqual(['gs', 'algo']);
    expect(header?.packageTypeItems.map((item) => item.key)).toEqual(['new_data']);
    expect(parseFrameHeader('0B 01 03 00 00 00 0F 80 10')?.functionId).toBe(1);
    expect(parseFrameHeader('0103')).toBeNull();
    expect(parseFrameHeader('')).toBeNull();
  });
});

describe('GH3220 功能位文本解析（F5）', () => {
  it('支持语义名、历史别名、十六进制与十进制数值', () => {
    expect(parseFunctionMaskText('hr,spo2')).toBe(0x42);
    expect(parseFunctionMaskText('hr hrv lead_det')).toBe(0x02 | 0x04 | 0x80000);
    expect(parseFunctionMaskText('bp')).toBe(1 << 4);
    expect(parseFunctionMaskText('0x6')).toBe(0x06);
    expect(parseFunctionMaskText('6')).toBe(0x06);
    expect(parseFunctionMaskText('')).toBe(0);
    expect(() => parseFunctionMaskText('unknown_func')).toThrow('未知功能');
    // parseFuncMask 委托语义层，旧调用保持兼容
    expect(parseFuncMask('hr,spo2')).toBe(0x42);
  });
});

describe('GH3220 展示层标注', () => {
  it('缓冲 ID 保持稳定，仅追加语义名', () => {
    expect(formatBufferLabel('gh3220:func1', 'zh')).toBe('gh3220:func1 · 心率 HR');
    expect(formatBufferLabel('serial:raw', 'zh')).toBe('serial:raw');
  });

  it('语种解析按 i18next language 收敛', () => {
    expect(resolveGh3220Lang('zh-CN')).toBe('zh');
    expect(resolveGh3220Lang('en-US')).toBe('en');
    expect(resolveGh3220Lang(undefined)).toBe('en');
  });
});

describe('sendRaw 完整帧模板（真机回归）', () => {
  it('CRC8 与 gh-uprotocol 一致', () => {
    expect(gh3220Crc8([0xaa, 0x11, 0x16, 0x01, 0x02])).toBe(0xa2);
  });

  it('三条 sendRaw 模板都能被 Frame::decode 接受', () => {
    expect(GH3220_RAW_FRAME_TEMPLATES.length).toBe(3);
    for (const template of GH3220_RAW_FRAME_TEMPLATES) {
      expect(isValidUProtocolFrame(template.hex)).toBe(true);
    }
  });

  it('历史缺 CRC 的模板与帧头/回显示例被识别为非法完整帧', () => {
    expect(isValidUProtocolFrame('AA 11 19 01 01')).toBe(false);
    for (const template of GH3220_HEX_TEMPLATES) {
      expect(isValidUProtocolFrame(template.hex)).toBe(false);
    }
  });
});
