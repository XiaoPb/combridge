export type DataFormat = 'hex' | 'text' | 'decimal' | 'binary';

export type Endianness = 'little' | 'big';

export interface ProtocolConfig {
  format: DataFormat;
  endianness: Endianness;
  encoding: string;
  lineEnding: 'cr' | 'lf' | 'crlf' | 'none';
  showTimestamp: boolean;
  showDirection: boolean;
}

export interface DataPacket {
  id: string;
  timestamp: number;
  direction: 'send' | 'receive';
  data: number[];
  format: DataFormat;
  displayText: string;
  size: number;
}

export interface ProtocolParser {
  name: string;
  pattern: RegExp | string;
  description?: string;
}

export interface ChecksumConfig {
  algorithm: 'none' | 'xor' | 'sum' | 'crc8' | 'crc16' | 'crc32';
  initialValue?: number;
  polynomial?: number;
}

export interface FrameConfig {
  header: number[];
  footer: number[];
  maxLength: number;
  checksum: ChecksumConfig;
}

export const DEFAULT_PROTOCOL_CONFIG: ProtocolConfig = {
  format: 'hex',
  endianness: 'little',
  encoding: 'utf-8',
  lineEnding: 'crlf',
  showTimestamp: true,
  showDirection: true,
};

export const HEX_CHARS = '0123456789ABCDEFabcdef';
export const HEX_REGEX = /^[0-9A-Fa-f\s]*$/;
