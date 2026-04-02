export interface SerialPortInfo {
  portName: string;
  manufacturer?: string;
  product?: string;
  serialNumber?: string;
  vid?: number;
  pid?: number;
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

export type BaudRate = 300 | 1200 | 2400 | 4800 | 9600 | 14400 | 19200 | 38400 | 57600 | 115200 | 230400 | 460800 | 921600;

export const DEFAULT_BAUD_RATES: BaudRate[] = [
  300, 1200, 2400, 4800, 9600, 14400, 19200, 38400, 57600, 115200, 230400, 460800, 921600
];

export const DEFAULT_SERIAL_CONFIG: SerialConfig = {
  baudRate: 115200,
  dataBits: 8,
  stopBits: 1,
  parity: 'none',
  flowControl: 'none',
};
