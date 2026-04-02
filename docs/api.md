# ComBridge API 文档

本文档整理了 ComBridge 项目前后端交互的所有 Tauri 命令格式，采用蛇形命名（snake_case）规范。

## 目录

- [串口模块 (Serial)](#串口模块-serial)
- [BLE 模块 (BLE)](#ble-模块-ble)
- [协议模块 (Protocol)](#协议模块-protocol)
- [WebSocket 模块 (WebSocket)](#websocket-模块-websocket)
- [系统模块 (System)](#系统模块-system)

---

## 串口模块 (Serial)

### scan_serial_ports

扫描可用串口列表。

**后端命令**: `scan_serial_ports`

**参数**: 无

**返回**: `SerialPortInfo[]`

```typescript
// 前端调用
await serialApi.scanPorts();

// 返回类型
interface SerialPortInfo {
  name: string;           // 端口名称，如 "COM1"
  port_type: string;      // 端口类型
  manufacturer?: string;  // 制造商
  product?: string;       // 产品名称
  serial_number?: string; // 序列号
}
```

---

### open_serial_port

打开指定串口。

**后端命令**: `open_serial_port`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| config | object | 是 | 串口配置对象 |
| config.port_name | string | 是 | 端口名称 |
| config.baud_rate | string | 是 | 波特率（字符串），如 "9600", "115200" |
| config.data_bits | number | 否 | 数据位（5, 6, 7, 8），默认 8 |
| config.parity | string | 否 | 校验位（"none", "odd", "even"），默认 "none" |
| config.stop_bits | number | 否 | 停止位（1, 2），默认 1 |
| config.flow_control | string | 否 | 流控制（"none", "hardware", "software"），默认 "none" |
| config.timeout_ms | number | 否 | 超时时间（毫秒），默认 1000 |

**返回**: `void`

```typescript
// 前端调用
await serialApi.open('COM1', {
  baudRate: 115200,
  dataBits: 8,
  parity: 'none',
  stopBits: 1,
  flowControl: 'none',
});
```

---

### close_serial_port

关闭指定串口。

**后端命令**: `close_serial_port`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| port_name | string | 是 | 端口名称 |

**返回**: `void`

```typescript
// 前端调用
await serialApi.close('COM1');
```

---

### send_serial_data

向串口发送数据。

**后端命令**: `send_serial_data`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| portName | string | 是 | 端口名称 |
| data | number[] | 是 | 要发送的字节数据 |

**返回**: `number` (发送的字节数)

```typescript
// 前端调用
await serialApi.sendData('COM1', [0x01, 0x02, 0x03]);
```

---

### get_open_ports

获取已打开的端口列表。

**后端命令**: `get_open_ports`

**参数**: 无

**返回**: `string[]`

```typescript
// 前端调用
const ports = await serialApi.getOpenPorts();
```

---

### is_port_open

检查端口是否已打开。

**后端命令**: `is_port_open`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| port_name | string | 是 | 端口名称 |

**返回**: `boolean`

```typescript
// 前端调用
const isOpen = await serialApi.isConnected('COM1');
```

---

### 串口事件 (Serial Events)

#### serial-data

串口接收数据事件。

```typescript
// 事件类型
interface SerialDataEvent {
  port_name: string;     // 端口名称
  data: number[];       // 接收到的数据
}

// 前端监听
import { onSerialData } from '../api/events';
const unlisten = await onSerialData((event) => {
  console.log('收到数据:', event.data);
});
```

---

## BLE 模块 (BLE)

### configure_ble

配置 BLE 工作模式。

**后端命令**: `configure_ble`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| config | object | 是 | BLE 配置对象 |
| config.mode | string | 是 | 模式："native" 或 "at" |
| config.port_name | string | AT 模式必填 | AT 模式下的串口名称 |
| config.baud_rate | number | 否 | AT 模式波特率，默认 115200 |
| config.timeout_ms | number | 否 | AT 指令超时，默认 1000 |

**返回**: `void`

```typescript
// 前端调用
await bleApi.configure({ mode: 'native' });
// 或 AT 模式
await bleApi.configure({ mode: 'at', serialPort: 'COM3' });
```

---

### scan_ble_devices

扫描周围 BLE 设备。

**后端命令**: `scan_ble_devices`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| duration_ms | number | 是 | 扫描持续时间（毫秒） |

**返回**: `BleDeviceInfo[]`

```typescript
// 前端调用
const devices = await bleApi.scan({ timeout: 5000 });

// 返回类型
interface BleDeviceInfo {
  address: string;
  name?: string;
  rssi?: number;
  isConnectable: boolean;
  services?: string[];
  manufacturerData?: Record<string, number[]>;
}
```

---

### connect_ble

连接 BLE 设备。

**后端命令**: `connect_ble`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备地址 |

**返回**: `BleConnection`

```typescript
// 前端调用
const connection = await bleApi.connect({ address: 'AA:BB:CC:DD:EE:FF' });
```

---

### disconnect_ble

断开 BLE 连接。

**后端命令**: `disconnect_ble`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备地址 |

**返回**: `void`

```typescript
// 前端调用
await bleApi.disconnect('AA:BB:CC:DD:EE:FF');
```

---

### get_ble_connections

获取当前已连接的 BLE 设备列表。

**后端命令**: `get_ble_connections`

**参数**: 无

**返回**: `BleConnection[]`

---

### discover_ble_services

发现 BLE 设备的所有 GATT 服务。

**后端命令**: `discover_ble_services`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备地址 |

**返回**: `BleService[]`

```typescript
// 前端调用
const services = await bleApi.discoverServices({ deviceId: 'AA:BB:CC:DD:EE:FF' });
```

---

### discover_ble_characteristics

发现指定服务的所有 GATT 特征。

**后端命令**: `discover_ble_characteristics`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备地址 |
| service_uuid | string | 是 | 服务 UUID |

**返回**: `BleCharacteristic[]`

```typescript
// 前端调用
const characteristics = await bleApi.discoverCharacteristics({
  deviceId: 'AA:BB:CC:DD:EE:FF',
  serviceUuid: '6e400001-b5a3-f393-e0a9-e50e24dcca9e',
});
```

---

### read_ble_characteristic

读取特征值。

**后端命令**: `read_ble_characteristic`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备地址 |
| characteristic_uuid | string | 是 | 特征 UUID |

**返回**: `number[]`

```typescript
// 前端调用
const data = await bleApi.read({
  deviceId: 'AA:BB:CC:DD:EE:FF',
  characteristicUuid: '6e400003-b5a3-f393-e0a9-e50e24dcca9e',
});
```

---

### write_ble_characteristic

写入特征值。

**后端命令**: `write_ble_characteristic`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备地址 |
| characteristic_uuid | string | 是 | 特征 UUID |
| data | number[] | 是 | 要写入的数据 |

**返回**: `void`

```typescript
// 前端调用
await bleApi.write({
  deviceId: 'AA:BB:CC:DD:EE:FF',
  characteristicUuid: '6e400003-b5a3-f393-e0a9-e50e24dcca9e',
  data: [0x01, 0x02, 0x03],
});
```

---

### subscribe_ble_notify

订阅特征通知。

**后端命令**: `subscribe_ble_notify`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备地址 |
| characteristic_uuid | string | 是 | 特征 UUID |

**返回**: `void`

```typescript
// 前端调用
await bleApi.subscribe({
  deviceId: 'AA:BB:CC:DD:EE:FF',
  characteristicUuid: '6e400003-b5a3-f393-e0a9-e50e24dcca9e',
});
```

---

### unsubscribe_ble_notify

取消订阅特征通知。

**后端命令**: `unsubscribe_ble_notify`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备地址 |
| characteristic_uuid | string | 是 | 特征 UUID |

**返回**: `void`

```typescript
// 前端调用
await bleApi.unsubscribe('AA:BB:CC:DD:EE:FF', '6e400003-b5a3-f393-e0a9-e50e24dcca9e');
```

---

### get_ble_rssi

获取 BLE 信号强度。

**后端命令**: `get_ble_rssi`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备地址 |

**返回**: `number` (RSSI 值，单位 dBm)

```typescript
// 前端调用
const rssi = await bleApi.getRssi('AA:BB:CC:DD:EE:FF');
```

---

### get_ble_mode

获取当前 BLE 工作模式。

**后端命令**: `get_ble_mode`

**参数**: 无

**返回**: `string` ("native" 或 "at")

---

### is_ble_configured

检查 BLE 是否已配置。

**后端命令**: `is_ble_configured`

**参数**: 无

**返回**: `boolean`

---

### BLE 事件 (BLE Events)

#### ble-notify

BLE 特征通知事件。

```typescript
// 事件类型
interface BleNotifyEvent {
  address: string;
  char_uuid: string;
  data: number[];
}

// 前端监听
import { listen } from '@tauri-apps/api/event';
const unlisten = await listen<BleNotifyEvent>('ble-notify', (event) => {
  console.log('收到通知:', event.payload.data);
});
```

---

## 协议模块 (Protocol)

### load_protocol

加载协议插件。

**后端命令**: `load_protocol`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| plugin_id | string | 是 | 插件 ID |
| path | string | 是 | 插件脚本路径 |

**返回**: `PluginInfo`

```typescript
// 前端调用
await protocolApi.load({ plugin_id: 'my_protocol', path: '/path/to/script.lua' });
```

---

### unload_protocol

卸载协议插件。

**后端命令**: `unload_protocol`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| plugin_id | string | 是 | 插件 ID |

**返回**: `void`

```typescript
// 前端调用
await protocolApi.unload({ plugin_id: 'my_protocol' });
```

---

### enable_protocol

启用协议插件。

**后端命令**: `enable_protocol`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| plugin_id | string | 是 | 插件 ID |

**返回**: `void`

```typescript
// 前端调用
await protocolApi.enable({ plugin_id: 'my_protocol' });
```

---

### disable_protocol

禁用协议插件。

**后端命令**: `disable_protocol`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| plugin_id | string | 是 | 插件 ID |

**返回**: `void`

```typescript
// 前端调用
await protocolApi.disable({ plugin_id: 'my_protocol' });
```

---

### bind_protocol

绑定协议到设备。

**后端命令**: `bind_protocol`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| plugin_id | string | 是 | 插件 ID |
| device_id | string | 是 | 设备 ID |

**返回**: `void`

```typescript
// 前端调用
await protocolApi.bind({ plugin_id: 'my_protocol', device_id: 'COM1' });
```

---

### unbind_protocol

解绑协议。

**后端命令**: `unbind_protocol`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| plugin_id | string | 是 | 插件 ID |
| device_id | string | 是 | 设备 ID |

**返回**: `void`

```typescript
// 前端调用
await protocolApi.unbind({ plugin_id: 'my_protocol', device_id: 'COM1' });
```

---

### list_protocols

获取已加载的协议列表。

**后端命令**: `list_protocols`

**参数**: 无

**返回**: `PluginInfo[]`

```typescript
// 前端调用
const protocols = await protocolApi.list();
```

---

### get_protocol

获取单个协议信息。

**后端命令**: `get_protocol`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| plugin_id | string | 是 | 插件 ID |

**返回**: `PluginInfo`

---

### get_bound_protocols

获取设备绑定的协议。

**后端命令**: `get_bound_protocols`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| device_id | string | 是 | 设备 ID |

**返回**: `PluginInfo[]`

```typescript
// 前端调用
const protocols = await protocolApi.getBound({ device_id: 'COM1' });
```

---

## WebSocket 模块 (WebSocket)

### connect_websocket

连接 WebSocket 服务器。

**后端命令**: `connect_websocket`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| config | object | 是 | WebSocket 配置对象 |
| config.id | string | 是 | 连接 ID |
| config.url | string | 是 | 服务器 URL |
| config.reconnect | boolean | 否 | 是否自动重连，默认 true |
| config.reconnect_interval_ms | number | 否 | 重连间隔（毫秒），默认 5000 |
| config.max_reconnect_attempts | number | 否 | 最大重连次数，默认 10 |
| config.heartbeat_interval_ms | number | 否 | 心跳间隔（毫秒），默认 30000 |
| config.connection_timeout_ms | number | 否 | 连接超时（毫秒），默认 10000 |

**返回**: `string` (连接 ID)

```typescript
// 前端调用
await websocketApi.connect('ws1', 'ws://localhost:8080');
```

---

### send_websocket_message

发送 WebSocket 消息。

**后端命令**: `send_websocket_message`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| id | string | 是 | 连接 ID |
| message | string | 是 | 消息内容 |

**返回**: `void`

```typescript
// 前端调用
await websocketApi.send('ws1', 'Hello, server!');
```

---

### disconnect_websocket

断开 WebSocket 连接。

**后端命令**: `disconnect_websocket`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| id | string | 是 | 连接 ID |

**返回**: `void`

```typescript
// 前端调用
await websocketApi.disconnect('ws1');
```

---

### get_websocket_status

获取连接状态。

**后端命令**: `get_websocket_status`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| id | string | 是 | 连接 ID |

**返回**: `string` | `null` (状态: "Connected", "Connecting", "Disconnected", etc.)

---

### get_all_websocket_connections

获取所有连接 ID。

**后端命令**: `get_all_websocket_connections`

**参数**: 无

**返回**: `string[]`

---

### get_all_websocket_status

获取所有连接状态。

**后端命令**: `get_all_websocket_status`

**参数**: 无

**返回**: `Record<string, string>` (ID -> 状态)

---

### WebSocket 事件

#### websocket-status

连接状态变化事件。

```typescript
// 事件类型
interface WebSocketStatusEvent {
  id: string;
  status: string;
}
```

---

## 系统模块 (System)

### get_system_info

获取系统信息。

**后端命令**: `get_system_info`

**参数**: 无

**返回**:

```typescript
interface SystemInfo {
  os_name: string;
  os_version: string;
  arch: string;
  hostname: string;
  cpu_count: number;
  total_memory: number;
  app_version: string;
}
```

---

### get_system_status

获取系统状态。

**后端命令**: `get_system_status`

**参数**: 无

**返回**:

```typescript
interface SystemStatus {
  cpu_usage: number;
  memory_usage: number;
  used_memory: number;
  total_memory: number;
  uptime_secs: number;
  disk_usage: DiskUsage[];
}

interface DiskUsage {
  name: string;
  total_space: number;
  available_space: number;
  used_space: number;
  usage_percent: number;
}
```

---

### get_runtime_status

获取运行时状态。

**后端命令**: `get_runtime_status`

**参数**: 无

**返回**:

```typescript
interface RuntimeStatus {
  active_connections: number;
  serial_ports_open: number;
  ble_connections: number;
  websocket_connections: number;
  protocols_loaded: number;
  uptime_secs: number;
}
```

---

### get_app_version

获取应用版本。

**后端命令**: `get_app_version`

**参数**: 无

**返回**: `string`

---

### get_platform

获取平台信息。

**后端命令**: `get_platform`

**参数**: 无

**返回**: `string` ("windows", "macos", "linux")

---

### open_url

打开 URL。

**后端命令**: `open_url`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| url | string | 是 | 要打开的 URL |

**返回**: `void`

---

### show_in_folder

在文件管理器中显示文件。

**后端命令**: `show_in_folder`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| path | string | 是 | 文件或文件夹路径 |

**返回**: `void`

---

### configure_log

配置日志。

**后端命令**: `configure_log`

**参数**:

| 参数名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| level | string | 是 | 日志级别: "trace", "debug", "info", "warn", "error" |
| max_files | number | 否 | 最大日志文件数 |
| max_size_mb | number | 否 | 单个日志文件最大大小（MB） |
| console_enabled | boolean | 否 | 是否启用控制台日志 |
| file_enabled | boolean | 否 | 是否启用文件日志 |

**返回**: `void`

---

### get_log_config

获取日志配置。

**后端命令**: `get_log_config`

**参数**: 无

**返回**: `LogConfig`

---

## 类型定义汇总

### 前端类型 (src/types/)

```typescript
// Serial 类型
interface SerialPortInfo {
  name: string;
  port_type: string;
  manufacturer?: string;
  product?: string;
  serial_number?: string;
}

interface SerialConfig {
  baudRate: number;
  dataBits: 5 | 6 | 7 | 8;
  stopBits: 1 | 2;
  parity: 'none' | 'odd' | 'even';
  flowControl: 'none' | 'hardware' | 'software';
}

// BLE 类型
interface BleDeviceInfo {
  address: string;
  name?: string;
  rssi?: number;
  isConnectable: boolean;
  services?: string[];
  manufacturerData?: Record<string, number[]>;
}

interface BleConnection {
  deviceId: string;
  address: string;
  name?: string;
  isConnected: boolean;
  services: BleService[];
  connectedAt?: number;
  mtu?: number;
}

interface BleService {
  uuid: string;
  isPrimary: boolean;
  characteristics: BleCharacteristic[];
}

interface BleCharacteristic {
  uuid: string;
  properties: BleCharacteristicProperties;
  value?: number[];
}

// Protocol 类型
type PluginState = 'Unloaded' | 'Loaded' | 'Enabled' | 'Disabled' | 'Error';

interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string | null;
  author: string | null;
  path: string;
  state: PluginState;
  hooks: string[];
  bound_devices: string[];
  error_message: string | null;
}
```

### API 参数类型 (src/api/types.ts)

```typescript
interface BleConfigureParams {
  mode: 'native' | 'at';
  serialPort?: string;
}

interface BleConnectParams {
  address: string;
  timeout?: number;
}

interface BleDiscoverServicesParams {
  deviceId: string;
}

interface BleDiscoverCharacteristicsParams {
  deviceId: string;
  serviceUuid: string;
}

interface BleReadParams {
  deviceId: string;
  characteristicUuid: string;
}

interface BleWriteParams {
  deviceId: string;
  characteristicUuid: string;
  data: number[];
  withoutResponse?: boolean;
}

interface BleSubscribeParams {
  deviceId: string;
  characteristicUuid: string;
}

interface ProtocolLoadParams {
  plugin_id: string;
  path: string;
}

interface ProtocolBindParams {
  plugin_id: string;
  device_id: string;
}
```

---

## 命名规范

本文档遵循以下命名规范：

| 类型 | 规范 | 示例 |
|------|------|------|
| Rust 函数/变量 | snake_case | `open_serial_port`, `port_name` |
| Rust 结构体/特征 | UpperCamelCase | `SerialManager`, `BleBackend` |
| TypeScript 变量/函数 | camelCase | `deviceId`, `scanDevices` |
| 数据库/配置 | snake_case | `serial_number`, `baud_rate` |

---

## 更新日志

- **2026-04-02**: 统一使用蛇形命名规范，修复 BLE 和 Serial 模块所有参数不匹配问题
