export type ChannelDirection = 'read' | 'write' | 'notify';

export interface BufferEntry {
  timestamp: number;
  data: number[];
}

export interface ChannelBuffer {
  entries: BufferEntry[];
  totalBytes: number;
}

export interface Channel {
  id: string;
  direction: ChannelDirection;
  buffer: ChannelBuffer;
  subscribed: boolean;
}

export type DataBits = 'five' | 'six' | 'seven' | 'eight';
export type ParityType = 'none' | 'odd' | 'even';
export type StopBitsType = 'one' | 'two';

export interface ConnectionParams {
  interval: number;
  latency: number;
  timeout: number;
}

export interface SerialDevice {
  id: string;
  name: string;
  connected: boolean;
  connectable: boolean;
  baudRate: number;
  dataBits: DataBits;
  parity: ParityType;
  stopBits: StopBitsType;
  channels: Record<string, Channel>;
}

export interface BleDevice {
  id: string;
  name: string;
  mac: string;
  connected: boolean;
  connectable: boolean;
  mtu: number;
  connectionParams: ConnectionParams;
  channels: Record<string, Channel>;
}

export type Device = 
  | { type: 'serial' } & SerialDevice 
  | { type: 'ble' } & BleDevice;

export interface TabState {
  key: string;
  deviceId: string;
  channelId?: string;
  label: string;
  isActive: boolean;
}

export interface ChannelWindowState {
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
  devices: Record<string, Device>;
  activeDeviceId: string | null;
  settings: ChannelAppSettings;
  windowState: ChannelWindowState;
}

export type Action =
  | { type: 'DEVICE_ADD_SERIAL'; id: string; name: string; baudRate: number }
  | { type: 'DEVICE_ADD_BLE'; id: string; name: string; mac: string }
  | { type: 'DEVICE_REMOVE'; deviceId: string }
  | { type: 'DEVICE_CONNECT'; deviceId: string }
  | { type: 'DEVICE_DISCONNECT'; deviceId: string }
  | { type: 'DEVICE_UPDATE_CONFIG'; deviceId: string; config: Record<string, unknown> }
  | { type: 'CHANNEL_ADD'; deviceId: string; channelId: string; direction: string }
  | { type: 'CHANNEL_SUBSCRIBE'; deviceId: string; channelId: string; subscribe: boolean }
  | { type: 'DATA_SEND'; deviceId: string; channelId: string; data: number[] }
  | { type: 'DATA_RECEIVE'; deviceId: string; channelId: string; data: number[] }
  | { type: 'BUFFER_CLEAR'; deviceId: string; channelId: string }
  | { type: 'DEVICE_SWITCH'; deviceId: string }
  | { type: 'TAB_ADD'; deviceId: string; channelId?: string; label: string }
  | { type: 'TAB_REMOVE'; tabKey: string }
  | { type: 'TAB_SWITCH'; tabKey: string }
  | { type: 'SETTINGS_UPDATE'; settings: Record<string, unknown> }
  | { type: 'STATE_RESTORE'; windowState: Record<string, unknown> };

export interface ActionResult {
  success: boolean;
  message?: string;
  data?: Record<string, unknown>;
}
