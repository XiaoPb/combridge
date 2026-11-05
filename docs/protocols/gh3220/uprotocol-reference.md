# GH3220 UProtocol 协议规范

> 本文只描述 **UProtocol**（`demo_kernel_code/module/gh_protocol/`），即当前固件实际启用、
> 已进入 Zephyr 构建的那套主机通信协议。
>
> SDK 里另有一套 **GHRPC**（`gh_protocol/`，字符串 Key 寻址、未接入构建），
> 两者帧头同为 `AA 11` 但**语义完全不同、互不兼容**，详见
> [`gh3220_ghrpc_reference.md`](gh3220_ghrpc_reference.md)。
> 两套协议**不能同时挂在同一条字节流上**，构建期必须二选一。
>
> 本文只讲"协议长什么样、怎么跑"；协议与驱动的耦合、全局状态、并发模型见
> [`gh3220_uprotocol_integration.md`](gh3220_uprotocol_integration.md)。

- 代码版本：`gh3220_sdk` 工作区（`af51a39` 之后）
- 行号相对仓库根目录

---

## 目录

1. [范围与边界](#1-范围与边界)
2. [帧层](#2-帧层)
3. [接收状态机](#3-接收状态机)
4. [命令集](#4-命令集)
5. [数据上报帧 0x0B](#5-数据上报帧-0x0b)
6. [数据压缩（4bit 差分）](#6-数据压缩4bit-差分)
7. [事件通道 0x16 与 ACK](#7-事件通道-0x16-与-ack)
8. [发送队列与定时器](#8-发送队列与定时器)
9. [与 SDK 的接口契约](#9-与-sdk-的接口契约)
10. [已知问题](#10-已知问题)
11. [附录](#11-附录)

---

## 1. 范围与边界

### 1.1 代码位置

| 文件 | 职责 |
|---|---|
| `demo_kernel_code/module/gh_protocol/gh_uprotocol.c` | 帧编解码、接收状态机、命令分发、非压缩上报 |
| `demo_kernel_code/module/gh_protocol/gh_uprotocol.h` | 帧常量、命令号、payload 偏移宏 |
| `demo_kernel_code/module/gh_protocol/gh_zip.c` | 4bit 差分压缩上报 |
| `demo_kernel_code/module/gh_protocol/gh_zip.h` | 压缩器状态结构 |
| `demo_kernel_code/kernel/gh_demo_protocol.c` | 传输层：凑帧、发送 FIFO、事件 FIFO、10ms 定时器 |
| `demo_kernel_code/driver/inc/gh_drv.h` | `EMUprotocolParseCmdType`、`STGh3x2xProtocolData`、容量宏 |

两文件均被 `zephyr/CMakeLists.txt` 的 `GH3220_MODULE_SOURCES` 引用，
由 `CONFIG_GH3220_SUPPORT_PROTOCOL_ANALYZE`（默认 y）启用。

### 1.2 设计定位

UProtocol 是"寄存器级 + 数据流"的轻量协议：

- 命令用 **1 字节 CMD** 区分，payload 布局按命令固定；
- 只有**事件**通道带 ACK/重传，命令与数据都是"发出去就不管"；
- 数据上报用 4bit 半字节差分压缩，面向实时波形流；
- 全部状态都是单例全局变量，假定单线程访问（由集成层保证）。

它是 EVK/产测工具（Goodix 上位机）使用的格式，也是当前产品的默认协议。

### 1.3 分层

```
┌──────────────────────── 应用/集成层 ────────────────────────┐
│ 字节流进 → 凑帧 → 命令执行 → 组响应 → 入发送队列             │
│ 采样帧 → 压缩/非压缩组包 → 入发送队列                        │
└───────────────┬──────────────────────────────▲──────────────┘
                │                              │
┌───────────────▼──────────────────────────────┴──────────────┐
│ 传输层  kernel/gh_demo_protocol.c                            │
│   data_fifo[64][255] / event_fifo[16][4] / 10ms 定时器        │
└───────────────┬──────────────────────────────▲──────────────┘
                │                              │
┌───────────────▼──────────────────────────────┴──────────────┐
│ 语义层  module/gh_protocol/gh_uprotocol.c + gh_zip.c         │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 帧层

### 2.1 帧格式

实现：`gh_uprotocol.c:289` `GH3X2X_UprotocolPacketFormat()`

```
偏移  0     1     2     3     4 ................ 4+LEN-1  4+LEN
    0xAA  0x11  CMD   LEN        PAYLOAD                    CRC8
     └──── 固定头 ────┘  └─ 长度 ─┘
```

| 字段 | 长度 | 说明 |
|---|---|---|
| `0xAA` | 1 | 固定头 `GH3X2X_UPROTOCOL_FIXED_HEADER` |
| `0x11` | 1 | 版本字节；协议版本字符串 `Health_Uprotocol_v0.4` |
| `CMD` | 1 | 命令号，见第 4 章 |
| `LEN` | 1 | **仅 payload 长度**，0 合法 |
| `PAYLOAD` | LEN | 命令/数据载荷 |
| `CRC8` | 1 | 覆盖 `AA,11,CMD,LEN,PAYLOAD`，不含自身 |

总帧长 = `LEN + 5`。

### 2.2 CRC8

实现：`gh_uprotocol.c:240` `GH3X2X_CalcByteCrc8Val()`、`:261` `GH3X2X_CalcArrayCrc8Val()`

- 多项式 `0x07`（MSB-first，无反射），**初值 0xFF**，无输出异或
- 表驱动：`crc = tbl[(crc ^ byte) & 0xFF]`，表定义在 `gh_uprotocol.c:105`

```c
uint8_t crc8(const uint8_t *p, size_t n) {
    uint8_t c = 0xFF;
    while (n--) c = tbl[c ^ *p++];
    return c;
}
/* 组包校验范围：p[0] .. p[3+len] 共 len+4 字节 */
```

自测向量：

| 报文 | 字节 | CRC8 |
|---|---|---|
| GET_VER，空 payload | `AA 11 19 00` | `91` |
| GET_VER，type=0x01 | `AA 11 19 01 01` | `EC` |
| GET_MAX_LEN | `AA 11 A0 00` | `63` |
| CHIP_CTRL wakeup | `AA 11 17 01 C3` | `80` |
| 事件 ACK，id=2 | `AA 11 16 01 02` | `A2` |

### 2.3 容量上限

```c
#define GH3X2X_UPROTOCOL_PACKET_LEN_MAX (243)                 // 整帧上限
#define GH3X2X_UPROTOCOL_PACKET_HEADER_LEN (4)
#define GH3X2X_UPROTOCOL_PACKET_TAIL_LEN (1)
#define GH3X2X_UPROTOCOL_PAYLOAD_LEN_MAX (243 - 4 - 1) = 238  // "payload 上限"
#define GH3X2X_UPROTOCOL_GET_PAYLOAD_LEN_SUPPORT(v) ((v) - 5)
```

初始化时 `GH3X2X_UprotocolPacketMaxLenConfig(GH3X2X_UPROTOCOL_PAYLOAD_LEN_MAX)`
（`gh_demo.c:1629`）把 238 当作"整帧最大长度"保存到 `g_uchPacketMaxLenSupport`
（函数会把入参钳位到 `[20, 243]`），此后业务代码用
`GET_PAYLOAD_LEN_SUPPORT(238) = 233` 作为**单包 payload 上限**。

所以实际约束：

| 量 | 值 |
|---|---|
| 协议理论整帧上限 | 243 |
| 运行时整帧上限 | 238 |
| **单包 payload 上限** | **233** |
| 命令响应缓冲 | 238（`puchRespondBuffer[]`） |

### 2.4 组包

```c
uint8_t GH3X2X_UprotocolPacketFormat(uint8_t cmd, uint8_t *out,
                                     uint8_t *payload, uint8_t len);
```

1. `real_len = min(GET_PAYLOAD_LEN_SUPPORT(g_uchPacketMaxLenSupport), len)` —— 超长**静默截断**
2. 写 `AA 11 CMD LEN`
3. 拷 payload
4. 写 CRC8，返回总长（`real_len + 5`）

---

## 3. 接收状态机

实现：`gh_uprotocol.c:333` `GH3X2X_UprotocolParseBuffer()`，状态变量 `g_emPacketParseStatus`。

### 3.1 状态定义

```c
typedef enum {
    UPROTOCOL_STATUS_IDLE = 0,      // 等帧头 0xAA
    UPROTOCOL_STATUS_GOT_FIXED,     // 已收 0xAA，等 0x11
    UPROTOCOL_STATUS_GOT_VER,       // 已收 0x11，等 CMD
    UPROTOCOL_STATUS_GOT_CMD,       // 已收 CMD，等 LEN
    UPROTOCOL_STATUS_GOT_LEN,       // 收 payload 中
    UPROTOCOL_STATUS_GOT_PAYLOAD,   // 收 CRC 字节
    UPROTOCOL_STATUS_GOT_CRC8,      // 保留，永不进入
} EMUprotocolParseStatus;
```

### 3.2 状态转移表

逐字节推进，`crc` 为运行中的 CRC8，`idx` 为 payload 写指针。

| 当前状态 | 输入字节 | 条件 | 动作 | 下一状态 |
|---|---|---|---|---|
| IDLE | `b` | `b == 0xAA` | `crc = crc1(0xFF, b)` | GOT_FIXED |
| IDLE | `b` | 其它 | 丢弃 | IDLE |
| GOT_FIXED | `b` | `b == 0x11` | `crc = crc1(crc, b)` | GOT_VER |
| GOT_FIXED | `b` | 其它 | `crc = 0xFF` | IDLE（本字节不重试，`AA AA 11` 会丢同步） |
| GOT_VER | `b` | 任意 | `cmd = b`；`crc` 更新 | GOT_CMD |
| GOT_CMD | `b` | `b == 0` | `len = 0`；`crc` 更新 | GOT_PAYLOAD（直接验 CRC） |
| GOT_CMD | `b` | `1 <= b <= 238` | `len = b`, `idx = 0`；`crc` 更新 | GOT_LEN |
| GOT_CMD | `b` | `b > 238` | `crc = 0xFF` | IDLE |
| GOT_LEN | `b` | — | `payload[idx++] = b`；`crc` 更新 | `idx >= len` ? GOT_PAYLOAD : GOT_LEN |
| GOT_PAYLOAD | `b` | `b == crc` | 返回 `RET_OK`（本帧完成） | IDLE，`crc = 0xFF` |
| GOT_PAYLOAD | `b` | `b != crc` | 静默丢弃 | IDLE，`crc = 0xFF` |

> CRC 错误**不会**产生回包或错误码，只能靠主机超时重试。

### 3.3 多命令拼接

`GH3X2X_UprotocolParseHandler()`（`gh_uprotocol.c:1308`）在一个接收缓冲里循环：

```c
while (usParseIndex < usRecvLen) {
    if (ParseBuffer(&has_parse, buf + usParseIndex, usRecvLen - usParseIndex) == RET_OK) {
        cmd_type = CmdHandler(&respond[usRespondIndex], &cmd_resp_len);
        usRespondIndex += cmd_resp_len;      // 多条命令的响应首尾相接
    }
    usParseIndex += has_parse;               // 已消费字节数（含 CRC）
}
*pusRespondLen = usRespondIndex;
```

- 一次 `ParseBuffer` 只解析**一条**完整命令
- 多条命令的响应拼进同一块 `puchRespondBuffer[]`（容量 238）
- 若无有效命令，`puchRespondBuffer[3]` 保持 0 且返回 `UPROTOCOL_CMD_IGNORE`，
  上层据此跳过发送（`gh_demo_protocol.c:413`）

### 3.4 上层凑帧状态机

`Gh3x2xDemoHandleRecvBleData()`（`gh_demo_protocol.c:507`）逐字节凑帧，**不做 CRC**：

```
WAIT_HEADER(0xAA) → WAIT_VERSION(0x11) → WAIT_CMD
   → WAIT_LEN(0 或 >238 复位) → WAIT_DATA(收满 LEN+4) → WAIT_CRC8(收完交给 ParseHandler)
```

它只是缓冲拼装；语法与校验在 `ParseBuffer` 里。应用层还有一层等价实现
`gh3220_frame_feed()`（`l-evk-t2-gh3220/app_cardiff_a/src/gh3220_frame.c:8`），
在进入 SDK 前就把整帧切好，二者行为一致（因此实际会走两遍凑帧，见第 10.6 节）。

UART 侧另有 `0x47 0x44 LEN PAYLOAD 0x0A` 封装分支（`gh_demo_protocol.c:447`），
仅在 `GOODIX_DEMO_PLATFORM` + UART 模式启用；本仓库 Zephyr 移植未定义该宏，BLE 直通。

---

## 4. 命令集

分发实现：`GH3X2X_UprotocolCmdHandler()`（`gh_uprotocol.c:1208`）。

### 4.1 总表

| CMD | 名称 | 方向 | 请求 payload | 响应 payload | 处理函数 |
|---|---|---|---|---|---|
| 0x03 | REG_RW | 双向 | `mode,cnt,addrH,addrL,data...` | 写：原样回显；读：`mode,cnt,addrH,addrL,data...` | `:442` |
| 0x05 | PACKAGE_TEST | 双向 | `n, ...` | 回显 `n` 字节 | `:1041` |
| 0x0B | NEW_STRUCT_RAWDATA | 设备→主机 | 8B 包头 + 帧数据 | — | `gh_uprotocol.c:1453` / `gh_zip.c:290` |
| 0x0C | START_CTRL | 主机→设备 | `start,mode,slotEn,func[4]` | `status(1)` | `:927` |
| 0x10 | WORK_MODE | 主机→设备 | `workMode,funcMask[4]` | `status(1)` | `:977` |
| 0x12 | FIFO_THR_CONFIG | 主机→设备 | `thrH,thrL` | `status(1)` | `:655` |
| 0x13 | REPORT_EVENT_SET | 主机→设备 | `eventMask[2]` | `status(1)` | `:1016` |
| 0x14 | ECG_CLASSIFICATION | 主机→设备 | — | `class(1)` | `:1064` |
| 0x16 | CHIP_EVENT_REPORT | 双向 | ACK：`reportId(1)` | 事件：`event[2],id(1)` | `:1151` / `:1922` |
| 0x17 | CHIP_CTRL | 主机→设备 | `ctrlType(1)` | `status(1)` | `:689` |
| 0x19 | GET_VER | 主机→设备 | `verType(1)`（可空） | `type,len,string` | `:615` |
| 0x1A | GET_CHIPSET_CONN_STA | 主机→设备 | — | `status(1)` | `:1177` |
| 0x1C | SWITCH_SLOT_FUNC | 主机→设备 | `slot,extra,func[4],onoff` | `status(1)` | `:784` |
| 0x25 | GET_OPENED_FUNC | 主机→设备 | — | `maxLen,funcMask[4]` | `:598` |
| 0x26 | SET_ECG_CASCADE | 主机→设备 | `op,val` | **无响应** | `:1079` |
| 0x2B | BP_DATA_TRANS | 主机→设备 | 透传 | `status(1)` | `:1127` |
| 0x2C | FUNCTION_INFO_UPDATE | 设备→主机 | — | 功能信息 | `:1373` |
| 0xA0 | GET_MAX_LEN | 主机→设备 | — | `maxLen(1)` | `:762` |
| 0xA1 | LOAD_REG_LIST | 主机→设备 | `(addr,val) × n`，4B/条 | `status(1)` | `:834` |
| 0xA2 | DEBUG_STATUS | 主机→设备 | `type(1)` | `type,bytesLen,data[4]` | `:869` |

未列出的 CMD 走 default 分支，回一个 payload 长度为 0 的响应
（`AA 11 CMD 00 CRC`），可用于探测命令是否被实现。

### 4.2 关键命令详解

#### 0x03 REG_RW（寄存器读写）

```
请求: [0]=mode  [1]=cnt  [2]=addrH  [3]=addrL  [4..]=data（每寄存器 2 字节，高字节在前）
      mode 0 = 读，1 = 写
写响应: 原样回显整个请求 payload
读响应: [0..3] 回显，[4..] = cnt 个寄存器的值（每个 2 字节，高字节在前）
```

- 写：`cnt*2 == payload_len-4` 才执行；地址 0x0000 段访问真实寄存器，其余为虚拟寄存器
- 读：`cnt` 被钳位到 `(233-4)/2 = 114`
- 特殊地址 `0xAAAA`：直接从 FIFO 读 `cnt*2` 字节（`gh_uprotocol.c:526`）
- 写虚拟寄存器会**改变上行报文结构**（GS 使能、FIFO package 模式等），主机需重新解析包头

#### 0x19 GET_VER（版本查询）

```
请求: [0]=verType（payload 为 0 时默认 0x01 固件版本）
响应: [0]=verType  [1]=strLen  [2..]=字符串（无 '\0'，超长按 233-2 截断）
```

| 值 | 含义 | 值 | 含义 |
|---|---|---|---|
| 0x01 | 固件版本 | 0x0B | 虚拟寄存器表版本 |
| 0x0A | Demo 版本 | 0x0C | Bootloader 版本 |
| 0x0E | 协议版本 | 0x0F | 驱动功能支持位 |
| 0x10 | 驱动库版本 | 0x11 | 芯片版本（`vXX_evYY`） |
| 0x12 | 算法版本起点 | 0x12+N | 第 N 号功能对应算法版本（`N < GH3X2X_FUNC_OFFSET_MAX`） |

#### 0x0C START_CTRL / 0x1C SWITCH_SLOT_FUNC（启停）

```c
func_mask = LE32(payload[func_index .. +3]);          // 请求小端
if ((g_unAllFuncMode | func_mask) == g_unAllFuncMode) // 只允许已登记的功能位
    status = 0，按 onoff 更新 g_unFuncModeOpened，返回 CMD_START / CMD_STOP
else
    status = 1
```

- `START_CTRL` 的 onoff 在 `[0]`（0 启动 / 1 停止），`SWITCH_SLOT_FUNC` 在 `[6]`
- **必须先发 0x10 WORK_MODE 声明允许的功能位**，否则 `g_unAllFuncMode == 0`，永远返回失败
  （详见集成文档 3.4）

#### 0x10 WORK_MODE

```
请求: [0]=workMode  [1..4]=允许的功能位掩码（小端）
响应: [0]=0（永远成功）
返回值: cmd_type = workMode + 2
```

映射：0=EVK→2、1=APP→3、2=MCU在线→4、3=MCU离线→5、4=测试调优→6、5=透传→7、
6=读取当前→8、7=MPT→9。

副作用：写 `g_unAllFuncMode`，并把 `g_unFuncModeOpened` / `g_unTargetFuncMode` 清零
（**不会停止正在运行的功能**）；MPT 模式下 IRQ 处理会直接返回，不采样。

#### 0x17 CHIP_CTRL

| 值 | 动作 |
|---|---|
| 0x5A | 硬复位 `GH3X2X_HardReset()` |
| 0x5B | RX 复位（仅 `gh_protocol_cmd.h` 定义，UProtocol 未实现） |
| 0xC2 | 软复位 `GH3X2X_SoftReset()` |
| 0xC3 | 退出低功耗 |
| 0xC4 | 进入低功耗 |

status：`0` 成功、`1` 失败/不支持、`0xFF` 功能指针为空。
复位类命令还会置 `g_uchResetFromProtocolFlag = 1` 并复位软 AGC 状态。

#### 0xA1 LOAD_REG_LIST（批量配置下发）

```
请求: [addr(2), val(2)] × n     // 先按小端解析，再 SwapLittleBigEndianData
响应: [0] = 0 成功 / 1 失败
```

#### 0xA2 DEBUG_STATUS

```
请求: [0]=type
响应: [0]=type [1]=单值字节数（当前恒 1） [2..5]=data（固定 6 字节）
```

| type | 含义 |
|---|---|
| 0x00 / 0x01 / 0x02 / 0x03 | 读 INT / RST / SPCS / SPDO 引脚 |
| 0x04 | 通信确认（1 表示成功） |
| 0x10 及其它 | 读工作状态 `g_uchGh3x2xStatus` |

#### 0x16 CHIP_EVENT_REPORT（事件 / ACK 复用）

- 设备→主机：`[0..1]=IRQ 状态（高字节在前）`，`[2]=reportId`
- 主机→设备：`[0]=reportId`，匹配 FIFO 队首 id 则置 ACK

详见第 7 章。

### 4.3 已定义但未实现

`gh_uprotocol.h` 中有宏、但 `switch` 无分支（落入 default 回空包）：
0x00 NOP、0x01 CTRL_ACK、0x02 GET_STATUS、0x04 LOAD_CONFIG、0x07 READ_OTP_REG、
0x08 RAWDATA、0x09 ZIP_EVEN_RAWDATA、0x0A ZIP_ODD_RAWDATA、0x15 FUNC_MAP、
0x29 HSM_RESULT_UPDATE、0x2A RAWDATA_FIFO_UPDATE。

> 0x09/0x0A 虽定义"偶数/奇数压缩原始数据"命令，当前实现**不使用**——压缩标志放在
> 0x0B 包头里。0x08 是旧的非压缩通道，同样被 0x0B 取代。

---

## 5. 数据上报帧 0x0B

两种实现共用外层命令与 8 字节包头，只有"帧数据"部分不同：

- 非压缩：`Gh2x2xUploadDataToMaster()`（`gh_uprotocol.c:1453`）
- 压缩：`Gh2x2xUploadZipDataToMaster()`（`gh_zip.c:290`）

由 `__SUPPORT_ZIP_PROTOCOL__`（Kconfig `GH3220_SUPPORT_ZIP_PROTOCOL`，默认 y）二选一。

### 5.1 8 字节包头

`Gh2x2xPackPakcageHeader()`（`gh_uprotocol.c:1403`）：

```
[0]    function_id    功能号（unFunctionID 最高置位对应的序号）
[1]    data_type      位域
[2..5] channel_mask   本次上传涉及的通道位图（大端，按 bit 计通道数）
[6]    package_type   位域
[7]    data_len       后续帧数据字节数
```

`data_type`：

| bit | 含义 |
|---|---|
| 0 | G-Sensor 数据 |
| 1 | 算法结果 |
| 2 | AGC 信息 |
| 3 | 环境光（当前恒 0） |
| 4 | 陀螺仪（需同时使能 G-Sensor） |
| 5 | 电容（CAP） |
| 6 | 温度（TEMP） |

`package_type`：

| bit | 含义 |
|---|---|
| 0 | **zip 压缩标志** |
| 1 | **奇偶翻转标志**（1 = 本包首帧为绝对帧） |
| 2..4 | FIFO package 模式（1 时 rawdata 段只发 1 字节 package id） |
| 3..4 | 拼接包序号（仅拼接模式，覆盖上面的模式位） |
| 5 | 拼接包结束标志 |

### 5.2 非压缩帧数据布局

数值除特别说明外均为**大端**：

```
frame_id              1B     帧计数低 8 位
g-sensor              3×int16（acc），陀螺使能时再 3×int16（gyro）
CAP                   4×3B（24bit）
TEMP                  4×3B
rawdata               chnl × 4B（bit26=LED 调节0/快速恢复，bit25=LED 调节1，bit23..0 数值）
                      或 FIFO 模式下的 1B package_id
agc                   chnl × 4B（原样 memcpy，含 gain/drv/dc-cancel）
ambience              chnl × 3B（当前不写）
result block          [len][sub-block ...]
```

写完帧后 `punFrameRawdata[i] &= 0x00FFFFFF` 清掉高 8 位 tag，保证下一帧差分基于 24 位有效值。

### 5.3 结果块（TLV）

```
[len]           1B  本块除自身外字节数
[type][4B] × n  子块
```

| type | 内容 |
|---|---|
| 2 | `punFrameFlag[2]` |
| 3 | `punFrameFlag[3]`，**仅一帧的第一个包**发送 |
| 4 | `punFrameFlag[4]`（除零标志，宏开启时） |
| `0x80 \| i` | 第 i 号算法结果，int32 **小端**；只发 `usResultBit` 置位项，最多 `uchResultNum` 个 |

`len` 在整块写完后回填：`buf[use - resultLen] = resultLen - 1`。

### 5.4 压缩帧数据布局

```
frame_id              1B
g-sensor / CAP / TEMP 同非压缩（不压缩）
rawdata 块            [N][marker][tag × chnl][nibble 流]
agc 块                [N][nibble 流]
result block          与非压缩相同，但
                      · 非 ECG 功能额外固定发一个 type=0（软 AGC 标志）子块
                      · type=2 仅在 flag2 变化或帧号为 0 时发送
```

### 5.5 上传状态机（UPLOAD_STATUS + 奇偶翻转）

| 变量 | 取值 | 含义 |
|---|---|---|
| `g_uchGh3x2xUploadStatus` | `NULL(0)` | 无待发缓冲，下帧到来时重建包头 |
| | `HAVE_STORE_DATA(1)` | 包头已建好，正在追加帧 |
| `g_uchOddEvenChangeFlag` | 0/1 | 每次发包翻转；为 1 时本包首帧写绝对值 |
| `g_uchGh3x2xUploadBufUse` | — | 当前包已用字节数 |

```
              ┌── status == NULL ? ──是──► 重建 8B 包头，status = HAVE_STORE_DATA
              │
              ▼
    首帧？(oddEven==1 且 bufUse<=8) ──是──► rawdata/agc 写绝对值并记录 last
              │
              └──否──► 4bit 差分编码
                                 │
                                 ▼
        bufUse + frameLen >= 233 ? ──是──► 发送当前包，status=NULL，奇偶翻转，重来
                                 │否
                                 ▼
                    追加到 payload；本组最后一帧？ ──是──► 发送 + 状态复位
                                 │否
                                 ▼
                              结束
```

- **奇偶标志表示"本包首帧是否为绝对帧"，不是"每包必绝对"**：标志为 1 时首帧写绝对值；为 0 时首帧继续用上一次历史做差分。历史**跨包保留**，只有绝对帧会重新播种（实测：连续包呈 绝对/差分 交替）
- `usFrameCnt == 0` 记 HEAD（额外发 flag3），`== usFrameNum-1` 记 TAIL（立即 flush）
- 单帧装不下且 `bufUse <= 8` 时进入**拼接包模式**

### 5.6 拼接包（splice pack）

```
usMaxPackLen = 233 - bufUse
usPackNum    = frameLen / usMaxPackLen
依次发送 usPackNum 个满包（package_type bit3..4 写序号 1,2,...）
最后发送剩余字节（序号 +1，并置 bit5 表示结束）
```

每个分片是独立 0x0B 包，`data_len` 各自正确；只有最后一片置 bit5。

---

## 6. 数据压缩（4bit 差分）

实现：`gh_zip.c:60` `GH3X2X_GetRawDataDiff()`（带 tag 块）、`:167` `GH3X2X_GetDataDiff()`
（不带 tag 块，用于 AGC）。

### 6.1 数据模型

```
bit31..24  tag（LED 调节 / 快速恢复标志）
bit23..0   24 位有效 rawdata
```

差分只针对低 24 位；tag 变化时单独整块发送。

### 6.2 块结构

```
byte0        N        后续字节数（不含自身）
byte1        marker   1: 紧跟 chnl 个 tag 字节；0: 无 tag 块
byte2..      tag[]    marker==1 时存在，长度 = 通道数
byte...      nibble 流  4bit 打包，高半字节在前
```

总长 = `N + 1`，其中

```
N = 1 + (marker ? chnl : 0) + ceil(nibbles / 2)
```

> `puchZipDataArr[0]` 是长度字节，`*puchZipDataArrSize = N + 1`（size 含长度字节本身）。
> AGC 块没有 marker 却沿用同一公式，所以**声明长度比实际多 1 字节**（见 10.4）。

### 6.3 通道编码（type + 幅值）

```c
diff = |cur - last|;                      // 24 位无符号
dir  = (cur > last) ? 0 : 1;              // 0 = 上升，1 = 下降（相等也归入 1）
k    = diff 的最高非零 4bit 组序号;        // 0..8，diff==0 时 k=0
type = 2*k + dir;                         // 0..17
```

1. 写 `type` 半字节
2. 写 `k+1` 个半字节：`diff >> (k*4)`，从高到低（MSB first）

| type | 含义 |
|---|---|
| 0 / 1 | 1 个半字节（0..15），dir 0/1 |
| 2 / 3 | 2 个半字节（0..255） |
| 4 / 5 | 3 个半字节（0..4095） |
| ... | ... |
| 16 / 17 | 9 个半字节（0..2^36-1，实际受 24 位限制） |

### 6.4 半字节对齐

```c
if (cnt4 % 2)  buf[cnt8]      |= nibble;          // 低半字节
else         { cnt8++; buf[cnt8] |= nibble << 4; } // 新字节高半字节
cnt4++;
```

第一个半字节写在字节高 4 位，第二个写低 4 位。tag 块按整字节写，但会把 `cnt4` 加 `2*chnl`，
从而保持后续半字节流相位对齐。

### 6.5 手工示例

2 通道，tag 无变化，`last = {1000, 2000}`，`cur = {1005, 1990}`：

```
ch0: diff=5,  dir=0, k=0 → type=0，半字节 0x5
ch1: diff=10, dir=1, k=0 → type=1，半字节 0xA

nibble 流: [0][5][1][A]
byte1 = 0x05   (高=type0, 低=5)
byte2 = 0x1A   (高=type1, 低=A)
N = 1(marker) + ceil(4/2) = 3
块: 03 00 05 1A
```

### 6.6 解码算法

```c
/* buf 指向 zip 块的首字节（长度字节），chnl 由包头 channel_mask 统计 */
const uint8_t *p = buf;
int n      = p[0];
int marker = p[1];
int pos    = 2;
uint8_t tags[MAX_CH];

if (marker) { memcpy(tags, p + pos, chnl); pos += chnl; }

int nib_idx = (pos - 1) * 2;    // 相对 p[1] 的半字节序号

#define READ_NIB()  ( (nib_idx & 1) ? (p[1 + nib_idx++ / 2] & 0x0F) \
                                    : (p[1 + nib_idx++ / 2] >> 4) )

for (int ch = 0; ch < chnl; ch++) {
    uint8_t type = READ_NIB();
    int dir      = type & 1;            // 0: 上升, 1: 下降
    int groups   = (type >> 1) + 1;
    uint32_t mag = 0;
    for (int i = 0; i < groups; i++) mag = (mag << 4) | READ_NIB();
    cur[ch]  = dir ? (last[ch] - mag) : (last[ch] + mag);
    last[ch] = cur[ch];
}
/* 使用前应校验 nib_idx 不越过 n+1 字节，并检查 n 与实际消费量一致 */
```

游标起点 `(2 + chnl - 1) * 2`：tag 块占 `chnl` 个整字节（`2*chnl` 个半字节）。

### 6.7 边界与约束

- `diff` 用 `uint32` 计算，真实数据只有 24 位，实际最多 9 个半字节
- `cur == last` 时 `dir = 1`（diff=0）
- 通道数来自功能配置，最大 `CHANNEL_MAP_ID_NUM = 32`
- **`channel_mask` 是位图不是连续位段**：真机 HR 4 通道时实测为 `0x00000033`
  （bit0/1/4/5），通道数应取 `popcount`；按"最高位+1"会算成 6 并导致整包解析失败
- `puchZipDataArrSize` 与 `N` 都是 `GU8`，通道数 > 约 50 时长度字段会回绕

---

## 7. 事件通道 0x16 与 ACK

实现：`gh_demo_protocol.c:172/182/225`、`gh_uprotocol.c:1151/1922`。

### 7.1 事件队列

```c
GU8  g_puchGh3x2xProtocolEventSendBuf[16][4];  // [event_h, event_l, report_id, event_ex]
GU8  g_puchGh3x2xProtocolEventSendFifoWp, g_uchGh3x2xProtocolEventSendFifoRp;
GU8  g_uchGh3x2xProtocolEventAckStatus;        // 0 未确认 / 1 已确认
GU8  g_uchGh3x2xProtocolEventReportRetryCnt;
GU16 g_usGh3x2xProtocolEventWaitingAckTime;
```

上报入口：`Gh3x2xDemoReportEvent(usEvent, uchEventEx)`
→ 过滤 → `Gh3x2x_HalSerialWriteEventToFifo()`。

### 7.2 发送/确认状态机（每 10ms 执行一次）

```c
/* 第一步：队首状态（出队 / 超时 / 超限丢弃） */
if (wp != rp) {
    if (ack_status == ACK) {
        rp = (rp + 1) % FIFO_LEN;
        ack_status = NO_ACK;
        wait_ack_ms = 0;
        retry_cnt   = 0;
    } else if (wait_ack_ms > 500) {      // 未确认且超时
        if (retry_cnt < 255) retry_cnt++;
        wait_ack_ms = 0;                 // 归零后本 tick 立即重发
    }
    if (retry_cnt > RESEND_NUM) {        // 超过上限，丢弃
        rp = (rp + 1) % FIFO_LEN;
        retry_cnt = 0;
    }
}

/* 第二步：发送（等待窗口归零时发） */
if (wp != rp) {
    if (wait_ack_ms == 0) {
        send_event_packet();
        have_send_pkg = 1;
    }
    wait_ack_ms += SEND_TIMER_PERIOD;
}
```

- `wait_ack_ms` 是"距上次发送的时长"，每 tick 累加，超过 500ms 归零并重发
- 只有 ACK（或超限丢弃）才让 `rp` 前进
- 只有本 tick 没发事件时才会发数据包（`have_send_pkg`），事件优先

### 7.3 事件报文

```
payload[0..1] = IRQ 状态（大端）
payload[2]    = report_id（每入队自增）
payload[3]    = event_ex
→ 0x16 事件帧
```

主机回 `AA 11 16 01 <report_id> CRC`：

```c
if (Gh3x2xGetProtocolEventReportId() == payload[0])   // 队首 id
    Gh3x2xSetProtocolEventAck();
```

该 ACK 命令**不回响应**（`*pusRespondLen = 0`）。

### 7.4 事件优先级

定时器每轮**先发事件、后发数据**，事件队列为空时才发数据包，避免 ACK 超时丢事件。

---

## 8. 发送队列与定时器

实现：`gh_demo_protocol.c:84` 起。

### 8.1 结构

```c
GU8 data_fifo[64][255];     // 每槽一整帧
GU8 event_fifo[16][4];
```

写 FIFO：满（差值 ≥ 63）或低功耗深睡时直接丢弃并返回；单次写入长度被钳位到
`GH3X2X_PROTOCOL_DATA_PKG_SIZE-4 = 251`（上游 `Gh3x2xDemoSendProtocolData()` 已把整帧
限制在 238 以内，该钳位不会触发）。

### 8.2 定时器状态机

```
IDLE ──有数据入队──► RUNNING（启动 10ms 周期定时器）
RUNNING ──两个队列都空──► 再次检查（防竞争）──仍空──► IDLE（停定时器）
```

每轮顺序：

1. 事件队列：确认/超时/重发（第 7 章）
2. 若本轮未发事件：取数据队首发送 `5 + buf[3]` 字节（`LEN+5`）
3. 队列全空 → 停定时器；停止后再检查一次，防止停止瞬间又有入队

### 8.3 与会话（BLE 连接）绑定

`gh3220_protocol_user.c:update_session()` 在 BLE 会话变化时：

```c
Gh3x2xSerialSendTimerStop();
Gh3x2x_HalSerialFifoInit();      // 清 FIFO、重置 id/retry/ack
event_rp = event_wp;             // 丢弃旧会话未发事件
```

保证旧连接的未发/未确认事件不会串到新连接。

---

## 9. 与 SDK 的接口契约

### 9.1 协议对外依赖的函数

| 接口 | 位置 | 默认实现 | 说明 |
|---|---|---|---|
| `Gh3x2xUserSerialSendData(GU8*, GU16)` | 应用实现 | 无 | 真正把字节写到 BLE/UART |
| `Gh3x2x_HalSerialSendData(GU8*, GU16)` | `gh_demo_user.c:812` | 转调上者 | SDK 内部发送出口 |
| `Gh3x2xSerialSendTimerInit(GU8 ms)` | 平台 | weak 空实现 | 建立周期定时器 |
| `Gh3x2xSerialSendTimerStart(void)` | 平台 | weak 空实现 | 启动/重装定时器 |
| `Gh3x2xSerialSendTimerStop(void)` | 平台 | weak 空实现 | 停止定时器 |
| `Gh3x2x_HalSerialFifoInit(void)` | `gh_demo_protocol.c:84` | 有 | 清 FIFO/状态，初始化定时器 |
| `Gh3x2x_HalSerialWriteDataToFifo(buf,len)` | `:102` | 有 | 数据入队（满则丢） |
| `Gh3x2x_HalSerialWriteEventToFifo(evt,ex)` | `:182` | 有 | 事件入队（满则丢） |
| `Gh3x2xSerialSendTimerHandle(void)` | `:225` | 有 | 定时器回调体（须在任务上下文调用） |
| `Gh3x2xSetProtocolEventAck(void)` | `:172` | 有 | 标记事件已确认 |
| `Gh3x2xGetProtocolEventReportId(void)` | `:177` | 有 | 取队首事件 id |
| `Gh3x2xDemoSendProtocolData(buf,len)` | `:381` | 有 | 上层组包后的统一出口 |
| `GH3X2X_RegisterGetFirmwareVersionFunc(fw,demo,boot)` | `gh_uprotocol.c:588` | 有 | 注入版本字符串来源 |

### 9.2 协议提供给 SDK 的入口

| 接口 | 用途 |
|---|---|
| `GH3X2X_UprotocolParseHandler()` | 单帧解析 + 分发 + 组响应 |
| `GH3X2X_UprotocolPacketFormat()` | 组包 |
| `Gh2x2xUploadDataToMaster()` / `Gh2x2xUploadZipDataToMaster()` | 数据上报 |
| `GH3X2X_GetFuncModeOpened()` / `GetTargetFuncMode()` | 功能位查询 |
| `GH3X2X_ReportIrqStatus()` | 旧事件上报接口（当前无调用者） |

### 9.3 上下文与线程

- 解析器、上传状态机、FIFO 都是**单例**，不内置锁
- 必须由集成层保证串行（本仓库用一把 `gh3220_sdk_mutex`，详见集成文档第 7 章）
- `Gh2x2xUpload*()` 含浮点运算与 memcpy，不宜在 ISR 中调用

### 9.4 数据生产者的契约

协议从 `STGh3x2xFrameInfo`（`gh_drv.h:663`）取数，关键字段：

```c
GU32 unFunctionID;
STGh3x2xFunctionInfo *pstFunctionInfo;   // uchChnlNum / unChnlEnForUserSetting
GU8  *pchChnlMap;
GU32 *punFrameRawdata;                   // 24bit 值 + 8bit tag
GS16 *pusFrameGsensordata;
STCapRawdata  *pstFrameCapdata;
STTempRawdata *pstFrameTempdata;
GU32 *punFrameAgcInfo;                   // gain | drv | dc-cancel
GU32 *punFrameFlag;                      // 5 个
GU32 *punFrameCnt;
STGh3x2xAlgoResult *pstAlgoResult;       // uchUpdateFlag/uchResultNum/usResultBit/snResult[16]
```

这些缓冲**由所有功能共享**，必须在处理当拍同步上传（集成文档 4.2）。

---

## 10. 已知问题

> 均为阅读当前代码得出，建议实测确认后修复。

### 10.1 `GH3X2X_UprotocolPacketMaxLenConfig` 语义不一致

传入"最大 payload 长度"（238），函数按"最大整帧长度"保存，调用点又用
`GET_PAYLOAD_LEN_SUPPORT()` 再减 5。理解这一点才能解释"一包只能放 233 字节"。

### 10.2 `Gh3x2xDemoSendProtocolData` 长度校验边界

```c
if ((usProtocolDataLen > GH3X2X_UPROTOCOL_PAYLOAD_LEN_MAX) || (0 == usProtocolDataLen)) return;
```

传入的是**整帧长度**（最大 238），却与 `PAYLOAD_LEN_MAX`（238）比较，恰好相等不触发返回；
若以后改 `PACKET_LEN_MAX`，这里会先于其他逻辑出错。

### 10.3 0x2C 功能信息包发送偏移错误

`gh_uprotocol.c:1397-1399`：

```c
GH3X2X_UprotocolPacketFormat(CMD_FUNCTION_INFO_UPDATE, puchUploadBuf - 4, puchUploadBuf, len);
Gh3x2xDemoSendProtocolData(puchUploadBuf, usRespondLen);   // ← 应为 puchUploadBuf - 4
```

整包从 payload 位置开始发送，主机收到的是 `[function_id][function info]...` 而非 `AA 11 2C ...`，
末尾多发 4 字节残留数据。表现是**每次功能启动后第一包功能信息丢失**。
触发条件：功能启动后第一帧（`*punFrameCnt == 0`）。

### 10.4 AGC 差分块多 1 字节

`GH3X2X_GetDataDiff()` 没有 tag 块却沿用 `N = (cnt4+1)/2 + 1`，导致 `size` 比实际多 1，
编码时把 1 个 0 字节写进帧。编解码两端一致所以功能正常，第三方解析必须以长度字段为准跳过。

### 10.5 TAIL flush 在下采样时可能失效

`usFrameCnt == usFrameNum-1` 是唯一的批次结束判据，而 `usFrameCnt` 只统计"已上传帧"。
当 `uchDownSampleFactor > 0` 时该等式可能永不成立，包只能等累计到 233 字节才发。
默认配置（`CONFIG_GH3220_PPG_DOWNSAMP_CHANNEL_NUM=0`、采样率修改关闭）不触发。

### 10.6 上下行各有一套凑帧状态机

app 的 `gh3220_frame_feed()` 已经切好整帧，SDK 内 `Gh3x2xDemoHandleRecvBleData()`
又跑一遍。功能正确但多一次拷贝与状态机，精简时可只保留一层。

### 10.7 事件通道的边界行为

- 队列满时静默丢弃事件，只打日志，无错误码
- `RESEND_NUM` 默认 255（无限），事件可能永久占用队首，阻塞后续事件
- `report_id` 为 `GU8`，回绕本身安全（队列深度 16），但主机缓存旧 id 延迟 ACK 可能误确认

### 10.8 无参命令不能发 LEN=0

`Gh3x2xDemoHandleRecvBleData()` 的 `WAIT_LEN` 分支对 `LEN == 0` 直接复位状态机
（`gh_demo_protocol.c:543`），因此 `GET_MAX_LEN(0xA0)`、`GET_OPENED_FUNC(0x25)`、
`GET_CONN_STATUS(0x1A)` 这些天然无参的命令，**必须补一个占位字节**才能被设备接收。
客户端已在 `cmd.rs` 里统一补 `0x00`。这是实测踩到的问题。

### 10.9 FIFO_PACKAGE_SEND_ENABLE 下 rawdata 只有 1 字节

该模式 rawdata 段写 `g_uchFifoPackageID`（8 位包号）而非原始数据，主机需从芯片 FIFO 另行读取。
长度计算与 `package_type` 的模式位必须一起变。

---

## 11. 附录

### 11.1 常量速查

| 常量 | 值 | 位置 |
|---|---|---|
| `GH3X2X_UPROTOCOL_FIXED_HEADER` | 0xAA | `gh_uprotocol.h:268` |
| `GH3X2X_UPROTOCOL_VERSION` | 0x11 | `:271` |
| `GH3X2X_UPROTOCOL_PACKET_LEN_MAX` | 243 | `gh_drv.h:475` |
| `GH3X2X_UPROTOCOL_PAYLOAD_LEN_MAX` | 238 | `gh_drv.h:487` |
| 实际单包 payload 上限 | 233 | 由 `GET_PAYLOAD_LEN_SUPPORT` 推导 |
| CRC8 初值 / 多项式 | 0xFF / 0x07 | `gh_uprotocol.c:105/264` |
| `RAWDATA_DIFF_SIZE` | 8（→9 个半字节） | `gh_uprotocol.h:295` |
| `RAWDATA_DIFF_BYTE_SIZE` | 4 | `:298` |
| `GH3X2X_UPLOAD_PACKAGE_HEAD_SIZE` | 8 | `gh_uprotocol.h:28` |
| 数据 FIFO | 64 × 255B | `gh_demo_protocol.c:71` |
| 事件 FIFO | 16 × 4B | `:74` |
| 发送周期 | 10ms | Kconfig |

### 11.2 示例：一帧完整的数据上行（非压缩）

```
AA 11 0B <LEN>
   01                 function_id = 1
   07                 data_type = gs|algo|agc
   00 00 00 03        channel_mask = ch0, ch1
   00                 package_type = 非压缩
   <data_len>
   -- 帧数据 --
   0A                 frame_id = 10
   XX XX XX XX XX XX  gs x/y/z
   XX*8               rawdata ch0, ch1
   XX*8               agc   ch0, ch1
   <result> ...
<CRC8>
```

### 11.3 CRC8 表生成

```c
uint8_t tbl[256];
for (int i = 0; i < 256; i++) {
    uint8_t c = (uint8_t)i;
    for (int b = 0; b < 8; b++)
        c = (c & 0x80) ? (uint8_t)((c << 1) ^ 0x07) : (uint8_t)(c << 1);
    tbl[i] = c;
}
/* 与 g_uchCrc8TabArr 前几项比对: 00 07 0E 09 1C 1B 12 15 */
```

### 11.4 关键代码索引

| 功能 | 位置 |
|---|---|
| 组包 | `gh_uprotocol.c:289` |
| 解析状态机 | `:333` |
| 命令分发 | `:1208` |
| 非压缩上报 | `:1453` |
| 压缩上报 | `gh_zip.c:290` |
| 差分编码 | `gh_zip.c:60`、`:167` |
| 发送 FIFO / 定时器 | `gh_demo_protocol.c:102`、`:225` |
| 事件上报 / ACK | `gh_demo_protocol.c:182`、`gh_uprotocol.c:1151` |
| 凑帧 | `gh_demo_protocol.c:507` |
