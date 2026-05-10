# ComBridge API 文档

本文档整理了 ComBridge 项目前后端交互的所有 Tauri 命令格式，采用蛇形命名（snake\_case）规范。

## 目录

- [串口模块 (Serial)](#串口模块-serial)
- [BLE 模块 (BLE)](#ble-模块-ble)
- [协议模块 (Protocol)](#协议模块-protocol)
- [系统模块 (System)](#系统模块-system)
- [Dashboard 模块 (Dashboard)](#dashboard-模块-dashboard)
- [GH3036 模块 (GH3036)](#gh3036-模块-gh3036)
- [波形模块 (Waveform)](#波形模块-waveform)
- [状态模块 (State)](#状态模块-state)
- [偏好设置模块 (Preferences)](#偏好设置模块-preferences)
- [事件汇总](#事件汇总)
- [类型定义汇总](#类型定义汇总)
- [命名规范](#命名规范)

***

## 串口模块 (Serial)

### scan\_serial\_ports

扫描可用串口列表。

**后端命令**: `scan_serial_ports`

**参数**: 无

**返回**: `PortInfo[]`

```typescript
const ports = await invoke<PortInfo[]>('scan_serial_ports');

interface PortInfo {
  name: string;
  port_type: string;
  manufacturer?: string;
  product?: string;
  serial_number?: string;
}
```

***

### open\_serial\_port

打开指定串口。

**后端命令**: `open_serial_port`

**参数**:

| 参数名                      | 类型     | 必填 | 描述                                            |
| ------------------------ | ------ | -- | --------------------------------------------- |
| config                   | object | 是  | 串口配置对象                                        |
| config.port\_name        | string | 是  | 端口名称                                          |
| config.baud\_rate        | string | 否  | 波特率（字符串），如 "9600", "115200"                   |
| config.data\_bits        | number | 否  | 数据位（5, 6, 7, 8），默认 8                          |
| config.parity            | string | 否  | 校验位（"none", "odd", "even"），默认 "none"          |
| config.stop\_bits        | number | 否  | 停止位（1, 2），默认 1                                |
| config.flow\_control     | string | 否  | 流控制（"none", "hardware", "software"），默认 "none" |
| config.timeout\_ms       | number | 否  | 超时时间（毫秒），默认 1000                              |
| config.pack\_timeout\_ms | number | 否  | 数据包超时（毫秒），默认 50                               |

**返回**: `void`

```typescript
await invoke('open_serial_port', {
  config: {
    portName: 'COM1',
    baudRate: '115200',
    dataBits: 8,
    parity: 'none',
    stopBits: 1,
    flowControl: 'none',
  },
});
```

***

### close\_serial\_port

关闭指定串口。

**后端命令**: `close_serial_port`

**参数**:

| 参数名        | 类型     | 必填 | 描述   |
| ---------- | ------ | -- | ---- |
| port\_name | string | 是  | 端口名称 |

**返回**: `void`

```typescript
await invoke('close_serial_port', { portName: 'COM1' });
```

***

### send\_serial\_data

向串口发送数据。

**后端命令**: `send_serial_data`

**参数**:

| 参数名        | 类型        | 必填 | 描述       |
| ---------- | --------- | -- | -------- |
| port\_name | string    | 是  | 端口名称     |
| data       | number\[] | 是  | 要发送的字节数据 |

**返回**: `number` (发送的字节数)

```typescript
const bytesWritten = await invoke<number>('send_serial_data', {
  portName: 'COM1',
  data: [0x01, 0x02, 0x03],
});
```

***

### get\_open\_ports

获取已打开的端口列表。

**后端命令**: `get_open_ports`

**参数**: 无

**返回**: `string[]`

```typescript
const ports = await invoke<string[]>('get_open_ports');
```

***

### is\_port\_open

检查端口是否已打开。

**后端命令**: `is_port_open`

**参数**:

| 参数名        | 类型     | 必填 | 描述   |
| ---------- | ------ | -- | ---- |
| port\_name | string | 是  | 端口名称 |

**返回**: `boolean`

```typescript
const isOpen = await invoke<boolean>('is_port_open', { portName: 'COM1' });
```

***

### export\_serial\_data

导出串口数据到文件。

**后端命令**: `export_serial_data`

**参数**:

| 参数名        | 类型                 | 必填 | 描述       |
| ---------- | ------------------ | -- | -------- |
| port\_name | string             | 是  | 端口名称     |
| all\_data  | ExportDataEntry\[] | 是  | 所有数据条目   |
| rx\_data   | number\[]          | 是  | 接收数据原始字节 |

**返回**: `ExportResult`

```typescript
const result = await invoke<ExportResult>('export_serial_data', {
  portName: 'COM1',
  allData: [...],
  rxData: [...],
});

interface ExportResult {
  log_path: string;
  dat_path: string;
}

interface ExportDataEntry {
  timestamp: number;
  data: number[];
  direction: string;
}
```

***

## BLE 模块 (BLE)

### configure\_ble

配置 BLE 工作模式。

**后端命令**: `configure_ble`

**参数**:

| 参数名                | 类型     | 必填      | 描述                 |
| ------------------ | ------ | ------- | ------------------ |
| config             | object | 是       | BLE 配置对象           |
| config.mode        | string | 是       | 模式："native" 或 "at" |
| config.port\_name  | string | AT 模式必填 | AT 模式下的串口名称        |
| config.baud\_rate  | number | 否       | AT 模式波特率，默认 115200 |
| config.timeout\_ms | number | 否       | AT 指令超时，默认 1000    |
| config.tx\_uuid    | string | 否       | AT 模式写特征 UUID      |
| config.rx\_uuid    | string | 否       | AT 模式通知特征 UUID     |
| config.srv\_uuid   | string | 否       | AT 模式服务 UUID       |

**返回**: `void`

```typescript
await invoke('configure_ble', {
  config: { mode: 'native' },
});
// AT 模式
await invoke('configure_ble', {
  config: { mode: 'at', portName: 'COM3', baudRate: 115200 },
});
```

***

### scan\_ble\_devices

扫描周围 BLE 设备。

**后端命令**: `scan_ble_devices`

**参数**:

| 参数名          | 类型     | 必填 | 描述         |
| ------------ | ------ | -- | ---------- |
| duration\_ms | number | 是  | 扫描持续时间（毫秒） |

**返回**: `BleDevice[]`

```typescript
const devices = await invoke<BleDevice[]>('scan_ble_devices', { durationMs: 5000 });

interface BleDevice {
  address: string;
  name?: string;
  rssi?: number;
  is_connectable: boolean;
  services?: string[];
  manufacturer_data?: Record<string, number[]>;
}
```

***

### stop\_ble\_scan

停止 BLE 扫描。

**后端命令**: `stop_ble_scan`

**参数**: 无

**返回**: `BleDevice[]` (扫描期间发现的所有设备)

```typescript
const devices = await invoke<BleDevice[]>('stop_ble_scan');
```

***

### connect\_ble

连接 BLE 设备。

**后端命令**: `connect_ble`

**参数**:

| 参数名        | 类型     | 必填 | 描述   |
| ---------- | ------ | -- | ---- |
| device\_id | string | 是  | 设备地址 |

**返回**: `BleConnection`

```typescript
const connection = await invoke<BleConnection>('connect_ble', { deviceId: 'AA:BB:CC:DD:EE:FF' });
```

***

### disconnect\_ble

断开 BLE 连接。

**后端命令**: `disconnect_ble`

**参数**:

| 参数名        | 类型     | 必填 | 描述   |
| ---------- | ------ | -- | ---- |
| device\_id | string | 是  | 设备地址 |

**返回**: `void`

```typescript
await invoke('disconnect_ble', { deviceId: 'AA:BB:CC:DD:EE:FF' });
```

***

### get\_ble\_connections

获取当前已连接的 BLE 设备列表。

**后端命令**: `get_ble_connections`

**参数**: 无

**返回**: `BleConnection[]`

```typescript
const connections = await invoke<BleConnection[]>('get_ble_connections');
```

***

### discover\_ble\_services

发现 BLE 设备的所有 GATT 服务。

**后端命令**: `discover_ble_services`

**参数**:

| 参数名        | 类型     | 必填 | 描述   |
| ---------- | ------ | -- | ---- |
| device\_id | string | 是  | 设备地址 |

**返回**: `BleService[]`

```typescript
const services = await invoke<BleService[]>('discover_ble_services', {
  deviceId: 'AA:BB:CC:DD:EE:FF',
});
```

***

### discover\_ble\_characteristics

发现指定服务的所有 GATT 特征。

**后端命令**: `discover_ble_characteristics`

**参数**:

| 参数名           | 类型     | 必填 | 描述      |
| ------------- | ------ | -- | ------- |
| device\_id    | string | 是  | 设备地址    |
| service\_uuid | string | 是  | 服务 UUID |

**返回**: `BleCharacteristic[]`

```typescript
const chars = await invoke<BleCharacteristic[]>('discover_ble_characteristics', {
  deviceId: 'AA:BB:CC:DD:EE:FF',
  serviceUuid: '6e400001-b5a3-f393-e0a9-e50e24dcca9e',
});
```

***

### read\_ble\_characteristic

读取特征值。

**后端命令**: `read_ble_characteristic`

**参数**:

| 参数名                  | 类型     | 必填 | 描述      |
| -------------------- | ------ | -- | ------- |
| device\_id           | string | 是  | 设备地址    |
| characteristic\_uuid | string | 是  | 特征 UUID |

**返回**: `number[]`

```typescript
const data = await invoke<number[]>('read_ble_characteristic', {
  deviceId: 'AA:BB:CC:DD:EE:FF',
  characteristicUuid: '6e400003-b5a3-f393-e0a9-e50e24dcca9e',
});
```

***

### write\_ble\_characteristic

写入特征值（等待响应）。

**后端命令**: `write_ble_characteristic`

**参数**:

| 参数名                  | 类型        | 必填 | 描述      |
| -------------------- | --------- | -- | ------- |
| device\_id           | string    | 是  | 设备地址    |
| characteristic\_uuid | string    | 是  | 特征 UUID |
| data                 | number\[] | 是  | 要写入的数据  |

**返回**: `void`

```typescript
await invoke('write_ble_characteristic', {
  deviceId: 'AA:BB:CC:DD:EE:FF',
  characteristicUuid: '6e400002-b5a3-f393-e0a9-e50e24dcca9e',
  data: [0x01, 0x02, 0x03],
});
```

***

### write\_ble\_without\_response

无响应写入特征值。

**后端命令**: `write_ble_without_response`

**参数**:

| 参数名                  | 类型        | 必填 | 描述      |
| -------------------- | --------- | -- | ------- |
| device\_id           | string    | 是  | 设备地址    |
| characteristic\_uuid | string    | 是  | 特征 UUID |
| data                 | number\[] | 是  | 要写入的数据  |

**返回**: `void`

```typescript
await invoke('write_ble_without_response', {
  deviceId: 'AA:BB:CC:DD:EE:FF',
  characteristicUuid: '6e400002-b5a3-f393-e0a9-e50e24dcca9e',
  data: [0x01, 0x02, 0x03],
});
```

***

### subscribe\_ble\_notify

订阅特征通知。

**后端命令**: `subscribe_ble_notify`

**参数**:

| 参数名                  | 类型     | 必填 | 描述      |
| -------------------- | ------ | -- | ------- |
| device\_id           | string | 是  | 设备地址    |
| characteristic\_uuid | string | 是  | 特征 UUID |

**返回**: `void`

```typescript
await invoke('subscribe_ble_notify', {
  deviceId: 'AA:BB:CC:DD:EE:FF',
  characteristicUuid: '6e400003-b5a3-f393-e0a9-e50e24dcca9e',
});
```

***

### unsubscribe\_ble\_notify

取消订阅特征通知。

**后端命令**: `unsubscribe_ble_notify`

**参数**:

| 参数名                  | 类型     | 必填 | 描述      |
| -------------------- | ------ | -- | ------- |
| device\_id           | string | 是  | 设备地址    |
| characteristic\_uuid | string | 是  | 特征 UUID |

**返回**: `void`

```typescript
await invoke('unsubscribe_ble_notify', {
  deviceId: 'AA:BB:CC:DD:EE:FF',
  characteristicUuid: '6e400003-b5a3-f393-e0a9-e50e24dcca9e',
});
```

***

### get\_ble\_rssi

获取 BLE 信号强度。

**后端命令**: `get_ble_rssi`

**参数**:

| 参数名     | 类型     | 必填 | 描述   |
| ------- | ------ | -- | ---- |
| address | string | 是  | 设备地址 |

**返回**: `number` (RSSI 值，单位 dBm)

```typescript
const rssi = await invoke<number>('get_ble_rssi', { address: 'AA:BB:CC:DD:EE:FF' });
```

***

### get\_ble\_mode

获取当前 BLE 工作模式。

**后端命令**: `get_ble_mode`

**参数**: 无

**返回**: `string` ("native" 或 "at")

```typescript
const mode = await invoke<string>('get_ble_mode');
```

***

### is\_ble\_configured

检查 BLE 是否已配置。

**后端命令**: `is_ble_configured`

**参数**: 无

**返回**: `boolean`

```typescript
const configured = await invoke<boolean>('is_ble_configured');
```

***

### set\_ble\_mtu

设置 BLE MTU 大小。

**后端命令**: `set_ble_mtu`

**参数**:

| 参数名        | 类型     | 必填 | 描述         |
| ---------- | ------ | -- | ---------- |
| device\_id | string | 是  | 设备地址       |
| mtu        | number | 是  | 请求的 MTU 大小 |

**返回**: `number` (协商后的实际 MTU)

```typescript
const actualMtu = await invoke<number>('set_ble_mtu', { deviceId: 'AA:BB:CC:DD:EE:FF', mtu: 512 });
```

***

### get\_ble\_subscriptions

获取设备已订阅的特征列表。

**后端命令**: `get_ble_subscriptions`

**参数**:

| 参数名        | 类型     | 必填 | 描述   |
| ---------- | ------ | -- | ---- |
| device\_id | string | 是  | 设备地址 |

**返回**: `string[]` (已订阅的特征 UUID 列表)

```typescript
const subscriptions = await invoke<string[]>('get_ble_subscriptions', {
  deviceId: 'AA:BB:CC:DD:EE:FF',
});
```

***

### get\_at\_config

获取 AT 模式配置。

**后端命令**: `get_at_config`

**参数**: 无

**返回**: `AtConfig`

```typescript
const config = await invoke<AtConfig>('get_at_config');

interface AtConfig {
  port_name: string;
  baud_rate: number;
  timeout_ms: number;
  tx_uuid?: string;
  rx_uuid?: string;
  srv_uuid?: string;
}
```

***

### update\_at\_uuid\_config

更新 AT 模式的 UUID 配置。

**后端命令**: `update_at_uuid_config`

**参数**:

| 参数名       | 类型     | 必填 | 描述        |
| --------- | ------ | -- | --------- |
| tx\_uuid  | string | 否  | 写特征 UUID  |
| rx\_uuid  | string | 否  | 通知特征 UUID |
| srv\_uuid | string | 否  | 服务 UUID   |

**返回**: `void`

```typescript
await invoke('update_at_uuid_config', {
  txUuid: '6e400002-b5a3-f393-e0a9-e50e24dcca9e',
  rxUuid: '6e400003-b5a3-f393-e0a9-e50e24dcca9e',
  srvUuid: '6e400001-b5a3-f393-e0a9-e50e24dcca9e',
});
```

***

### get\_at\_tabs

获取 AT 模式连接标签页列表。

**后端命令**: `get_at_tabs`

**参数**: 无

**返回**: `AtConnectionTab[]`

```typescript
const tabs = await invoke<AtConnectionTab[]>('get_at_tabs');
```

***

### get\_at\_tab

获取指定 AT 连接标签页。

**后端命令**: `get_at_tab`

**参数**:

| 参数名     | 类型     | 必填 | 描述     |
| ------- | ------ | -- | ------ |
| tab\_id | string | 是  | 标签页 ID |

**返回**: `AtConnectionTab | null`

```typescript
const tab = await invoke<AtConnectionTab | null>('get_at_tab', { tabId: 'tab-1' });
```

***

### clear\_at\_tab\_data

清空 AT 连接标签页数据。

**后端命令**: `clear_at_tab_data`

**参数**:

| 参数名     | 类型     | 必填 | 描述     |
| ------- | ------ | -- | ------ |
| tab\_id | string | 是  | 标签页 ID |

**返回**: `void`

```typescript
await invoke('clear_at_tab_data', { tabId: 'tab-1' });
```

***

### remove\_at\_tab

移除 AT 连接标签页。

**后端命令**: `remove_at_tab`

**参数**:

| 参数名     | 类型     | 必填 | 描述     |
| ------- | ------ | -- | ------ |
| tab\_id | string | 是  | 标签页 ID |

**返回**: `void`

```typescript
await invoke('remove_at_tab', { tabId: 'tab-1' });
```

***

### send\_at\_data

通过 AT 透传模式发送数据。

**后端命令**: `send_at_data`

**参数**:

| 参数名        | 类型        | 必填 | 描述     |
| ---------- | --------- | -- | ------ |
| device\_id | string    | 是  | 设备地址   |
| data       | number\[] | 是  | 要发送的数据 |

**返回**: `void`

```typescript
await invoke('send_at_data', { deviceId: 'AA:BB:CC:DD:EE:FF', data: [0x01, 0x02] });
```

***

## 协议模块 (Protocol)

### load\_protocol

加载协议插件。

**后端命令**: `load_protocol`

**参数**:

| 参数名        | 类型     | 必填 | 描述     |
| ---------- | ------ | -- | ------ |
| plugin\_id | string | 是  | 插件 ID  |
| path       | string | 是  | 插件脚本路径 |

**返回**: `PluginInfo`

```typescript
const info = await invoke<PluginInfo>('load_protocol', {
  pluginId: 'my_protocol',
  path: '/path/to/script.lua',
});
```

***

### unload\_protocol

卸载协议插件。

**后端命令**: `unload_protocol`

**参数**:

| 参数名        | 类型     | 必填 | 描述    |
| ---------- | ------ | -- | ----- |
| plugin\_id | string | 是  | 插件 ID |

**返回**: `void`

```typescript
await invoke('unload_protocol', { pluginId: 'my_protocol' });
```

***

### enable\_protocol

启用协议插件。

**后端命令**: `enable_protocol`

**参数**:

| 参数名        | 类型     | 必填 | 描述    |
| ---------- | ------ | -- | ----- |
| plugin\_id | string | 是  | 插件 ID |

**返回**: `void`

```typescript
await invoke('enable_protocol', { pluginId: 'my_protocol' });
```

***

### disable\_protocol

禁用协议插件。

**后端命令**: `disable_protocol`

**参数**:

| 参数名        | 类型     | 必填 | 描述    |
| ---------- | ------ | -- | ----- |
| plugin\_id | string | 是  | 插件 ID |

**返回**: `void`

```typescript
await invoke('disable_protocol', { pluginId: 'my_protocol' });
```

***

### bind\_protocol

绑定协议到设备。

**后端命令**: `bind_protocol`

**参数**:

| 参数名        | 类型     | 必填 | 描述    |
| ---------- | ------ | -- | ----- |
| plugin\_id | string | 是  | 插件 ID |
| device\_id | string | 是  | 设备 ID |

**返回**: `void`

```typescript
await invoke('bind_protocol', { pluginId: 'my_protocol', deviceId: 'COM1' });
```

***

### unbind\_protocol

解绑协议。

**后端命令**: `unbind_protocol`

**参数**:

| 参数名        | 类型     | 必填 | 描述    |
| ---------- | ------ | -- | ----- |
| plugin\_id | string | 是  | 插件 ID |
| device\_id | string | 是  | 设备 ID |

**返回**: `void`

```typescript
await invoke('unbind_protocol', { pluginId: 'my_protocol', deviceId: 'COM1' });
```

***

### list\_protocols

获取已加载的协议列表。

**后端命令**: `list_protocols`

**参数**: 无

**返回**: `PluginInfo[]`

```typescript
const protocols = await invoke<PluginInfo[]>('list_protocols');
```

***

### get\_protocol

获取单个协议信息。

**后端命令**: `get_protocol`

**参数**:

| 参数名        | 类型     | 必填 | 描述    |
| ---------- | ------ | -- | ----- |
| plugin\_id | string | 是  | 插件 ID |

**返回**: `PluginInfo`

```typescript
const info = await invoke<PluginInfo>('get_protocol', { pluginId: 'my_protocol' });
```

***

### get\_bound\_protocols

获取设备绑定的协议。

**后端命令**: `get_bound_protocols`

**参数**:

| 参数名        | 类型     | 必填 | 描述    |
| ---------- | ------ | -- | ----- |
| device\_id | string | 是  | 设备 ID |

**返回**: `PluginInfo[]`

```typescript
const protocols = await invoke<PluginInfo[]>('get_bound_protocols', { deviceId: 'COM1' });
```

***

***

## 系统模块 (System)

### get\_system\_info

获取系统信息。

**后端命令**: `get_system_info`

**参数**: 无

**返回**: `SystemInfo`

```typescript
const info = await invoke<SystemInfo>('get_system_info');

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

***

### get\_system\_status

获取系统状态。

**后端命令**: `get_system_status`

**参数**: 无

**返回**: `SystemStatus`

```typescript
const status = await invoke<SystemStatus>('get_system_status');

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

***

### get\_runtime\_status

获取运行时状态。

**后端命令**: `get_runtime_status`

**参数**: 无

**返回**: `RuntimeStatus`

```typescript
const status = await invoke<RuntimeStatus>('get_runtime_status');

interface RuntimeStatus {
  active_connections: number;
  serial_ports_open: number;
  ble_connections: number;
  protocols_loaded: number;
  uptime_secs: number;
}
```

***

### get\_app\_version

获取应用版本。

**后端命令**: `get_app_version`

**参数**: 无

**返回**: `string`

```typescript
const version = await invoke<string>('get_app_version');
```

***

### get\_platform

获取平台信息。

**后端命令**: `get_platform`

**参数**: 无

**返回**: `string` ("windows", "macos", "linux")

```typescript
const platform = await invoke<string>('get_platform');
```

***

### open\_url

打开 URL。

**后端命令**: `open_url`

**参数**:

| 参数名 | 类型     | 必填 | 描述       |
| --- | ------ | -- | -------- |
| url | string | 是  | 要打开的 URL |

**返回**: `void`

```typescript
await invoke('open_url', { url: 'https://github.com' });
```

***

### show\_in\_folder

在文件管理器中显示文件。

**后端命令**: `show_in_folder`

**参数**:

| 参数名  | 类型     | 必填 | 描述       |
| ---- | ------ | -- | -------- |
| path | string | 是  | 文件或文件夹路径 |

**返回**: `void`

```typescript
await invoke('show_in_folder', { path: 'C:\\Users\\data.log' });
```

***

### configure\_log

配置日志。

**后端命令**: `configure_log`

**参数**:

| 参数名                     | 类型      | 必填 | 描述                                              |
| ----------------------- | ------- | -- | ----------------------------------------------- |
| config                  | object  | 是  | 日志配置对象                                          |
| config.level            | string  | 是  | 日志级别: "trace", "debug", "info", "warn", "error" |
| config.max\_files       | number  | 否  | 最大日志文件数                                         |
| config.max\_size\_mb    | number  | 否  | 单个日志文件最大大小（MB）                                  |
| config.console\_enabled | boolean | 否  | 是否启用控制台日志                                       |
| config.file\_enabled    | boolean | 否  | 是否启用文件日志                                        |

**返回**: `void`

```typescript
await invoke('configure_log', {
  config: { level: 'info', maxFiles: 10, maxSizeMb: 10, consoleEnabled: true, fileEnabled: true },
});
```

***

### get\_log\_config

获取日志配置。

**后端命令**: `get_log_config`

**参数**: 无

**返回**: `LogConfig`

```typescript
const config = await invoke<LogConfig>('get_log_config');
```

***

### get\_window\_status

获取窗口状态信息。

**后端命令**: `get_window_status`

**参数**: 无

**返回**: `WindowStatus`

```typescript
const status = await invoke<WindowStatus>('get_window_status');

interface WindowStatus {
  label: string;
  title: string;
  visible: boolean;
  focused: boolean;
  maximized: boolean;
  minimized: boolean;
  fullscreen: boolean;
  width: number;
  height: number;
  x: number;
  y: number;
}
```

***

### show\_main\_window

显示主窗口（带重试机制）。

**后端命令**: `show_main_window`

**参数**: 无

**返回**: `void`

```typescript
await invoke('show_main_window');
```

***

### open\_devtools

打开开发者工具（仅 devtools 特性启用时可用）。

**后端命令**: `open_devtools`

**参数**: 无

**返回**: `void`

```typescript
await invoke('open_devtools');
```

***

### close\_devtools

关闭开发者工具（仅 devtools 特性启用时可用）。

**后端命令**: `close_devtools`

**参数**: 无

**返回**: `void`

```typescript
await invoke('close_devtools');
```

***

## Dashboard 模块 (Dashboard)

### get\_parser\_scripts

获取解析脚本列表。

**后端命令**: `get_parser_scripts`

**参数**: 无

**返回**: `ParserScriptInfo[]`

```typescript
const scripts = await invoke<ParserScriptInfo[]>('get_parser_scripts');
```

***

### get\_parser\_script\_content

获取解析脚本内容。

**后端命令**: `get_parser_script_content`

**参数**:

| 参数名  | 类型     | 必填 | 描述   |
| ---- | ------ | -- | ---- |
| name | string | 是  | 脚本名称 |

**返回**: `string` (脚本内容)

```typescript
const content = await invoke<string>('get_parser_script_content', { name: 'my_parser' });
```

***

### save\_parser\_script

保存解析脚本。

**后端命令**: `save_parser_script`

**参数**:

| 参数名     | 类型     | 必填 | 描述   |
| ------- | ------ | -- | ---- |
| name    | string | 是  | 脚本名称 |
| content | string | 是  | 脚本内容 |

**返回**: `void`

```typescript
await invoke('save_parser_script', { name: 'my_parser', content: '...' });
```

***

### delete\_parser\_script

删除解析脚本。

**后端命令**: `delete_parser_script`

**参数**:

| 参数名  | 类型     | 必填 | 描述   |
| ---- | ------ | -- | ---- |
| name | string | 是  | 脚本名称 |

**返回**: `void`

```typescript
await invoke('delete_parser_script', { name: 'my_parser' });
```

***

### execute\_parser\_script

执行解析脚本。

**后端命令**: `execute_parser_script`

**参数**:

| 参数名  | 类型     | 必填 | 描述     |
| ---- | ------ | -- | ------ |
| name | string | 是  | 脚本名称   |
| data | string | 是  | 要解析的数据 |

**返回**: `Record<string, number>` (解析结果键值对)

```typescript
const result = await invoke<Record<string, number>>('execute_parser_script', {
  name: 'my_parser',
  data: '{"temp": 25.5}',
});
```

***

### init\_default\_parser\_scripts

初始化默认解析脚本。

**后端命令**: `init_default_parser_scripts`

**参数**: 无

**返回**: `void`

```typescript
await invoke('init_default_parser_scripts');
```

***

### analyze\_json\_structure

分析 JSON 结构。

**后端命令**: `analyze_json_structure`

**参数**:

| 参数名           | 类型     | 必填 | 描述         |
| ------------- | ------ | -- | ---------- |
| json\_content | string | 是  | JSON 内容字符串 |

**返回**: `JsonStructureInfo`

```typescript
const structure = await invoke<JsonStructureInfo>('analyze_json_structure', {
  jsonContent: '{"temp": 25.5, "humidity": 60}',
});
```

***

### generate\_parser\_from\_json

从 JSON 生成解析脚本。

**后端命令**: `generate_parser_from_json`

**参数**:

| 参数名              | 类型        | 必填 | 描述         |
| ---------------- | --------- | -- | ---------- |
| json\_content    | string    | 是  | JSON 内容字符串 |
| script\_name     | string    | 是  | 脚本名称       |
| selected\_fields | string\[] | 是  | 选择的字段列表    |

**返回**: `string` (生成的脚本内容)

```typescript
const script = await invoke<string>('generate_parser_from_json', {
  jsonContent: '{"temp": 25.5}',
  scriptName: 'temp_parser',
  selectedFields: ['temp'],
});
```

***

### get\_parser\_defined\_fields

获取解析脚本定义的字段。

**后端命令**: `get_parser_defined_fields`

**参数**:

| 参数名          | 类型     | 必填 | 描述   |
| ------------ | ------ | -- | ---- |
| script\_name | string | 是  | 脚本名称 |

**返回**: `FieldDefinition[]`

```typescript
const fields = await invoke<FieldDefinition[]>('get_parser_defined_fields', {
  scriptName: 'my_parser',
});
```

***

### merge\_json\_to\_parser

合并 JSON 到已有解析脚本。

**后端命令**: `merge_json_to_parser`

**参数**:

| 参数名              | 类型        | 必填 | 描述         |
| ---------------- | --------- | -- | ---------- |
| json\_content    | string    | 是  | JSON 内容字符串 |
| script\_name     | string    | 是  | 目标脚本名称     |
| selected\_fields | string\[] | 是  | 要合并的字段列表   |

**返回**: `string` (合并后的脚本内容)

```typescript
const script = await invoke<string>('merge_json_to_parser', {
  jsonContent: '{"humidity": 60}',
  scriptName: 'temp_parser',
  selectedFields: ['humidity'],
});
```

***

### get\_json\_files

获取 JSON 配置文件列表。

**后端命令**: `get_json_files`

**参数**: 无

**返回**: `string[]`

```typescript
const files = await invoke<string[]>('get_json_files');
```

***

### save\_json\_file

保存 JSON 配置文件。

**后端命令**: `save_json_file`

**参数**:

| 参数名        | 类型                  | 必填 | 描述                |
| ---------- | ------------------- | -- | ----------------- |
| file\_name | string              | 是  | 文件名称              |
| config     | DashboardJsonConfig | 是  | Dashboard JSON 配置 |

**返回**: `void`

```typescript
await invoke('save_json_file', { fileName: 'dashboard1', config: { ... } });
```

***

### delete\_json\_file

删除 JSON 配置文件。

**后端命令**: `delete_json_file`

**参数**:

| 参数名        | 类型     | 必填 | 描述   |
| ---------- | ------ | -- | ---- |
| file\_name | string | 是  | 文件名称 |

**返回**: `void`

```typescript
await invoke('delete_json_file', { fileName: 'dashboard1' });
```

***

### load\_json\_file

加载 JSON 配置文件。

**后端命令**: `load_json_file`

**参数**:

| 参数名        | 类型     | 必填 | 描述   |
| ---------- | ------ | -- | ---- |
| file\_name | string | 是  | 文件名称 |

**返回**: `DashboardJsonConfig`

```typescript
const config = await invoke<DashboardJsonConfig>('load_json_file', { fileName: 'dashboard1' });
```

***

## GH3036 模块 (GH3036)

### gh3036\_init

初始化 GH3036 管理器。

**后端命令**: `gh3036_init`

**参数**: 无

**返回**: `void`

```typescript
await invoke('gh3036_init');
```

***

### gh3036\_is\_initialized

检查 GH3036 是否已初始化。

**后端命令**: `gh3036_is_initialized`

**参数**: 无

**返回**: `boolean`

```typescript
const initialized = await invoke<boolean>('gh3036_is_initialized');
```

***

### gh3036\_configure\_tx\_channel

配置 TX 通道。

**后端命令**: `gh3036_configure_tx_channel`

**参数**:

| 参数名                  | 类型     | 必填 | 描述                    |
| -------------------- | ------ | -- | --------------------- |
| channel\_type        | string | 是  | 通道类型："serial" 或 "ble" |
| device\_id           | string | 是  | 设备 ID                 |
| characteristic\_uuid | string | 否  | BLE 特征 UUID（BLE 通道必填） |

**返回**: `void`

```typescript
await invoke('gh3036_configure_tx_channel', {
  channelType: 'serial',
  deviceId: 'COM1',
  characteristicUuid: null,
});
```

***

### gh3036\_configure\_rx\_channel

配置 RX 通道。

**后端命令**: `gh3036_configure_rx_channel`

**参数**:

| 参数名                  | 类型     | 必填 | 描述                    |
| -------------------- | ------ | -- | --------------------- |
| channel\_type        | string | 是  | 通道类型："serial" 或 "ble" |
| device\_id           | string | 是  | 设备 ID                 |
| characteristic\_uuid | string | 否  | BLE 特征 UUID（BLE 通道必填） |

**返回**: `void`

```typescript
await invoke('gh3036_configure_rx_channel', {
  channelType: 'ble',
  deviceId: 'AA:BB:CC:DD:EE:FF',
  characteristicUuid: '6e400003-b5a3-f393-e0a9-e50e24dcca9e',
});
```

***

### gh3036\_get\_channels

获取当前 TX/RX 通道配置。

**后端命令**: `gh3036_get_channels`

**参数**: 无

**返回**: `[ChannelConfig | null, ChannelConfig | null]` (TX 配置, RX 配置)

```typescript
const [txConfig, rxConfig] = await invoke<[ChannelConfig | null, ChannelConfig | null]>('gh3036_get_channels');

interface ChannelConfig {
  channel_type: string;
  device_id: string;
  characteristic_uuid?: string;
}
```

***

### gh3036\_send\_data

通过 GH3036 发送数据。

**后端命令**: `gh3036_send_data`

**参数**:

| 参数名  | 类型        | 必填 | 描述       |
| ---- | --------- | -- | -------- |
| data | number\[] | 是  | 要发送的字节数据 |

**返回**: `void`

```typescript
await invoke('gh3036_send_data', { data: [0x01, 0x02, 0x03] });
```

***

### gh3036\_set\_csv\_config

设置 CSV 导出配置。

**后端命令**: `gh3036_set_csv_config`

**参数**:

| 参数名         | 类型      | 必填 | 描述          |
| ----------- | ------- | -- | ----------- |
| enabled     | boolean | 是  | 是否启用 CSV 导出 |
| output\_dir | string  | 是  | 输出目录路径      |

**返回**: `void`

```typescript
await invoke('gh3036_set_csv_config', { enabled: true, outputDir: './output' });
```

***

### gh3036\_get\_csv\_config

获取 CSV 导出配置。

**后端命令**: `gh3036_get_csv_config`

**参数**: 无

**返回**: `CsvConfig`

```typescript
const config = await invoke<CsvConfig>('gh3036_get_csv_config');

interface CsvConfig {
  enabled: boolean;
  output_dir: string;
}
```

***

### gh3036\_get\_rpc\_commands

获取 RPC 命令列表。

**后端命令**: `gh3036_get_rpc_commands`

**参数**: 无

**返回**: `RpcCommand[]`

```typescript
const commands = await invoke<RpcCommand[]>('gh3036_get_rpc_commands');
```

***

### gh3036\_execute\_rpc

执行 RPC 命令。

**后端命令**: `gh3036_execute_rpc`

**参数**:

| 参数名          | 类型        | 必填 | 描述     |
| ------------ | --------- | -- | ------ |
| command\_key | string    | 是  | 命令键名   |
| params       | string\[] | 是  | 命令参数列表 |

**返回**: `number[]` (响应数据)

```typescript
const response = await invoke<number[]>('gh3036_execute_rpc', {
  commandKey: 'read_register',
  params: ['0x01'],
});
```

***

### gh3036\_subscribe\_events

订阅 GH3036 事件。

**后端命令**: `gh3036_subscribe_events`

**参数**: 无

**返回**: `boolean`

```typescript
const subscribed = await invoke<boolean>('gh3036_subscribe_events');
```

***

### gh3036\_get\_library\_status

获取库状态。

**后端命令**: `gh3036_get_library_status`

**参数**: 无

**返回**: `[boolean, boolean]` (库已加载, 库已初始化)

```typescript
const [loaded, initialized] = await invoke<[boolean, boolean]>('gh3036_get_library_status');
```

***

## 波形模块 (Waveform)

### waveform\_create\_buffer

创建波形数据缓冲区。

**后端命令**: `waveform_create_buffer`

**参数**:

| 参数名        | 类型                   | 必填 | 描述     |
| ---------- | -------------------- | -- | ------ |
| buffer\_id | string               | 是  | 缓冲区 ID |
| config     | WaveformBufferConfig | 是  | 缓冲区配置  |

**返回**: `void`

```typescript
await invoke('waveform_create_buffer', {
  bufferId: 'wave1',
  config: { maxRows: 1000, columnNames: ['x', 'y'] },
});
```

***

### waveform\_remove\_buffer

移除波形数据缓冲区。

**后端命令**: `waveform_remove_buffer`

**参数**:

| 参数名        | 类型     | 必填 | 描述     |
| ---------- | ------ | -- | ------ |
| buffer\_id | string | 是  | 缓冲区 ID |

**返回**: `void`

```typescript
await invoke('waveform_remove_buffer', { bufferId: 'wave1' });
```

***

### waveform\_configure\_parser

配置波形数据解析器。

**后端命令**: `waveform_configure_parser`

**参数**:

| 参数名        | 类型           | 必填 | 描述     |
| ---------- | ------------ | -- | ------ |
| buffer\_id | string       | 是  | 缓冲区 ID |
| config     | ParserConfig | 是  | 解析器配置  |

**返回**: `void`

```typescript
await invoke('waveform_configure_parser', {
  bufferId: 'wave1',
  config: { parserType: 'csv', delimiter: ',' },
});
```

***

### waveform\_parse\_and\_store

解析数据并存储到缓冲区。

**后端命令**: `waveform_parse_and_store`

**参数**:

| 参数名        | 类型     | 必填 | 描述        |
| ---------- | ------ | -- | --------- |
| buffer\_id | string | 是  | 缓冲区 ID    |
| data       | string | 是  | 要解析的数据字符串 |

**返回**: `void`

```typescript
await invoke('waveform_parse_and_store', { bufferId: 'wave1', data: '1.0,2.0,3.0' });
```

***

### waveform\_read\_data

读取波形数据。

**后端命令**: `waveform_read_data`

**参数**:

| 参数名        | 类型     | 必填 | 描述     |
| ---------- | ------ | -- | ------ |
| buffer\_id | string | 是  | 缓冲区 ID |
| rows       | number | 是  | 读取的行数  |

**返回**: `WaveformData`

```typescript
const data = await invoke<WaveformData>('waveform_read_data', {
  bufferId: 'wave1',
  rows: 100,
});

interface WaveformData {
  columns: string[];
  rows: number[][];
  timestamp: number;
}
```

***

### waveform\_get\_status

获取波形缓冲区状态。

**后端命令**: `waveform_get_status`

**参数**:

| 参数名        | 类型     | 必填 | 描述     |
| ---------- | ------ | -- | ------ |
| buffer\_id | string | 是  | 缓冲区 ID |

**返回**: `WaveformStatus`

```typescript
const status = await invoke<WaveformStatus>('waveform_get_status', { bufferId: 'wave1' });
```

***

### waveform\_clear\_buffer

清空波形缓冲区数据。

**后端命令**: `waveform_clear_buffer`

**参数**:

| 参数名        | 类型     | 必填 | 描述     |
| ---------- | ------ | -- | ------ |
| buffer\_id | string | 是  | 缓冲区 ID |

**返回**: `void`

```typescript
await invoke('waveform_clear_buffer', { bufferId: 'wave1' });
```

***

### waveform\_list\_buffers

列出所有波形缓冲区。

**后端命令**: `waveform_list_buffers`

**参数**: 无

**返回**: `string[]`

```typescript
const buffers = await invoke<string[]>('waveform_list_buffers');
```

***

## 状态模块 (State)

### dispatch\_action

分发一个动作到状态管理器。

**后端命令**: `dispatch_action`

**参数**:

| 参数名                 | 类型        | 必填 | 描述                                                            |
| ------------------- | --------- | -- | ------------------------------------------------------------- |
| action              | object    | 是  | 动作对象                                                          |
| action.action\_type | string    | 是  | 动作类型（CONNECT\_SERIAL/DISCONNECT\_SERIAL/SEND\_SERIAL\_DATA 等） |
| action.port\_name   | string    | 否  | 串口名称                                                          |
| action.config       | object    | 否  | 配置对象                                                          |
| action.data         | number\[] | 否  | 数据字节数组                                                        |
| action.device\_type | string    | 否  | 设备类型（serial/ble）                                              |
| action.device\_id   | string    | 否  | 设备 ID                                                         |
| action.save\_state  | boolean   | 否  | 是否保存状态                                                        |

**返回**: `StateResult`

```typescript
await invoke('dispatch_action', {
  action: {
    actionType: 'CONNECT_SERIAL',
    portName: 'COM1',
    config: { baudRate: '115200' },
    saveState: true
  }
});
```

***

### get\_state

获取当前应用状态。

**后端命令**: `get_state`

**参数**: 无

**返回**: `AppState`

```typescript
const state = await invoke<AppState>('get_state');
```

***

### get\_channel\_data

获取指定通道的数据。

**后端命令**: `get_channel_data`

**参数**:

| 参数名         | 类型     | 必填 | 描述    |
| ----------- | ------ | -- | ----- |
| channel\_id | string | 是  | 通道 ID |

**返回**: `ChannelData | null`

```typescript
const data = await invoke<ChannelData | null>('get_channel_data', { channelId: 'tx' });
```

***

### restore\_state

恢复应用状态。

**后端命令**: `restore_state`

**参数**:

| 参数名   | 类型       | 必填 | 描述       |
| ----- | -------- | -- | -------- |
| state | AppState | 是  | 要恢复的状态对象 |

**返回**: `void`

```typescript
await invoke('restore_state', { state: savedState });
```

***

### save\_state

保存当前应用状态。

**后端命令**: `save_state`

**参数**: 无

**返回**: `AppState`

```typescript
const savedState = await invoke<AppState>('save_state');
```

***

### get\_connected\_devices

获取已连接的设备列表。

**后端命令**: `get_connected_devices`

**参数**: 无

**返回**: `ConnectedDevice[]`

```typescript
const devices = await invoke<ConnectedDevice[]>('get_connected_devices');
```

***

### get\_window\_state

获取窗口状态。

**后端命令**: `get_window_state`

**参数**: 无

**返回**: `WindowState`

```typescript
const windowState = await invoke<WindowState>('get_window_state');
```

***

## 偏好设置模块 (Preferences)

### get\_preferences

获取所有偏好设置。

**后端命令**: `get_preferences`

**参数**: 无

**返回**: `Preferences`

```typescript
const prefs = await invoke<Preferences>('get_preferences');
```

***

### save\_preferences

保存偏好设置。

**后端命令**: `save_preferences`

**参数**:

| 参数名         | 类型     | 必填 | 描述     |
| ----------- | ------ | -- | ------ |
| preferences | object | 是  | 偏好设置对象 |

**返回**: `void`

```typescript
await invoke('save_preferences', { preferences: { theme: 'dark', language: 'zh-CN' } });
```

***

### update\_serial\_preferences

更新串口偏好设置。

**后端命令**: `update_serial_preferences`

**参数**:

| 参数名             | 类型      | 必填 | 描述      |
| --------------- | ------- | -- | ------- |
| display\_format | string  | 是  | 显示格式    |
| display\_mode   | string  | 是  | 显示模式    |
| send\_format    | string  | 是  | 发送格式    |
| append\_newline | boolean | 是  | 是否追加换行符 |
| newline\_type   | string  | 是  | 换行符类型   |
| auto\_scroll    | boolean | 是  | 是否自动滚动  |

**返回**: `void`

```typescript
await invoke('update_serial_preferences', {
  displayFormat: 'hex',
  displayMode: 'text',
  sendFormat: 'hex',
  appendNewline: true,
  newlineType: 'lf',
  autoScroll: true,
});
```

***

### update\_ble\_preferences

更新 BLE 偏好设置。

**后端命令**: `update_ble_preferences`

**参数**:

| 参数名               | 类型      | 必填 | 描述          |
| ----------------- | ------- | -- | ----------- |
| display\_format   | string  | 是  | 显示格式        |
| auto\_scroll      | boolean | 是  | 是否自动滚动      |
| input\_format     | string  | 是  | 输入格式        |
| without\_response | boolean | 是  | 是否无响应写入     |
| config\_collapsed | boolean | 是  | 配置面板是否折叠    |
| gatt\_collapsed   | boolean | 是  | GATT 面板是否折叠 |
| panel\_collapsed  | boolean | 是  | 侧边面板是否折叠    |

**返回**: `void`

```typescript
await invoke('update_ble_preferences', {
  displayFormat: 'hex',
  autoScroll: true,
  inputFormat: 'text',
  withoutResponse: false,
  configCollapsed: false,
  gattCollapsed: false,
  panelCollapsed: false,
});
```

***

## 事件汇总

### serial:data

串口接收数据事件。

```typescript
interface SerialDataPayload {
  device_id: string;
  data: number[];
  timestamp: number;
}

import { listen } from '@tauri-apps/api/event';
const unlisten = await listen<SerialDataPayload>('serial:data', (event) => {
  console.log('串口数据:', event.payload.device_id, event.payload.data);
});
```

***

### ble:data

BLE 数据通知事件。

```typescript
interface BleDataPayload {
  device_id: string;
  address: string;
  characteristic_uuid: string;
  data: number[];
  timestamp: number;
}

import { listen } from '@tauri-apps/api/event';
const unlisten = await listen<BleDataPayload>('ble:data', (event) => {
  console.log('BLE数据:', event.payload.device_id, event.payload.characteristic_uuid);
});
```

***

## 类型定义汇总

### 串口类型

```typescript
interface PortInfo {
  name: string;
  port_type: string;
  manufacturer?: string;
  product?: string;
  serial_number?: string;
}

interface SerialPortConfigDto {
  portName: string;
  baudRate?: string;
  dataBits?: number;
  parity?: string;
  stopBits?: number;
  flowControl?: string;
  timeoutMs?: number;
  packTimeoutMs?: number;
}

interface ExportResult {
  log_path: string;
  dat_path: string;
}

interface ExportDataEntry {
  timestamp: number;
  data: number[];
  direction: string;
}
```

### BLE 类型

```typescript
interface BleDevice {
  address: string;
  name?: string;
  rssi?: number;
  is_connectable: boolean;
  services?: string[];
  manufacturer_data?: Record<string, number[]>;
}

interface BleConnection {
  device_id: string;
  address: string;
  name?: string;
  is_connected: boolean;
  services: BleService[];
  connected_at?: number;
  mtu?: number;
}

interface BleService {
  uuid: string;
  primary: boolean;
  characteristics: BleCharacteristic[];
}

interface BleCharacteristic {
  uuid: string;
  properties: BleCharacteristicProperties;
  value?: number[];
}

interface BleCharacteristicProperties {
  read: boolean;
  write: boolean;
  notify: boolean;
}

interface AtConfig {
  port_name: string;
  baud_rate: number;
  timeout_ms: number;
  tx_uuid?: string;
  rx_uuid?: string;
  srv_uuid?: string;
}

interface BleDataEvent {
  deviceId: string;
  characteristicUuid: string;
  data: number[];
  timestamp: number;
}
```

### 协议类型

```typescript
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

### 系统类型

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

interface RuntimeStatus {
  active_connections: number;
  serial_ports_open: number;
  ble_connections: number;
  protocols_loaded: number;
  uptime_secs: number;
}

interface LogConfig {
  level: string;
  max_files: number;
  max_size_mb: number;
  console_enabled: boolean;
  file_enabled: boolean;
}

interface WindowStatus {
  label: string;
  title: string;
  visible: boolean;
  focused: boolean;
  maximized: boolean;
  minimized: boolean;
  fullscreen: boolean;
  width: number;
  height: number;
  x: number;
  y: number;
}
```

### GH3036 类型

```typescript
interface ChannelConfig {
  channel_type: string;
  device_id: string;
  characteristic_uuid?: string;
}

interface CsvConfig {
  enabled: boolean;
  output_dir: string;
}
```

### 波形类型

```typescript
interface WaveformBufferConfig {
  max_rows: number;
  column_names: string[];
}

interface ParserConfig {
  parser_type: string;
  delimiter?: string;
}

interface WaveformData {
  columns: string[];
  rows: number[][];
  timestamp: number;
}

interface WaveformStatus {
  buffer_id: string;
  row_count: number;
  max_rows: number;
  parser_type?: string;
}
```

***

## 命名规范

本文档遵循以下命名规范：

| 类型               | 规范             | 示例                              |
| ---------------- | -------------- | ------------------------------- |
| Rust 函数/变量       | snake\_case    | `open_serial_port`, `port_name` |
| Rust 结构体/特征      | UpperCamelCase | `SerialManager`, `BleBackend`   |
| TypeScript 变量/函数 | camelCase      | `deviceId`, `scanDevices`       |
| Tauri invoke 参数  | camelCase      | `{ portName: 'COM1' }`          |
| 数据库/配置           | snake\_case    | `serial_number`, `baud_rate`    |

***

## 更新日志

- **2026-04-13**: 全面重写 API 文档，新增 Dashboard、GH3036、Waveform、State、Preferences 模块；修复 BLE 事件类型命名（BleNotificationEvent → BleDataEvent，char\_uuid → characteristicUuid）；补充串口 export\_serial\_data 命令；补充系统模块 get\_window\_status、show\_main\_window 等命令
- **2026-04-02**: 统一使用蛇形命名规范，修复 BLE 和 Serial 模块所有参数不匹配问题

