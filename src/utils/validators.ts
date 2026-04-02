const SERIAL_PORT_REGEX = /^(COM\d+|\/dev\/(tty|cu)\S+)$/i;
const MAC_ADDRESS_REGEX = /^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$/;
const BLE_ADDRESS_REGEX = /^([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}$/;
const UUID_REGEX = /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;

export function isValidSerialPort(portName: string): boolean {
  return SERIAL_PORT_REGEX.test(portName);
}

export function isValidMacAddress(mac: string): boolean {
  return MAC_ADDRESS_REGEX.test(mac);
}

export function isValidBleAddress(address: string): boolean {
  return BLE_ADDRESS_REGEX.test(address);
}

export function isValidUuid(uuid: string): boolean {
  return UUID_REGEX.test(uuid);
}

export function isValidBaudRate(baudRate: number): boolean {
  const validBaudRates = [300, 1200, 2400, 4800, 9600, 14400, 19200, 38400, 57600, 115200, 230400, 460800, 921600];
  return validBaudRates.includes(baudRate);
}

export function isValidHex(input: string): boolean {
  const cleanInput = input.replace(/\s+/g, '');
  return /^[0-9A-Fa-f]*$/.test(cleanInput) && cleanInput.length % 2 === 0;
}

export function isValidDecimal(input: string): boolean {
  const parts = input.trim().split(/\s+/);
  return parts.every(p => /^\d{1,3}$/.test(p) && parseInt(p, 10) <= 255);
}

export function isValidBinary(input: string): boolean {
  const cleanInput = input.replace(/\s+/g, '');
  return /^[01]*$/.test(cleanInput) && cleanInput.length % 8 === 0;
}

export function validateInput(input: string, format: 'hex' | 'text' | 'decimal' | 'binary'): { valid: boolean; error?: string } {
  if (!input || input.trim().length === 0) {
    return { valid: false, error: '输入不能为空' };
  }

  switch (format) {
    case 'hex':
      if (!isValidHex(input)) {
        return { valid: false, error: '无效的十六进制格式' };
      }
      break;
    case 'decimal':
      if (!isValidDecimal(input)) {
        return { valid: false, error: '无效的十进制格式（每个字节应在0-255之间）' };
      }
      break;
    case 'binary':
      if (!isValidBinary(input)) {
        return { valid: false, error: '无效的二进制格式' };
      }
      break;
    case 'text':
      break;
  }

  return { valid: true };
}
