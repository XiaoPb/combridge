export interface SerialPortInfo {
  name: string;
  port_type: string;
  manufacturer?: string;
  product?: string;
  serial_number?: string;
}

export interface SerialConfig {
  baudRate: number;
  dataBits: 5 | 6 | 7 | 8;
  stopBits: 1 | 2;
  parity: 'none' | 'odd' | 'even';
  flowControl: 'none' | 'hardware' | 'software';
}

export interface SerialConnection {
  portName: string;
  config: SerialConfig;
  isConnected: boolean;
  openedAt?: number;
}

export type BaudRate = number;

export const DEFAULT_BAUD_RATES: number[] = [
  300, 1200, 2400, 4800, 9600, 14400, 19200, 38400, 57600, 115200, 230400, 460800, 921600
];

export const BAUD_RATE_MIN = 300;
export const BAUD_RATE_MAX = 2000000; // 2M

export const DEFAULT_SERIAL_CONFIG: SerialConfig = {
  baudRate: 115200,
  dataBits: 8,
  stopBits: 1,
  parity: 'none',
  flowControl: 'none',
};
