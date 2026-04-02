export const APP_NAME = 'ComBridge';

export const SERIAL_DATA_BITS = [5, 6, 7, 8] as const;
export const SERIAL_STOP_BITS = [1, 2] as const;
export const SERIAL_PARITY = ['none', 'odd', 'even'] as const;
export const SERIAL_FLOW_CONTROL = ['none', 'hardware', 'software'] as const;

export const DEFAULT_BAUD_RATE = 115200;
export const DEFAULT_DATA_BITS = 8;
export const DEFAULT_STOP_BITS = 1;
export const DEFAULT_PARITY = 'none';
export const DEFAULT_FLOW_CONTROL = 'none';

export const MAX_DATA_SIZE = 1024 * 1024;
export const MAX_LOG_ENTRIES = 10000;
export const MAX_PACKET_SIZE = 1024;

export const RECONNECT_INTERVAL = 3000;
export const SCAN_TIMEOUT = 10000;
export const CONNECTION_TIMEOUT = 30000;

export const LINE_ENDINGS = {
  CR: '\r',
  LF: '\n',
  CRLF: '\r\n',
  NONE: '',
} as const;

export const ENCODINGS = [
  'utf-8',
  'ascii',
  'gbk',
  'gb2312',
  'big5',
  'iso-8859-1',
  'utf-16',
  'utf-16le',
  'utf-16be',
] as const;

export const DATA_FORMATS = ['hex', 'text', 'decimal', 'binary'] as const;

export const THEMES = ['light', 'dark', 'system'] as const;

export const LANGUAGES = [
  { code: 'zh-CN', name: '简体中文' },
  { code: 'en-US', name: 'English' },
] as const;
