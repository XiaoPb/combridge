export type ChannelType = 'serial' | 'ble';

export interface ChannelSerialConfig {
  baudRate: number;
  dataBits: number;
  parity: string;
  stopBits: number;
  flowControl: string;
}

export interface BleCharacteristicConfig {
  deviceAddress: string;
  serviceUuid: string;
  characteristicUuid: string;
  properties: string[];
}

export type ChannelConfig = 
  | { type: 'serial'; baudRate: number; dataBits: number; parity: string; stopBits: number; flowControl: string }
  | { type: 'bleCharacteristic'; deviceAddress: string; serviceUuid: string; characteristicUuid: string; properties: string[] };

export interface BufferEntry {
  timestamp: number;
  data: number[];
  direction: string;
}

export interface ChannelBuffer {
  entries: BufferEntry[];
  totalBytes: number;
}

export interface DeviceChannel {
  id: string;
  name: string;
  type: ChannelType;
  connected: boolean;
  txBuffer: ChannelBuffer;
  rxBuffer: ChannelBuffer;
  config?: ChannelConfig;
  createdAt: number;
  bytesSent: number;
  bytesReceived: number;
}

export interface TabState {
  key: string;
  channelId: string;
  label: string;
  isActive: boolean;
}

export interface AppWindowState {
  tabs: TabState[];
  activeTabKey: string | null;
  sidebarWidth?: number;
  panelHeight?: number;
}

export interface ChannelAppSettings {
  theme: string;
  language: string;
  autoReconnect: boolean;
  logLevel: string;
  maxBufferSize: number;
}

export interface AppState {
  channels: DeviceChannel[];
  activeChannelId: string | null;
  settings: ChannelAppSettings;
  windowState: AppWindowState;
}

export type Action =
  | { type: 'CHANNEL_ADD'; name: string; channelType: string; config?: Record<string, unknown> }
  | { type: 'CHANNEL_REMOVE'; id: string }
  | { type: 'CHANNEL_CONNECT'; id: string; config?: Record<string, unknown> }
  | { type: 'CHANNEL_DISCONNECT'; id: string }
  | { type: 'DATA_SEND'; channelId: string; data: number[] }
  | { type: 'CHANNEL_SWITCH'; channelId: string }
  | { type: 'BUFFER_CLEAR'; channelId: string; direction: string }
  | { type: 'TAB_ADD'; channelId: string; label: string }
  | { type: 'TAB_REMOVE'; tabKey: string }
  | { type: 'TAB_SWITCH'; tabKey: string }
  | { type: 'SETTINGS_UPDATE'; settings: Record<string, unknown> }
  | { type: 'STATE_RESTORE'; windowState: Record<string, unknown> };

export interface ActionResult {
  success: boolean;
  message?: string;
  data?: Record<string, unknown>;
}
