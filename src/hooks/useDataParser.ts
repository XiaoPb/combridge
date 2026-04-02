import { useMemo, useCallback } from 'react';
import messageParser, { type ParsedMessage, type MessageSchema } from '../services/messageParser';
import dataFormatter, { type DataFormat, type FormatOptions, type ParsedData } from '../services/dataFormatter';

interface UseDataParserReturn {
  parse: (data: number[], schemaName?: string) => ParsedMessage;
  format: (data: number[], options: FormatOptions) => ParsedData;
  parseInput: (input: string, format: DataFormat) => number[];
  registerSchema: (name: string, schema: MessageSchema) => void;
  unregisterSchema: (name: string) => void;
  calculateChecksum: (data: number[], algorithm?: 'sum' | 'xor' | 'crc8' | 'crc16') => number;
  formatBytes: (bytes: number) => string;
  formatTimestamp: (timestamp: number, format?: 'full' | 'time' | 'date') => string;
  toHex: (data: number[]) => string;
  toText: (data: number[]) => string;
  fromHex: (hex: string) => number[];
  fromText: (text: string) => number[];
}

export const useDataParser = (): UseDataParserReturn => {
  const parse = useCallback((data: number[], schemaName?: string) => {
    return messageParser.parse(data, schemaName);
  }, []);

  const format = useCallback((data: number[], options: FormatOptions) => {
    return dataFormatter.format(data, options);
  }, []);

  const parseInput = useCallback((input: string, format: DataFormat) => {
    return dataFormatter.parse(input, format);
  }, []);

  const registerSchema = useCallback((name: string, schema: MessageSchema) => {
    messageParser.registerSchema(name, schema);
  }, []);

  const unregisterSchema = useCallback((name: string) => {
    messageParser.unregisterSchema(name);
  }, []);

  const calculateChecksum = useCallback((data: number[], algorithm: 'sum' | 'xor' | 'crc8' | 'crc16' = 'sum') => {
    return dataFormatter.calculateChecksum(data, algorithm);
  }, []);

  const formatBytes = useCallback((bytes: number) => {
    return dataFormatter.formatBytes(bytes);
  }, []);

  const formatTimestamp = useCallback((timestamp: number, format: 'full' | 'time' | 'date' = 'full') => {
    return dataFormatter.formatTimestamp(timestamp, format);
  }, []);

  const toHex = useCallback((data: number[]) => {
    return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
  }, []);

  const toText = useCallback((data: number[]) => {
    try {
      return new TextDecoder().decode(new Uint8Array(data));
    } catch {
      return '';
    }
  }, []);

  const fromHex = useCallback((hex: string) => {
    const cleaned = hex.replace(/[^0-9A-Fa-f]/g, '');
    const result: number[] = [];
    for (let i = 0; i < cleaned.length; i += 2) {
      result.push(parseInt(cleaned.substr(i, 2), 16));
    }
    return result;
  }, []);

  const fromText = useCallback((text: string) => {
    return Array.from(new TextEncoder().encode(text));
  }, []);

  return useMemo(() => ({
    parse,
    format,
    parseInput,
    registerSchema,
    unregisterSchema,
    calculateChecksum,
    formatBytes,
    formatTimestamp,
    toHex,
    toText,
    fromHex,
    fromText,
  }), [
    parse,
    format,
    parseInput,
    registerSchema,
    unregisterSchema,
    calculateChecksum,
    formatBytes,
    formatTimestamp,
    toHex,
    toText,
    fromHex,
    fromText,
  ]);
};

export default useDataParser;
