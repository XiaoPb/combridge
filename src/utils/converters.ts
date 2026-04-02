export function bytesToHex(bytes: number[], separator: string = ' '): string {
  return bytes.map(b => b.toString(16).padStart(2, '0').toUpperCase()).join(separator);
}

export function hexToBytes(hex: string): number[] {
  const cleanHex = hex.replace(/\s+/g, '');
  const bytes: number[] = [];
  for (let i = 0; i < cleanHex.length; i += 2) {
    bytes.push(parseInt(cleanHex.substr(i, 2), 16));
  }
  return bytes;
}

export function bytesToText(bytes: number[], encoding: string = 'utf-8'): string {
  const decoder = new TextDecoder(encoding);
  return decoder.decode(new Uint8Array(bytes));
}

export function textToBytes(text: string): number[] {
  const encoder = new TextEncoder();
  return Array.from(encoder.encode(text));
}

export function bytesToDecimal(bytes: number[], separator: string = ' '): string {
  return bytes.map(b => b.toString(10).padStart(3, '0')).join(separator);
}

export function decimalToBytes(decimal: string): number[] {
  const parts = decimal.trim().split(/\s+/);
  return parts.map(p => parseInt(p, 10));
}

export function bytesToBinary(bytes: number[], separator: string = ' '): string {
  return bytes.map(b => b.toString(2).padStart(8, '0')).join(separator);
}

export function binaryToBytes(binary: string): number[] {
  const cleanBinary = binary.replace(/\s+/g, '');
  const bytes: number[] = [];
  for (let i = 0; i < cleanBinary.length; i += 8) {
    bytes.push(parseInt(cleanBinary.substr(i, 8), 2));
  }
  return bytes;
}

export function formatBytes(bytes: number[]): string {
  if (bytes.length === 0) return '';
  return bytesToHex(bytes);
}

export function parseInput(input: string, format: 'hex' | 'text' | 'decimal' | 'binary'): number[] {
  switch (format) {
    case 'hex':
      return hexToBytes(input);
    case 'text':
      return textToBytes(input);
    case 'decimal':
      return decimalToBytes(input);
    case 'binary':
      return binaryToBytes(input);
    default:
      return [];
  }
}

export function formatOutput(bytes: number[], format: 'hex' | 'text' | 'decimal' | 'binary'): string {
  switch (format) {
    case 'hex':
      return bytesToHex(bytes);
    case 'text':
      return bytesToText(bytes);
    case 'decimal':
      return bytesToDecimal(bytes);
    case 'binary':
      return bytesToBinary(bytes);
    default:
      return '';
  }
}
