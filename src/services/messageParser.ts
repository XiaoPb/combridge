export interface ParsedMessage {
  type: MessageType;
  id: string;
  timestamp: number;
  payload: unknown;
  raw: number[];
}

export type MessageType =
  | 'data'
  | 'command'
  | 'response'
  | 'event'
  | 'error'
  | 'heartbeat'
  | 'unknown';

export interface MessageSchema {
  type: MessageType;
  fields: FieldDefinition[];
}

export interface FieldDefinition {
  name: string;
  type: 'uint8' | 'uint16' | 'uint32' | 'int8' | 'int16' | 'int32' | 'float' | 'string' | 'bytes';
  offset: number;
  length?: number;
}

const MAGIC_BYTE = 0xcb;
const HEADER_SIZE = 8;

class MessageParser {
  private schemas: Map<string, MessageSchema> = new Map();

  registerSchema(name: string, schema: MessageSchema): void {
    this.schemas.set(name, schema);
  }

  unregisterSchema(name: string): void {
    this.schemas.delete(name);
  }

  parse(data: number[], schemaName?: string): ParsedMessage {
    if (data.length < HEADER_SIZE) {
      return this.createUnknownMessage(data);
    }

    if (data[0] === MAGIC_BYTE) {
      return this.parseMsgPack(data);
    }

    if (schemaName && this.schemas.has(schemaName)) {
      return this.parseWithSchema(data, schemaName);
    }

    return this.parseAuto(data);
  }

  private parseMsgPack(data: number[]): ParsedMessage {
    try {
      let offset = 2;

      const idLen = (data[offset] << 8) | data[offset + 1];
      offset += 2;
      const id = String.fromCharCode(...data.slice(offset, offset + idLen));
      offset += idLen;

      const timestamp =
        (BigInt(data[offset]) << 56n) |
        (BigInt(data[offset + 1]) << 48n) |
        (BigInt(data[offset + 2]) << 40n) |
        (BigInt(data[offset + 3]) << 32n) |
        (BigInt(data[offset + 4]) << 24n) |
        (BigInt(data[offset + 5]) << 16n) |
        (BigInt(data[offset + 6]) << 8n) |
        BigInt(data[offset + 7]);
      offset += 8;

      const typeLen = (data[offset] << 8) | data[offset + 1];
      offset += 2;
      const typeStr = String.fromCharCode(...data.slice(offset, offset + typeLen)).toLowerCase();
      offset += typeLen;

      const metaLen = (data[offset] << 8) | data[offset + 1];
      offset += 2 + metaLen;

      const payloadLen =
        (data[offset] << 24) |
        (data[offset + 1] << 16) |
        (data[offset + 2] << 8) |
        data[offset + 3];
      offset += 4;

      const payload = data.slice(offset, offset + payloadLen);

      return {
        type: typeStr as MessageType,
        id,
        timestamp: Number(timestamp),
        payload: this.tryParseJson(payload),
        raw: data,
      };
    } catch (err) {
      console.error('Failed to parse MsgPack:', err);
      return this.createUnknownMessage(data);
    }
  }

  private parseWithSchema(data: number[], schemaName: string): ParsedMessage {
    const schema = this.schemas.get(schemaName);
    if (!schema) {
      return this.createUnknownMessage(data);
    }

    const payload: Record<string, unknown> = {};

    for (const field of schema.fields) {
      try {
        payload[field.name] = this.extractField(data, field);
      } catch (err) {
        console.error(`Failed to extract field ${field.name}:`, err);
        payload[field.name] = null;
      }
    }

    return {
      type: schema.type,
      id: this.generateId(),
      timestamp: Date.now(),
      payload,
      raw: data,
    };
  }

  private parseAuto(data: number[]): ParsedMessage {
    if (this.looksLikeJson(data)) {
      return {
        type: 'data',
        id: this.generateId(),
        timestamp: Date.now(),
        payload: this.tryParseJson(data),
        raw: data,
      };
    }

    if (this.looksLikeAscii(data)) {
      return {
        type: 'data',
        id: this.generateId(),
        timestamp: Date.now(),
        payload: {
          text: this.toAscii(data),
          hex: this.toHex(data),
        },
        raw: data,
      };
    }

    return {
      type: 'data',
      id: this.generateId(),
      timestamp: Date.now(),
      payload: {
        hex: this.toHex(data),
        length: data.length,
      },
      raw: data,
    };
  }

  private createUnknownMessage(data: number[]): ParsedMessage {
    return {
      type: 'unknown',
      id: this.generateId(),
      timestamp: Date.now(),
      payload: {
        hex: this.toHex(data),
        length: data.length,
      },
      raw: data,
    };
  }

  private extractField(data: number[], field: FieldDefinition): unknown {
    const offset = field.offset;

    switch (field.type) {
      case 'uint8':
        return data[offset];
      case 'uint16':
        return (data[offset] << 8) | data[offset + 1];
      case 'uint32':
        return (data[offset] << 24) | (data[offset + 1] << 16) | (data[offset + 2] << 8) | data[offset + 3];
      case 'int8':
        const val8 = data[offset];
        return val8 > 127 ? val8 - 256 : val8;
      case 'int16':
        const val16 = (data[offset] << 8) | data[offset + 1];
        return val16 > 32767 ? val16 - 65536 : val16;
      case 'int32':
        const val32 = (data[offset] << 24) | (data[offset + 1] << 16) | (data[offset + 2] << 8) | data[offset + 3];
        return val32 > 2147483647 ? val32 - 4294967296 : val32;
      case 'float':
        const buf = new ArrayBuffer(4);
        new Uint8Array(buf).set(data.slice(offset, offset + 4));
        return new Float32Array(buf)[0];
      case 'string':
        const len = field.length || data.length - offset;
        return String.fromCharCode(...data.slice(offset, offset + len));
      case 'bytes':
        const byteLen = field.length || data.length - offset;
        return data.slice(offset, offset + byteLen);
      default:
        return null;
    }
  }

  private looksLikeJson(data: number[]): boolean {
    if (data.length < 2) return false;
    const first = data[0];
    const last = data[data.length - 1];
    return (first === 0x7b && last === 0x7d) || (first === 0x5b && last === 0x5d);
  }

  private looksLikeAscii(data: number[]): boolean {
    if (data.length === 0) return false;
    let printable = 0;
    for (const byte of data) {
      if ((byte >= 32 && byte <= 126) || byte === 10 || byte === 13 || byte === 9) {
        printable++;
      }
    }
    return printable / data.length > 0.8;
  }

  private tryParseJson(data: number[]): unknown {
    try {
      const str = new TextDecoder().decode(new Uint8Array(data));
      return JSON.parse(str);
    } catch {
      return null;
    }
  }

  private toHex(data: number[]): string {
    return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
  }

  private toAscii(data: number[]): string {
    return String.fromCharCode(...data.map((b) => (b >= 32 && b <= 126 ? b : 46)));
  }

  private generateId(): string {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }
}

export const messageParser = new MessageParser();
export default messageParser;
