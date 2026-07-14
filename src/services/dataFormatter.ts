import i18n from '../i18n';
import { formatErrorMessage } from '../utils/errorMessage';

export type DataFormat = 'hex' | 'text' | 'binary' | 'decimal' | 'json';

export interface FormatOptions {
  format: DataFormat;
  separator?: string;
  prefix?: string;
  suffix?: string;
  uppercase?: boolean;
  lineBreak?: number;
}

export interface ParsedData {
  original: number[];
  formatted: string;
  format: DataFormat;
  length: number;
  isValid: boolean;
  error?: string;
}

class DataFormatter {
  format(data: number[], options: FormatOptions): ParsedData {
    const { format, separator = ' ', prefix = '', suffix = '', uppercase = true, lineBreak = 0 } = options;

    try {
      let formatted: string;

      switch (format) {
        case 'hex':
          formatted = this.formatHex(data, separator, prefix, suffix, uppercase, lineBreak);
          break;
        case 'text':
          formatted = this.formatText(data);
          break;
        case 'binary':
          formatted = this.formatBinary(data, separator, lineBreak);
          break;
        case 'decimal':
          formatted = this.formatDecimal(data, separator, lineBreak);
          break;
        case 'json':
          formatted = this.formatJson(data);
          break;
        default:
          formatted = this.formatHex(data, separator, prefix, suffix, uppercase, lineBreak);
      }

      return {
        original: data,
        formatted,
        format,
        length: data.length,
        isValid: true,
      };
    } catch (err) {
      return {
        original: data,
        formatted: '',
        format,
        length: data.length,
        isValid: false,
        error: formatErrorMessage(err, i18n.t('common:message.formatFailed')),
      };
    }
  }

  private formatHex(
    data: number[],
    separator: string,
    prefix: string,
    suffix: string,
    uppercase: boolean,
    lineBreak: number
  ): string {
    const hexValues = data.map((b) => {
      const hex = b.toString(16).padStart(2, '0');
      return uppercase ? hex.toUpperCase() : hex;
    });

    let result = hexValues.map((h) => `${prefix}${h}${suffix}`).join(separator);

    if (lineBreak > 0) {
      const lines: string[] = [];
      for (let i = 0; i < hexValues.length; i += lineBreak) {
        lines.push(hexValues.slice(i, i + lineBreak).map((h) => `${prefix}${h}${suffix}`).join(separator));
      }
      result = lines.join('\n');
    }

    return result;
  }

  private formatText(data: number[]): string {
    try {
      return new TextDecoder().decode(new Uint8Array(data));
    } catch {
      return data.map((b) => (b >= 32 && b <= 126 ? String.fromCharCode(b) : '.')).join('');
    }
  }

  private formatBinary(data: number[], separator: string, lineBreak: number): string {
    const binaryValues = data.map((b) => b.toString(2).padStart(8, '0'));

    if (lineBreak > 0) {
      const lines: string[] = [];
      for (let i = 0; i < binaryValues.length; i += lineBreak) {
        lines.push(binaryValues.slice(i, i + lineBreak).join(separator));
      }
      return lines.join('\n');
    }

    return binaryValues.join(separator);
  }

  private formatDecimal(data: number[], separator: string, lineBreak: number): string {
    const decimalValues = data.map((b) => b.toString());

    if (lineBreak > 0) {
      const lines: string[] = [];
      for (let i = 0; i < decimalValues.length; i += lineBreak) {
        lines.push(decimalValues.slice(i, i + lineBreak).join(separator));
      }
      return lines.join('\n');
    }

    return decimalValues.join(separator);
  }

  private formatJson(data: number[]): string {
    try {
      const text = new TextDecoder().decode(new Uint8Array(data));
      const json = JSON.parse(text);
      return JSON.stringify(json, null, 2);
    } catch {
      return this.formatHex(data, ' ', '', '', true, 16);
    }
  }

  parse(input: string, format: DataFormat): number[] {
    switch (format) {
      case 'hex':
        return this.parseHex(input);
      case 'text':
        return Array.from(new TextEncoder().encode(input));
      case 'binary':
        return this.parseBinary(input);
      case 'decimal':
        return this.parseDecimal(input);
      case 'json':
        return Array.from(new TextEncoder().encode(input));
      default:
        return this.parseHex(input);
    }
  }

  private parseHex(input: string): number[] {
    const cleaned = input.replace(/[^0-9A-Fa-f]/g, '');
    const result: number[] = [];

    for (let i = 0; i < cleaned.length; i += 2) {
      const hex = cleaned.substr(i, 2);
      if (hex.length === 2) {
        result.push(parseInt(hex, 16));
      }
    }

    return result;
  }

  private parseBinary(input: string): number[] {
    const cleaned = input.replace(/[^01]/g, '');
    const result: number[] = [];

    for (let i = 0; i < cleaned.length; i += 8) {
      const binary = cleaned.substr(i, 8);
      if (binary.length === 8) {
        result.push(parseInt(binary, 2));
      }
    }

    return result;
  }

  private parseDecimal(input: string): number[] {
    const numbers = input.match(/\d+/g) || [];
    return numbers.map((n) => parseInt(n, 10) & 0xff);
  }

  calculateChecksum(data: number[], algorithm: 'sum' | 'xor' | 'crc8' | 'crc16' = 'sum'): number {
    switch (algorithm) {
      case 'sum':
        return data.reduce((acc, b) => (acc + b) & 0xff, 0);
      case 'xor':
        return data.reduce((acc, b) => acc ^ b, 0);
      case 'crc8':
        return this.crc8(data);
      case 'crc16':
        return this.crc16(data);
      default:
        return 0;
    }
  }

  private crc8(data: number[]): number {
    let crc = 0;
    for (const byte of data) {
      crc ^= byte;
      for (let i = 0; i < 8; i++) {
        if (crc & 0x80) {
          crc = ((crc << 1) ^ 0x07) & 0xff;
        } else {
          crc = (crc << 1) & 0xff;
        }
      }
    }
    return crc;
  }

  private crc16(data: number[]): number {
    let crc = 0xffff;
    for (const byte of data) {
      crc ^= byte;
      for (let i = 0; i < 8; i++) {
        if (crc & 0x0001) {
          crc = ((crc >> 1) ^ 0xa001) & 0xffff;
        } else {
          crc = (crc >> 1) & 0xffff;
        }
      }
    }
    return crc;
  }

  formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  }

  formatTimestamp(timestamp: number, format: 'full' | 'time' | 'date' = 'full'): string {
    const date = new Date(timestamp);

    switch (format) {
      case 'time':
        const ms = String(date.getMilliseconds()).padStart(3, '0');
        return date.toLocaleTimeString('zh-CN', {
          hour: '2-digit',
          minute: '2-digit',
          second: '2-digit',
        }) + '.' + ms;
      case 'date':
        return date.toLocaleDateString('zh-CN');
      case 'full':
      default:
        return date.toLocaleString('zh-CN', {
          year: 'numeric',
          month: '2-digit',
          day: '2-digit',
          hour: '2-digit',
          minute: '2-digit',
          second: '2-digit',
        });
    }
  }
}

export const dataFormatter = new DataFormatter();
export default dataFormatter;
