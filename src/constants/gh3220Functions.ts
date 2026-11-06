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
