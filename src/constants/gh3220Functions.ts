/**
 * GH3220 协议语义层（对齐计划 `docs/gh3220-ui-alignment-v2-plan.md` P0）
 *
 * 数据与 SDK 出处：
 * - 功能位：gh3220_sdk/demo_kernel_code/driver/inc/gh_drv.h L364-410，
 *   帧头 function_id 转换见 module/gh_protocol/gh_uprotocol.c L1491-1498
 *   （func id === 功能位掩码的 bit 序号）。
 * - WORK_MODE：module/gh_protocol/gh_uprotocol.c L977-1001、
 *   driver/inc/gh_drv.h L90-103 / L442-449（mode = payload[0] + UPROTOCOL_CMD_EVK_MODE）。
 * - 事件码：driver/inc/gh_drv.h L334-348；上报掩码过滤见
 *   kernel/gh_demo_protocol.c L623。
 * - 0x0B 帧头位定义：module/gh_protocol/gh_uprotocol.h L194-213。
 * - 各功能通道数上限：kernel/gh_demo_config.h L168-181。
 *
 * 约定：本模块只做「值 → 可读名称」的展示层解析，缓冲区 ID、CSV 命名、
 * 下发给设备的原始数值一律保持不变。
 */

export type Gh3220Lang = 'zh' | 'en';

/** 把 i18next 的 language 收敛成本模块支持的语种 */
export function resolveGh3220Lang(language?: string): Gh3220Lang {
  return language && language.toLowerCase().startsWith('zh') ? 'zh' : 'en';
}

export interface Gh3220FunctionInfo {
  /** 0x0B 帧包头首字节 function_id，等于功能位掩码的 bit 序号 */
  id: number;
  /** 语义文本键（parseFuncMask 使用） */
  key: string;
  /** UI 缩写 */
  abbr: string;
  nameZh: string;
  nameEn: string;
  /** SDK 宏对应的功能位掩码 */
  mask: number;
  /** gh_demo_config.h 中的通道数上限，未列出则不填 */
  maxChannels?: number;
  sdkMacro: string;
}

/**
 * F1 功能位映射全表。
 * 真机观察：func0 = ADT(1 CH)、func1 = HR(4 CH)，与 SDK 一致。
 * id 15 / 19 的 mask 按「bit 序号」规则推导，SDK 未直接给出宏值。
 */
export const GH3220_FUNCTION_LIST: Gh3220FunctionInfo[] = [
  { id: 0, key: 'adt', abbr: 'ADT', nameZh: '佩戴检测', nameEn: 'Wear Detection', mask: 0x0001, maxChannels: 8, sdkMacro: 'GH3X2X_FUNCTION_ADT' },
  { id: 1, key: 'hr', abbr: 'HR', nameZh: '心率', nameEn: 'Heart Rate', mask: 0x0002, maxChannels: 32, sdkMacro: 'GH3X2X_FUNCTION_HR' },
  { id: 2, key: 'hrv', abbr: 'HRV', nameZh: '心率变异性', nameEn: 'HRV', mask: 0x0004, sdkMacro: 'GH3X2X_FUNCTION_HRV' },
  { id: 3, key: 'hsm', abbr: 'HSM', nameZh: '心脏健康监测', nameEn: 'Heart Health Monitoring', mask: 0x0008, sdkMacro: 'GH3X2X_FUNCTION_HSM' },
  { id: 4, key: 'fpbp', abbr: 'FPBP', nameZh: '指尖 PPG 血压', nameEn: 'Finger PPG Blood Pressure', mask: 0x0010, sdkMacro: 'GH3X2X_FUNCTION_FPBP' },
  { id: 5, key: 'pwa', abbr: 'PWA', nameZh: '脉搏波分析', nameEn: 'Pulse Wave Analysis', mask: 0x0020, sdkMacro: 'GH3X2X_FUNCTION_PWA' },
  { id: 6, key: 'spo2', abbr: 'SPO2', nameZh: '血氧', nameEn: 'SpO2', mask: 0x0040, maxChannels: 32, sdkMacro: 'GH3X2X_FUNCTION_SPO2' },
  { id: 7, key: 'ecg', abbr: 'ECG', nameZh: '心电（强制无 AGC）', nameEn: 'ECG (AGC forced off)', mask: 0x0080, maxChannels: 8, sdkMacro: 'GH3X2X_FUNCTION_ECG' },
  { id: 8, key: 'pwtt', abbr: 'PWTT', nameZh: '脉搏波传导时间', nameEn: 'Pulse Wave Transit Time', mask: 0x0100, sdkMacro: 'GH3X2X_FUNCTION_PWTT' },
  { id: 9, key: 'soft_adt_green', abbr: 'SADT-G', nameZh: '软件佩戴检测(绿光)', nameEn: 'Soft Wear Detection (Green)', mask: 0x0200, sdkMacro: 'GH3X2X_FUNCTION_SOFT_ADT_GREEN' },
  { id: 10, key: 'bt', abbr: 'BT', nameZh: '血压趋势 (BPTT)', nameEn: 'Blood Pressure Trend', mask: 0x0400, sdkMacro: 'GH3X2X_FUNCTION_BT' },
  { id: 11, key: 'resp', abbr: 'RESP', nameZh: '呼吸', nameEn: 'Respiration', mask: 0x0800, sdkMacro: 'GH3X2X_FUNCTION_RESP' },
  { id: 12, key: 'af', abbr: 'AF', nameZh: '房颤检测', nameEn: 'Atrial Fibrillation', mask: 0x1000, sdkMacro: 'GH3X2X_FUNCTION_AF' },
  { id: 15, key: 'soft_adt_ir', abbr: 'SADT-IR', nameZh: '软件佩戴检测(红外)', nameEn: 'Soft Wear Detection (IR)', mask: 0x8000, sdkMacro: 'GH3X2X_FUNCTION_SOFT_ADT_IR' },
  { id: 19, key: 'lead_det', abbr: 'LEAD_DET', nameZh: '心电导联检测', nameEn: 'ECG Lead Detection', mask: 0x80000, sdkMacro: '(SDK 未定义宏，按 bit 序号推导)' },
];

export const GH3220_FUNCTION_MAP: Record<number, Gh3220FunctionInfo> =
  GH3220_FUNCTION_LIST.reduce<Record<number, Gh3220FunctionInfo>>((acc, info) => {
    acc[info.id] = info;
    return acc;
  }, {});

/** 语义文本 → bit 序号；包含历史别名，保证旧输入 `hr,spo2` 等继续可用 */
export const GH3220_FUNCTION_BIT_ALIASES: Record<string, number> = (() => {
  const aliases: Record<string, number> = {
    // 历史别名（types/gh3220.ts 原 GH3220_FUNCTION_BITS）
    bp: 4,
    pwa: 5,
    green: 9,
    soft_adt_green: 9,
    sadt_g: 9,
    bptt: 10,
    test1: 13,
    test2: 14,
    ir: 15,
    soft_adt_ir: 15,
    sadt_ir: 15,
    lead_det: 19,
  };
  for (const info of GH3220_FUNCTION_LIST) {
    aliases[info.key] = info.id;
    aliases[info.abbr.toLowerCase().replace(/[^a-z0-9]+/g, '_')] = info.id;
  }
  return aliases;
})();

/** F2 WORK_MODE 模式值 → 语义名（数值仍原样下发给设备） */
export interface Gh3220WorkModeInfo {
  mode: number;
  key: string;
  nameZh: string;
  nameEn: string;
  sdkEnum: string;
}

export const GH3220_WORK_MODE_LIST: Gh3220WorkModeInfo[] = [
  { mode: 0, key: 'evk', nameZh: 'EVK 上位机调试模式', nameEn: 'EVK host debug mode', sdkEnum: 'EVK_MODE' },
  { mode: 1, key: 'app', nameZh: 'APP 蓝牙直连模式', nameEn: 'APP BLE direct mode', sdkEnum: 'APP_MODE' },
  { mode: 2, key: 'mcu_online', nameZh: 'MCU 在线模式（算法在主控）', nameEn: 'MCU online mode (algorithm on host)', sdkEnum: 'MCU_ONLINE_MODE' },
  { mode: 3, key: 'mcu_offline', nameZh: 'MCU 离线模式（芯片内算）', nameEn: 'MCU offline mode (on-chip algorithm)', sdkEnum: 'MCU_OFFLINE_MODE' },
  { mode: 4, key: 'test_tune', nameZh: '测试/调优模式', nameEn: 'Test / tuning mode', sdkEnum: 'TEST_TUNE_MODE' },
  { mode: 5, key: 'pass_through', nameZh: '透传模式', nameEn: 'Pass-through mode', sdkEnum: 'PASS_THROUGH_MODE' },
  { mode: 6, key: 'event_ack', nameZh: '事件上报 ACK', nameEn: 'Event report ACK', sdkEnum: 'IRQ_STATUS_REPORT_ACK' },
  { mode: 7, key: 'mpt', nameZh: 'MPT 多芯片并行模式', nameEn: 'MPT multi-chip parallel mode', sdkEnum: 'MPT_MODE' },
];

export const GH3220_WORK_MODE_MAP: Record<number, Gh3220WorkModeInfo> =
  GH3220_WORK_MODE_LIST.reduce<Record<number, Gh3220WorkModeInfo>>((acc, info) => {
    acc[info.mode] = info;
    return acc;
  }, {});

/** F3 事件位 → 语义名（全量录入，未知位 UI 兜底原始值） */
export interface Gh3220EventInfo {
  bit: number;
  key: string;
  nameZh: string;
  nameEn: string;
  sdkMacro: string;
  note?: string;
}

export const GH3220_EVENT_LIST: Gh3220EventInfo[] = [
  { bit: 0x0001, key: 'chip_wakeup', nameZh: '芯片唤醒', nameEn: 'Chip wakeup', sdkMacro: 'CHIP_WAKEUP' },
  { bit: 0x0002, key: 'lead_on', nameZh: '心电导联接上', nameEn: 'ECG lead on', sdkMacro: 'LEAD_ON' },
  { bit: 0x0004, key: 'lead_off', nameZh: '心电导联断开', nameEn: 'ECG lead off', sdkMacro: 'LEAD_OFF' },
  {
    bit: 0x0008,
    key: 'fast_recovery',
    nameZh: '快速恢复',
    nameEn: 'Fast recovery',
    sdkMacro: 'FAST_RECOVERY',
    // 真机观察到该事件，但 gh_demo_protocol.c L623 的上报掩码未包含，疑为固件私有扩展
    note: 'SDK 上报掩码外，真机观察到，疑为固件私有扩展',
  },
  { bit: 0x0010, key: 'adc_done', nameZh: 'ADC 采样完成', nameEn: 'ADC sampling done', sdkMacro: 'ADC_DONE' },
  { bit: 0x0020, key: 'fifo_watermark', nameZh: 'FIFO 阈值中断', nameEn: 'FIFO watermark', sdkMacro: 'FIFO_WATERMARK' },
  { bit: 0x0040, key: 'fifo_full', nameZh: 'FIFO 满', nameEn: 'FIFO full', sdkMacro: 'FIFO_FULL' },
  { bit: 0x0100, key: 'tunning_fail', nameZh: '调谐失败', nameEn: 'Tuning failed', sdkMacro: 'TUNNING_FAIL' },
  { bit: 0x0200, key: 'tunning_done', nameZh: '调谐完成', nameEn: 'Tuning done', sdkMacro: 'TUNNING_DONE' },
  { bit: 0x0400, key: 'wear_on', nameZh: '佩戴上', nameEn: 'Wear on', sdkMacro: 'WEAR_ON' },
  { bit: 0x0800, key: 'wear_off', nameZh: '摘下', nameEn: 'Wear off', sdkMacro: 'WEAR_OFF' },
  { bit: 0x1000, key: 'slot_timeout', nameZh: '槽位超时', nameEn: 'Slot timeout', sdkMacro: 'SLOT_TIMEOUT' },
  { bit: 0x2000, key: 'sample_rate_err', nameZh: '采样率错误', nameEn: 'Sample rate error', sdkMacro: 'SAMPLE_RATE_ERR' },
  { bit: 0x4000, key: 'chip_reset', nameZh: '芯片复位', nameEn: 'Chip reset', sdkMacro: 'CHIP_RESET' },
];

export const GH3220_EVENT_MAP: Record<number, Gh3220EventInfo> =
  GH3220_EVENT_LIST.reduce<Record<number, Gh3220EventInfo>>((acc, info) => {
    acc[info.bit] = info;
    return acc;
  }, {});

/** F4 帧头位定义与解析工具（gh_uprotocol.h L194-213） */
export interface Gh3220BitInfo {
  bit: number;
  value: number;
  key: string;
  nameZh: string;
  nameEn: string;
}

/** data_type：bit0=GS、bit1=算法结果、bit2=AGC、bit3=环境光、bit4=陀螺仪、bit5=CAP、bit6=TEMP */
export const GH3220_DATA_TYPE_BITS: Gh3220BitInfo[] = [
  { bit: 0, value: 0x01, key: 'gs', nameZh: 'GS 加速度', nameEn: 'GS accelerometer' },
  { bit: 1, value: 0x02, key: 'algo', nameZh: '算法结果', nameEn: 'Algorithm result' },
  { bit: 2, value: 0x04, key: 'agc', nameZh: 'AGC', nameEn: 'AGC' },
  { bit: 3, value: 0x08, key: 'ambient', nameZh: '环境光', nameEn: 'Ambient light' },
  { bit: 4, value: 0x10, key: 'gyro', nameZh: '陀螺仪', nameEn: 'Gyroscope' },
  { bit: 5, value: 0x20, key: 'cap', nameZh: 'CAP', nameEn: 'Capacitance' },
  { bit: 6, value: 0x40, key: 'temp', nameZh: 'TEMP', nameEn: 'Temperature' },
];

/** package_type：bit0=压缩包、bit1=奇偶帧、bit2+=FIFO 包模式、bit3=拼包计数、bit5=拼包结束、bit7=新数据/算法结果 */
export const GH3220_PACKAGE_TYPE_BITS: Gh3220BitInfo[] = [
  { bit: 0, value: 0x01, key: 'compressed', nameZh: '压缩包', nameEn: 'Compressed packet' },
  { bit: 1, value: 0x02, key: 'parity', nameZh: '奇偶帧标志', nameEn: 'Odd/even frame flag' },
  { bit: 2, value: 0x04, key: 'fifo', nameZh: 'FIFO 包模式', nameEn: 'FIFO packet mode' },
  { bit: 3, value: 0x08, key: 'splice_count', nameZh: '拼包计数', nameEn: 'Splice count' },
  { bit: 5, value: 0x20, key: 'splice_end', nameZh: '拼包结束', nameEn: 'Splice end' },
  { bit: 7, value: 0x80, key: 'new_data', nameZh: '新数据/算法结果', nameEn: 'New data / algorithm result' },
];

function enabledBits(bits: number, definitions: Gh3220BitInfo[]): Gh3220BitInfo[] {
  return definitions.filter((item) => (bits & item.value) !== 0);
}

export function parseDataType(bits: number): Gh3220BitInfo[] {
  return enabledBits(bits, GH3220_DATA_TYPE_BITS);
}

export function parsePackageType(bits: number): Gh3220BitInfo[] {
  return enabledBits(bits, GH3220_PACKAGE_TYPE_BITS);
}

export function formatBitNames(items: Gh3220BitInfo[], lang: Gh3220Lang): string {
  if (items.length === 0) return lang === 'zh' ? '无' : 'None';
  return items.map((item) => (lang === 'zh' ? item.nameZh : item.nameEn)).join(' / ');
}

/** 0x0B 帧的 8 字节包头：[function_id][data_type][channel_mask 4B 大端][package_type][data_len] */
export interface Gh3220FrameHeader {
  functionId: number;
  dataType: number;
  channelMask: number;
  channelCount: number;
  packageType: number;
  dataLen: number;
  dataTypeItems: Gh3220BitInfo[];
  packageTypeItems: Gh3220BitInfo[];
}

function hexToBytes(text: string): number[] | null {
  const cleaned = text.replace(/0x/gi, '').replace(/[^0-9a-f]/gi, '');
  if (!cleaned || cleaned.length % 2 !== 0) return null;
  const bytes: number[] = [];
  for (let i = 0; i < cleaned.length; i += 2) {
    const value = Number.parseInt(cleaned.slice(i, i + 2), 16);
    if (Number.isNaN(value)) return null;
    bytes.push(value);
  }
  return bytes;
}

function countBits(value: number): number {
  let count = 0;
  let remaining = value >>> 0;
  while (remaining !== 0) {
    remaining &= remaining - 1;
    count += 1;
  }
  return count;
}

/**
 * 解析 hex 字符串里的 0x0B 帧头。
 * 允许以 CMD 0x0B 开头（自动跳过该字节），至少需要 8 字节包头；
 * 解析失败返回 null，由 UI 提示。仅用于展示，不改变任何下发数据。
 */
export function parseFrameHeader(hex: string): Gh3220FrameHeader | null {
  const bytes = hexToBytes(hex);
  if (!bytes) return null;
  const offset = bytes[0] === 0x0b ? 1 : 0;
  if (bytes.length < offset + 8) return null;
  const [functionId, dataType] = bytes.slice(offset, offset + 2);
  const channelMask =
    ((bytes[offset + 2] << 24) |
      (bytes[offset + 3] << 16) |
      (bytes[offset + 4] << 8) |
      bytes[offset + 5]) >>>
    0;
  const packageType = bytes[offset + 6];
  const dataLen = bytes[offset + 7];
  return {
    functionId,
    dataType,
    channelMask,
    channelCount: countBits(channelMask),
    packageType,
    dataLen,
    dataTypeItems: parseDataType(dataType),
    packageTypeItems: parsePackageType(packageType),
  };
}

/** F5 语义文本 → 功能位 mask（扩展版：全表语义名、历史别名、十进制/十六进制数值） */
export function parseFunctionMaskText(input: string): number {
  const text = input.trim();
  if (!text) return 0;

  let mask = 0;
  for (const part of text.split(/[,;\s]+/)) {
    const name = part.trim().toLowerCase();
    if (!name) continue;
    const bit = GH3220_FUNCTION_BIT_ALIASES[name];
    if (bit !== undefined) {
      mask |= 1 << bit;
      continue;
    }
    // 向后兼容：非语义文本按数值 mask 解析（0x6 / 6）
    if (/^0x[0-9a-f]+$/i.test(name)) {
      mask |= Number.parseInt(name.slice(2), 16);
      continue;
    }
    if (/^\d+$/.test(name)) {
      mask |= Number.parseInt(name, 10);
      continue;
    }
    throw new Error(`未知功能: ${part.trim()}`);
  }
  return mask >>> 0;
}

/** 计划 F5 使用的命名（语义文本 → mask 的解析扩展），保留别名以便按图索骥 */
export const funcMaskToText = parseFunctionMaskText;

/** 展示层工具 ---------------------------------------------------------- */

export function getFunctionInfo(functionId: number): Gh3220FunctionInfo | undefined {
  return GH3220_FUNCTION_MAP[functionId];
}

export function getFunctionName(functionId: number, lang: Gh3220Lang): string {
  const info = GH3220_FUNCTION_MAP[functionId];
  if (!info) return lang === 'zh' ? `功能 func${functionId}` : `Function func${functionId}`;
  return `${lang === 'zh' ? info.nameZh : info.nameEn} ${info.abbr}`;
}

/** `心率 HR (4 CH)` / `Heart Rate HR (4 CH)`；未知 func 兜底 `功能 funcN` */
export function formatFunctionLabel(
  functionId: number,
  channelCount: number | null | undefined,
  lang: Gh3220Lang
): string {
  const channelText = channelCount === null || channelCount === undefined ? '' : ` (${channelCount} CH)`;
  return `${getFunctionName(functionId, lang)}${channelText}`;
}

/** WORK_MODE mask → 名称列表；未知位以 0x 原始值兜底 */
export function formatFunctionMaskNames(mask: number, lang: Gh3220Lang): string[] {
  const value = mask >>> 0;
  const names: string[] = [];
  let knownMask = 0;
  for (const info of GH3220_FUNCTION_LIST) {
    if ((value & info.mask) !== 0) {
      names.push(`${lang === 'zh' ? info.nameZh : info.nameEn} ${info.abbr}`);
      knownMask |= info.mask;
    }
  }
  const unknown = (value & ~knownMask) >>> 0;
  if (unknown !== 0) names.push(`0x${unknown.toString(16)}`);
  return names;
}

export function formatWorkModeLabel(mode: number, lang: Gh3220Lang): string {
  const info = GH3220_WORK_MODE_MAP[mode];
  return info
    ? `${mode} · ${lang === 'zh' ? info.nameZh : info.nameEn}`
    : `${mode} · ${lang === 'zh' ? '未知模式' : 'Unknown mode'}`;
}

/** 事件码 → 语义名；组合位逐项列出，未知位显示 0x 原始值 */
export function formatEventName(eventId: number, lang: Gh3220Lang): string {
  const value = eventId >>> 0;
  const names: string[] = [];
  let knownMask = 0;
  for (const info of GH3220_EVENT_LIST) {
    if ((value & info.bit) !== 0) {
      names.push(lang === 'zh' ? info.nameZh : info.nameEn);
      knownMask |= info.bit;
    }
  }
  const unknown = (value & ~knownMask) >>> 0;
  if (unknown !== 0) names.push(`0x${unknown.toString(16).padStart(4, '0')}`);
  if (names.length === 0) return `0x${value.toString(16).padStart(4, '0')}`;
  return names.join(' / ');
}

/** `/waveform` 缓冲标注：保持 value 为 `gh3220:funcN`，仅追加展示用语义名 */
export function formatBufferLabel(bufferId: string, lang: Gh3220Lang): string {
  const match = /^gh3220:func(\d+)$/.exec(bufferId);
  if (!match) return bufferId;
  return `${bufferId} · ${getFunctionName(Number(match[1]), lang)}`;
}

/* ==========================================================================
 * V3：语义值编解码与选项工厂
 *
 * 设计约定（对齐 GH3036 的 `bitMap.reduce` 范式）：
 * - UI 状态只存语义键（'hr' / 'spo2' / 'int' / 'wakeup'），不存数值文本；
 * - 数值编码（多选 → u32 / u16）集中在下面这些函数，组件不得自行 parse；
 * - 未知位保留 `0x…` 占位项，保证设备回读值不丢失。
 * ========================================================================== */

/** 下拉/多选选项；`value` 一律是字符串（多选为 string[]） */
export interface Gh3220Option {
  value: string;
  label: string;
  note?: string;
}

/** 把任意 UI 原始值收敛成单个字符串（多选时取首项，仅用于非多选字段） */
export function asText(raw: string | string[]): string {
  return Array.isArray(raw) ? (raw[0] ?? '') : raw;
}

/** 解析数值字面量：支持 `0x` 前缀、十进制、空值兜底 */
export function parseNumericText(text: string, fallback = 0): number {
  const value = text.trim();
  if (!value) return fallback;
  const parsed = /^0x[0-9a-f]+$/i.test(value)
    ? Number.parseInt(value.slice(2), 16)
    : Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

/**
 * 多选语义键 → u32 功能位掩码。
 * 兼容历史别名（bp/ir/sadt_g…）、`0x…` 占位项与十进制/十六进制字面量，
 * 与 `parseFunctionMaskText()` 共用同一张别名表，保证两种入口结果一致。
 */
export function encodeFunctionMask(keys: string[]): number {
  let mask = 0;
  for (const raw of keys) {
    const name = raw.trim().toLowerCase();
    if (!name) continue;
    const bit = GH3220_FUNCTION_BIT_ALIASES[name];
    if (bit !== undefined) {
      mask |= 1 << bit;
      continue;
    }
    if (/^0x[0-9a-f]+$/i.test(name)) {
      const value = Number.parseInt(name.slice(2), 16);
      if (!Number.isFinite(value)) throw new Error(`未知功能: ${raw}`);
      mask |= value;
      continue;
    }
    if (/^\d+$/.test(name)) {
      mask |= Number.parseInt(name, 10);
      continue;
    }
    throw new Error(`未知功能: ${raw}`);
  }
  return mask >>> 0;
}

/** 设备回读的 mask → 多选语义键；未定义的位生成 `0x…` 占位项 */
export function decodeFunctionMask(mask: number): string[] {
  const value = mask >>> 0;
  const keys: string[] = [];
  let knownMask = 0;
  for (const info of GH3220_FUNCTION_LIST) {
    if ((value & info.mask) !== 0) {
      keys.push(info.key);
      knownMask |= info.mask;
    }
  }
  const unknown = (value & ~knownMask) >>> 0;
  if (unknown !== 0) keys.push(`0x${unknown.toString(16)}`);
  return keys;
}

/** `0x0042 · 心率 HR / 血氧 SPO2`；未知位由 formatFunctionMaskNames 兜底 */
export function formatFunctionMaskPreview(mask: number, lang: Gh3220Lang): string {
  const hex = `0x${(mask >>> 0).toString(16).padStart(4, '0')}`;
  const names = formatFunctionMaskNames(mask, lang);
  return names.length > 0 ? `${hex} · ${names.join(' / ')}` : `${hex} · ${lang === 'zh' ? '未选择功能' : 'No functions'}`;
}

/** 15 项功能多选选项（value = 语义键） */
export function functionOptions(lang: Gh3220Lang): Gh3220Option[] {
  return GH3220_FUNCTION_LIST.map((info) => ({
    value: info.key,
    label: `${lang === 'zh' ? info.nameZh : info.nameEn} ${info.abbr}`,
  }));
}

/** 14 项事件位多选选项（value = 位掩码十进制字符串，便于直接 OR） */
export function eventMaskOptions(lang: Gh3220Lang): Gh3220Option[] {
  return GH3220_EVENT_LIST.map((info) => ({
    value: String(info.bit),
    label: `0x${info.bit.toString(16).padStart(4, '0')} · ${lang === 'zh' ? info.nameZh : info.nameEn}`,
    note: info.note,
  }));
}

/** 事件位多选键 → u16 掩码；未知位以 `0x…` / 十进制字符串形式保留 */
export function encodeEventMask(keys: string[]): number {
  let mask = 0;
  for (const raw of keys) {
    const name = raw.trim().toLowerCase();
    if (!name) continue;
    const known = GH3220_EVENT_LIST.find((info) => info.key === name || String(info.bit) === name);
    if (known) {
      mask |= known.bit;
      continue;
    }
    if (!/^(0x[0-9a-f]+|\d+)$/i.test(name)) throw new Error(`未知事件位: ${raw}`);
    mask |= parseNumericText(name);
  }
  const result = mask >>> 0;
  if (!Number.isFinite(result) || result > 0xffff) {
    throw new Error(`事件掩码超出 u16 范围: ${keys.join(',')}`);
  }
  return result;
}

/** 事件掩码 → 多选键（未知位生成 0x 占位项） */
export function decodeEventMask(mask: number): string[] {
  const value = mask >>> 0;
  const keys: string[] = [];
  let knownMask = 0;
  for (const info of GH3220_EVENT_LIST) {
    if ((value & info.bit) !== 0) {
      keys.push(String(info.bit));
      knownMask |= info.bit;
    }
  }
  const unknown = (value & ~knownMask) >>> 0;
  if (unknown !== 0) keys.push(`0x${unknown.toString(16)}`);
  return keys;
}

export function formatEventMaskPreview(mask: number, lang: Gh3220Lang): string {
  const hex = `0x${(mask >>> 0).toString(16).padStart(4, '0')}`;
  const names = GH3220_EVENT_LIST.filter((info) => (mask & info.bit) !== 0).map((info) =>
    lang === 'zh' ? info.nameZh : info.nameEn
  );
  const knownMask = GH3220_EVENT_LIST.reduce((acc, info) => acc | ((mask & info.bit) !== 0 ? info.bit : 0), 0);
  const unknown = (mask & ~knownMask) >>> 0;
  if (unknown !== 0) names.push(`0x${unknown.toString(16)}`);
  return names.length > 0 ? `${hex} · ${names.join(' / ')}` : `${hex} · ${lang === 'zh' ? '无事件' : 'No events'}`;
}

/** 后端 `manager.rs:507-521` 的 debugStatus.kind 命名值 */
export interface Gh3220NamedValue {
  key: string;
  value: number | string;
  nameZh: string;
  nameEn: string;
}

export const GH3220_DEBUG_STATUS_KINDS: Gh3220NamedValue[] = [
  { key: 'int', value: 0x00, nameZh: '中断状态 int', nameEn: 'Interrupt status (int)' },
  { key: 'rst', value: 0x01, nameZh: '复位状态 rst', nameEn: 'Reset status (rst)' },
  { key: 'spcs', value: 0x02, nameZh: 'SPCS 状态', nameEn: 'SPCS status' },
  { key: 'spdo', value: 0x03, nameZh: 'SPDO 状态', nameEn: 'SPDO status' },
  { key: 'conn', value: 0x04, nameZh: '连接状态 conn', nameEn: 'Connection status (conn)' },
  { key: 'status', value: 0x10, nameZh: '芯片总状态 status', nameEn: 'Chip status (status)' },
];

/** 后端 `manager.rs:487-497` 的芯片控制动作（key 即后端识别的字符串） */
export const GH3220_CHIP_ACTIONS: Array<{ key: string; nameZh: string; nameEn: string; code: number }> = [
  { key: 'wakeup', nameZh: '唤醒芯片', nameEn: 'Wake up', code: 0xc3 },
  { key: 'sleep', nameZh: '芯片睡眠', nameEn: 'Sleep', code: 0xc4 },
  { key: 'soft-reset', nameZh: '软复位', nameEn: 'Soft reset', code: 0xc2 },
  { key: 'hard-reset', nameZh: '硬复位', nameEn: 'Hard reset', code: 0x5a },
  { key: 'rx-reset', nameZh: '接收复位', nameEn: 'RX reset', code: 0x5b },
];

/** `libs/gh-uprotocol/src/lib.rs:42-52` 的 ver_type 全表（9 项） */
export const GH3220_VERSION_TYPES: Gh3220NamedValue[] = [
  { key: 'fw', value: 0x01, nameZh: '固件版本', nameEn: 'Firmware' },
  { key: 'demo', value: 0x0a, nameZh: 'Demo 版本', nameEn: 'Demo' },
  { key: 'virtual_reg', value: 0x0b, nameZh: '虚拟寄存器版本', nameEn: 'Virtual register' },
  { key: 'bootloader', value: 0x0c, nameZh: 'Bootloader 版本', nameEn: 'Bootloader' },
  { key: 'protocol', value: 0x0e, nameZh: '协议版本', nameEn: 'Protocol' },
  { key: 'func_support', value: 0x0f, nameZh: '功能支持表版本', nameEn: 'Function support' },
  { key: 'drv', value: 0x10, nameZh: '驱动版本', nameEn: 'Driver' },
  { key: 'chip', value: 0x11, nameZh: '芯片版本', nameEn: 'Chip' },
  { key: 'algo', value: 0x12, nameZh: '算法版本', nameEn: 'Algorithm' },
];

/** `libs/gh-uprotocol/src/lib.rs:18-38` 的 CMD 号（集中单一数据源） */
export const GH3220_COMMAND_CODES = {
  REG_RW: 0x03,
  PACKAGE_TEST: 0x05,
  NEW_STRUCT_RAWDATA: 0x0b,
  START_CTRL: 0x0c,
  WORK_MODE: 0x10,
  FIFO_THR_CONFIG: 0x12,
  REPORT_EVENT_SET: 0x13,
  ECG_CLASSIFICATION: 0x14,
  CHIP_EVENT_REPORT: 0x16,
  CHIP_CTRL: 0x17,
  GET_VER: 0x19,
  GET_CONN_STATUS: 0x1a,
  SWITCH_SLOT_FUNC: 0x1c,
  GET_OPENED_FUNC: 0x25,
  SET_ECG_CASCADE: 0x26,
  BP_DATA_TRANS: 0x2b,
  FUNCTION_INFO_UPDATE: 0x2c,
  GET_MAX_LEN: 0xa0,
  LOAD_REG_LIST: 0xa1,
  DEBUG_STATUS: 0xa2,
} as const;

/** 常用数值档位；`custom` 由 UI 展开输入框并以 `custom:<value>` 形式存放 */
export const GH3220_BOUNDED_PRESETS = {
  slot: ['0', '1', '2', '3', '4', '5', '6', '7'],
  slotEn: ['0', '1', '2', '4', '8'],
  extra: ['0', '1', '2'],
  fifoThr: ['0', '1', '2', '4', '8', '16', '32', '64', '128', '256', '512'],
  idleCloseSecs: ['0', '5', '10', '30', '60', '300', '3600'],
  count: ['1', '2', '4', '8', '16', '32', '64'],
  sampleRate: ['5', '25', '50', '100', '200'],
  displayDuration: ['1', '2', '5', '10', '20', '30', '60'],
} as const;

export type Gh3220PresetKey = keyof typeof GH3220_BOUNDED_PRESETS;

/** 常用帧模板（选模板自动填充 HEX 输入框） */
export const GH3220_HEX_TEMPLATES: Array<{ key: string; nameZh: string; nameEn: string; hex: string }> = [
  { key: 'raw', nameZh: '原始帧头示例', nameEn: 'Raw frame sample', hex: 'AA 11 19 01 01' },
  { key: 'header', nameZh: '0x0B 帧头（4 CH）', nameEn: '0x0B header (4 CH)', hex: '01 03 00 00 00 0F 80 10' },
  { key: 'echo', nameZh: '回显测试', nameEn: 'Echo test', hex: '0102' },
];

/** 与后端 `src-tauri/src/gh3220/types.rs` 一致的默认 GATT UUID */
export const GH3220_DEFAULT_UUIDS = {
  service: '0000190e-0000-1000-8000-00805f9b34fb',
  write: '00000004-0000-1000-8000-00805f9b34fb',
  notify: '00000003-0000-1000-8000-00805f9b34fb',
} as const;

/** 后端仅允许回环地址，预置即合法值 */
export const GH3220_TCP_ADDRESSES = ['127.0.0.1:9000', 'localhost:9000', '[::1]:9000'] as const;

export const GH3220_MTU_PRESETS = ['20', '23', '64', '128', '244', '517'] as const;

/** 所选功能中最大的通道数上限；无上限时默认 32（后端约束 1..32） */
export function maxChannelsForFunctions(keys: string[]): number {
  const limits = keys
    .map((raw) => GH3220_FUNCTION_BIT_ALIASES[raw.trim().toLowerCase()])
    .filter((bit): bit is number => bit !== undefined)
    .map((bit) => GH3220_FUNCTION_MAP[bit]?.maxChannels)
    .filter((limit): limit is number => typeof limit === 'number');
  return Math.min(32, Math.max(1, ...(limits.length > 0 ? limits : [32])));
}

/** 通道覆盖选项：`auto` + 1..N，N 随已选功能动态变化 */
export function channelOptions(keys: string[], lang: Gh3220Lang): Gh3220Option[] {
  const max = maxChannelsForFunctions(keys);
  return [
    { value: 'auto', label: lang === 'zh' ? '自动（按帧头）' : 'Auto (from frame header)' },
    ...Array.from({ length: max }, (_, i) => ({ value: String(i + 1), label: `${i + 1} CH` })),
    { value: 'custom', label: lang === 'zh' ? '自定义…' : 'Custom…' },
  ];
}

/** 生成带「自动 / 自定义…」的数值档位选项 */
export function boundedOptions(
  preset: Gh3220PresetKey,
  lang: Gh3220Lang,
  suffix = '',
  withAuto = false
): Gh3220Option[] {
  const values = GH3220_BOUNDED_PRESETS[preset] as readonly string[];
  const options: Gh3220Option[] = [];
  if (withAuto) options.push({ value: 'auto', label: lang === 'zh' ? '自动' : 'Auto' });
  for (const value of values) {
    options.push({ value, label: `${value}${suffix}` });
  }
  options.push({ value: 'custom', label: lang === 'zh' ? '自定义…' : 'Custom…' });
  return options;
}

/** 工作模式下拉（8 项，value 为数值字符串） */
export function workModeOptions(lang: Gh3220Lang): Gh3220Option[] {
  return GH3220_WORK_MODE_LIST.map((info) => ({
    value: String(info.mode),
    label: formatWorkModeLabel(info.mode, lang),
  }));
}

export function boolOptions(lang: Gh3220Lang): Gh3220Option[] {
  return [
    { value: 'true', label: lang === 'zh' ? '是' : 'Yes' },
    { value: 'false', label: lang === 'zh' ? '否' : 'No' },
  ];
}

export function chipActionOptions(lang: Gh3220Lang): Gh3220Option[] {
  return GH3220_CHIP_ACTIONS.map((item) => ({
    value: item.key,
    label: lang === 'zh' ? item.nameZh : item.nameEn,
  }));
}

export function debugKindOptions(lang: Gh3220Lang): Gh3220Option[] {
  return [
    ...GH3220_DEBUG_STATUS_KINDS.map((item) => ({
      value: item.key,
      label: `0x${Number(item.value).toString(16).padStart(2, '0')} · ${lang === 'zh' ? item.nameZh : item.nameEn}`,
    })),
    { value: 'custom', label: lang === 'zh' ? '自定义数值…' : 'Custom value…' },
  ];
}

export function versionTypeOptions(lang: Gh3220Lang): Gh3220Option[] {
  return GH3220_VERSION_TYPES.map((item) => ({
    value: `0x${Number(item.value).toString(16).padStart(2, '0')}`,
    label: `0x${Number(item.value).toString(16).padStart(2, '0')} · ${lang === 'zh' ? item.nameZh : item.nameEn}`,
  }));
}

/** `custom:<value>` 编码约定：把「自定义」选项的实际值读出 */
export const CUSTOM_PREFIX = 'custom:';
export function customValue(raw: string | string[], fallback = ''): string {
  const text = asText(raw);
  return text.startsWith(CUSTOM_PREFIX) ? text.slice(CUSTOM_PREFIX.length) : fallback;
}
export function isCustom(raw: string | string[]): boolean {
  const text = asText(raw);
  return text === 'custom' || text.startsWith(CUSTOM_PREFIX);
}
export function customOption(value: string | number): string {
  return `${CUSTOM_PREFIX}${value}`;
}
