# UProtocol 与 SDK 耦合分析（demo_kernel_code）

本文只处理 **UProtocol**（`demo_kernel_code/module/gh_protocol/`）与 SDK 的耦合关系：

- 协议本身（帧格式、状态机、命令表、压缩算法）见
  [`gh3220_uprotocol_reference.md`](gh3220_uprotocol_reference.md)。
- 本文讲**这套协议如何嵌进 SDK**：数据从芯片 FIFO 一路到 BLE 的每一跳、控制命令落到硬件的每一跳、
  被共享的全局变量、编译期/运行期开关、执行上下文与并发、以及在实际代码里发现的耦合缺陷。
- SDK 里另一套协议 **GHRPC** 与本文件无关，见
  [`gh3220_ghrpc_reference.md`](gh3220_ghrpc_reference.md)。

分析对象只包括 `modules/lierda/gh3220_sdk/demo_kernel_code`（即真正参与编译的 UProtocol 栈），
必要时引用 `port/zephyr` 与 `l-evk-t2-gh3220/app_cardiff_a` 的接线代码。

- 代码版本：`gh3220_sdk` 工作区（`af51a39` 之后）
- 行号相对仓库根目录

## 关键结论（先看这里）

1. 协议不是独立模块，它和驱动**共用同一组全局缓冲**（`g_punGh3x2xFrameRawdata` 等），
   帧数据必须在处理当拍同步上传，不能缓存指针延后发送。
2. 上行链路的唯一入口是 `gh_drv_control.c:2926`，三个入参中 `usFrameCnt/usFrameNum`
   决定了包何时 flush；它们与下采样配置强相关。
3. 下行命令里，`WORK_MODE(0x10)` 是 `START_CTRL(0x0C)` 的**隐式前置条件**，不发就永远启动失败。
4. 协议侧维护的 `g_unFuncModeOpened` 与真正生效的 `g_unDemoFuncMode` 是两套状态，
   在 WORK_MODE、配置切换、复位后可能失配。
5. 协议启动采样**不会**初始化算法（app 的 `Gh3x2xDemoStartSampling` 才会），
   在带算法库的构建里必须由应用先行初始化。
6. 本移植用一把 `gh3220_sdk_mutex` 把上下行、定时器、启停全部串行化，
   SDK 内部的单例状态才因此安全。
7. 发现一个确定的发送缺陷：0x2C 功能信息包从错误偏移发送（8.1）。
8. 下采样开启后，批次末尾 flush 的判据（TAIL）会失效（8.2），默认配置不受影响。

---

## 目录

1. [代码地图与执行上下文](#1-代码地图与执行上下文)
2. [上行数据链路（逐跳）](#2-上行数据链路逐跳)
3. [下行控制链路](#3-下行控制链路)
4. [共享状态与数据契约](#4-共享状态与数据契约)
5. [编译期配置体系](#5-编译期配置体系)
6. [运行期配置体系](#6-运行期配置体系)
7. [时序、并发与重入](#7-时序并发与重入)
8. [本次分析发现的缺陷与风险](#8-本次分析发现的缺陷与风险)
9. [改造与移植检查清单](#9-改造与移植检查清单)
10. [附录](#10-附录)

---

## 1. 代码地图与执行上下文

### 1.1 文件职责（只列与协议相关的）

| 文件 | 角色 | 与协议的关系 |
|---|---|---|
| `kernel/gh_demo_protocol.c` | 协议**传输层**（组帧/拆帧/发送队列） | 收发字节流、64 槽数据 FIFO、16 槽事件 FIFO、10ms 定时器 |
| `kernel/gh_demo.c` | Demo 调度层 | SDK 入口、IRQ 处理、启停、算法调用、事件上报 |
| `kernel/gh_demo_hook.c` | 钩子实现 | 帧处理后调用算法、填充 `pstAlgoResult` |
| `kernel/gh_demo_user.c` | 平台适配（weak） | `Gh3x2x_HalSerialSendData`、定时器空实现 |
| `kernel/gh_demo_config.h` | 编译期配置 | `__SUPPORT_PROTOCOL_ANALYZE__` 等 |
| `driver/src/gh_drv_control.c` | 驱动核心 | FIFO 读取、帧重组、`STGh3x2xFrameInfo` 填充、**调用上传** |
| `driver/src/gh_drv_config.c` | 配置管理 | 虚拟寄存器、功能表、AGC/采样率 |
| `driver/src/gh_drv_interface.c` | 寄存器/FIFO 原语 | `GH3X2X_ReadFifo()`、SPI/I2C 读写 |
| `module/gh_protocol/gh_uprotocol.c` | 协议**语义层** | 命令解析/分发/组包、非压缩上传 |
| `module/gh_protocol/gh_zip.c` | 压缩上传 | 4bit 差分编码 |
| `module/gh_agc/gh_agc.c` | AGC | 提供 `g_pstGh3x2xAgcInfo`（上传的 agc 段来源） |
| `module/gh_ecg/`, `gh_soft_adt/`, `gh_other/` | 功能模块 | 产生事件、影响帧标志 |
| `port/zephyr/include/gh3220_kernel_config.h` | Kconfig→宏 | 把 Zephyr 配置翻译成 SDK 宏 |

### 1.2 执行上下文

```
ISR（GPIO 中断）
  └─ 仅置标志 / 投递 work（不在中断里跑协议）

work / 线程（app 层，全部串行化）
  ├─ gh3220_interrupt_process()   ← 采样中断处理（上行数据源）
  ├─ gh3220_protocol_receive()    ← BLE 下行命令
  ├─ protocol_work_handler()      ← 10ms 发送定时器
  ├─ gh3220_start() / gh3220_stop()
  └─ gh3220_init()
        └─ 全部包在 k_mutex_lock(&gh3220_sdk_mutex) 内
```

`l-evk-t2-gh3220/app_cardiff_a/src/gh3220_framework.c:67` 起：

```c
static void gh3220_interrupt_process(void)
{
    k_mutex_lock(&gh3220_sdk_mutex, K_FOREVER);
    if (sdk_ready) {
#ifndef CONFIG_GH3220_RTC_POLLING
        hal_gh3x2x_int_handler_call_back();
#endif
        Gh3x2xDemoInterruptProcess();      // ← 数据上传发生在这里
    }
    k_mutex_unlock(&gh3220_sdk_mutex);
}
```

结论：**在本移植里，上下行与定时器被同一把互斥锁串行化**，所以 SDK 内部那些"假定单线程"的
写法（`receive_trans_buff`、FIFO 读写指针、`g_uchGh3x2xUploadStatus`）是安全的；
但只要换一个不做全局串行化的集成方式，就必须自己补锁。

`gh3220_protocol_receive()` 同样加锁（`app_cardiff_a/src/gh3220_protocol_user.c:57`），
并在锁内完成"凑帧 + 命令执行 + 入发送队列"。

### 1.3 三层入口

| 层 | 入口 | 调用者 |
|---|---|---|
| 字节流 | `Gh3x2xDemoProtocolProcess(buf, len)` | app（BLE 收包回调） |
| 单帧 | `Gh3x2xDemoOneFrameDataProcess(frame, len)` | 内部凑帧状态机 |
| 命令 | `GH3X2X_UprotocolParseHandler()` → `...CmdHandler()` | 单帧入口 |

发送侧：

| 层 | 入口 | 说明 |
|---|---|---|
| 入队 | `Gh3x2x_HalSerialWriteDataToFifo()` / `...EventToFifo()` | 满则丢弃 |
| 出队 | `Gh3x2xSerialSendTimerHandle()` | 10ms 周期，事件优先 |
| 落地 | `Gh3x2x_HalSerialSendData()` → `Gh3x2xUserSerialSendData()` | app 实现为 BLE notify |

---

## 2. 上行数据链路（逐跳）

### 2.1 总览

```
芯片 FIFO
  │  ① GH3X2X_ReadFifodata()
  ▼
g_puchGh3x2xReadRawdataBuffer (原始 4B/采样字)
  │  ② Gh3x2xDemoFunctionProcess()  —— 按功能遍历 g_pstGh3x2xFrameInfo[]
  ▼
GH3x2xFunctionProcess()
  │  ③ GH3x2xGetFrameNum() 估算本批帧数
  │  ④ GH3x2xGetFrameDataAndProcess() 逐个完整帧
  ▼
GH3x2xHandleFrameData()
  │  ⑤ 填充 STGh3x2xFrameInfo（rawdata/agc/flag/gs/cap/temp）
  │  ⑥ hook → 算法 → pstAlgoResult
  ▼
Gh2x2xUploadZipDataToMaster() / Gh2x2xUploadDataToMaster()
  │  ⑦ 组包 → Gh3x2xDemoSendProtocolData()
  ▼
Gh3x2x_HalSerialWriteDataToFifo()  (64 × 255B)
  │  ⑧ 10ms 定时器
  ▼
Gh3x2x_HalSerialSendData() → Gh3x2xUserSerialSendData() → gh3220_ble_notify()
```

### 2.2 IRQ 状态读取

`Gh3x2xDemoInterruptProcess()`（`kernel/gh_demo.c:1699`）是一个 `do/while` 循环：

```
Step 1  GH3X2X_GetIrqStatus()          读中断状态                       :1772
Step 2  硬 ADT 事件处理                                                  :1798
Step 3  若 FIFO 中断 → GH3X2X_ReadFifodata() 循环读空 FIFO             :1865
Step 4  ECG 软导联脱落处理                                              :1972
Step 5  软 AGC 处理                                                     :1989
Step 6  芯片复位事件处理（重建配置）                                     :2024
Step 7  Gh3x2xDemoReportEvent(usGotEvent, uchEventEx)  上报事件         :2117
Step 8  算法 + 数据上传（Gh3x2xDemoFunctionProcess）                    :2162
Step 9  用户事件钩子                                                    :2200
```

关键点：**Step 7 在 Step 8 之前**。事件先入事件 FIFO，数据后入数据 FIFO，配合发送端
"事件优先"策略，保证事件不会被大包数据堵住。

### 2.3 原始 FIFO 字格式

每个采样通道占 4 字节（`GH3X2X_FIFO_RAWDATA_SIZE = 4`），按 32 位大端拼装：

```
bit31..27  tag（5bit）        —— 通道标签，索引 puchTagArray[]
bit26      ADJ_FLAG0 / FAST_RECOVERY
bit25      ADJ_FLAG1
bit24      保留
bit23..0   24bit 原始值
```

对应宏（`driver/inc/gh_drv_common.h:157/184/187/193`）：

```c
#define GH3X2X_DWORD_RAWDATA_GET_SLOT_ADC_NUM(x)  (((x) & 0xF8000000) >> 27)
#define GH3X2X_RAWDATA_GET_ADJ_FLAG0(x)           (((x) & 0x04000000) >> 26)
#define GH3X2X_RAWDATA_GET_ADJ_FLAG1(x)           (((x) & 0x02000000) >> 25)
#define GH3X2X_RAWDATA_GET_FAST_RECOVERY_FLAG(x)  (((x) & 0x04000000) >> 26)
```

> `FAST_RECOVERY` 与 `ADJ_FLAG0` 复用 bit26，按功能区分语义（ECG/PWTT 的 0 通道解释为快速恢复）。

### 2.4 帧重组

三个函数构成重组逻辑：

| 函数 | 位置 | 作用 |
|---|---|---|
| `GH3x2xCreatTagArray()` | `driver/src/gh_drv_control.c:2628` | 由 `pchChnlMap` 生成 `tag → 通道号` 映射表，并算出本功能的"完整帧掩码" |
| `GH3x2xGetFrameNum()` | `:2649` | 用掩码推演这批 FIFO 数据里有多少个完整帧 |
| `GH3x2xGetFrameDataAndProcess()` | `:2871` | 逐 4 字节扫描，集齐一个完整帧就处理一次 |

关键代码：

```c
// 通道映射：pchChnlMap[i] = slot|adc  →  tag = chnlMap>>3
for (ch = 0; ch < chnlNum; ch++)
    puchTagArray[puchChnlMap[ch] >> 3] = ch;

// 完整帧掩码：低 chnlNum 位按用户通道使能过滤
*unCompeletMask = (((1ULL << chnlNum) - 1) & pstFunctionInfo->unChnlEnForUserSetting);
```

扫描时：

```c
uchTag   = buf[pos] >> 3;           // 4 字节字的最高字节高 5 位
chnlId   = puchTagArray[uchTag];
if (chnlId != 0xFF) {
    unTempIncompeletFlag |= (1 << chnlId);
    punIncompleteRawdata[chnlId] = MAKEUP_DWORD(buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]);
}
if (unCompeletMask == (unCompeletMask & unTempIncompeletFlag)) {
    // 一帧齐了 → 处理
}
```

两条不变量：

1. **帧可以跨 FIFO 批次**：`punIncompleteChnlMapBit` 是每功能持久状态，本次读到的半个帧会和下次读到的补齐。
2. **同一通道后到的数据覆盖先到的**：`punIncompleteRawdata[]` 直接赋值，不做去重。
   正常时序下同帧内一个标签只出现一次；若硬件/配置异常导致重复，后值会覆盖前值且不留痕迹。

### 2.5 帧字段填充来源表

`GH3x2xHandleFrameData()`（`:2748`）每帧先清空 `punFrameFlag/ pstAlgoResult / punFrameAgcInfo`，
再逐通道填充：

| 目标字段 | 来源 | 说明 |
|---|---|---|
| `punFrameRawdata[i]` | FIFO 字 `& 0x00FFFFFF` | 24 位原始值 |
| `punFrameFlag[0]` bit i | `ADJ_FLAG0` | 每通道一位 |
| `punFrameFlag[1]` bit i | `ADJ_FLAG1` | 每通道一位 |
| `punFrameFlag[2]` bit0 | `FAST_RECOVERY` | 仅 ECG/PWTT 的 0 通道 |
| `punFrameFlag[2]` bit1 | 帧计数 == 0 | 首帧标志（`Gh3x2xSetFrameFlag2()`，`gh_demo.c:3502`） |
| `punFrameFlag[2]` bit4 | 功能为 `SOFT_ADT_IR` | 软件 ADT 红外标识 |
| `punFrameFlag[4]` bit0 | `g_uchChipDivZeroFlag` | 除零标志，读取后清除 |
| `punFrameAgcInfo[i]` bit0..3 | `uchTiaGainRecord[tag]` | TIA 增益码 |
| `punFrameAgcInfo[i]` bit8..15 | `uchDrvCurrentRecord[tag/4]` | LED 驱动电流 |
| `punFrameAgcInfo[i]` bit24..31 | `uchDcCancelRecord[tag]` | DC 消除码 |
| `pusFrameGsensordata[0..2]` | 加速度软 FIFO | 按 `fGsensorIndex` 做**线性插值**对齐 PPG 帧 |
| `pusFrameGsensordata[3..5]` | 陀螺软 FIFO | 仅 `g_uchGyroEnable` |
| `pstFrameCapdata` | 电容软 FIFO | 仅 `GH3X2X_GetCapEnableFlag()` |
| `pstFrameTempdata` | 温度软 FIFO | 仅 `GH3X2X_GetTempEnableFlag()` |
| `pstAlgoResult` | 算法 hook | 见 2.6 |

插值细节（`gh_drv_control.c:2802` 起）：

```c
fGsensorIndex = fGsensorStep * 已处理帧序号;   // 可能是小数
if (fGsensorIndex != (GU16)fGsensorIndex)
    gs[i] = a[i]*(1-frac) + b[i]*frac;         // 相邻两点线性插值
else
    gs[i] = a[i];
```

即 PPG 帧率高于 G-Sensor 上报率时，用插值把传感器数据"铺"到每一帧上，避免复用整点值。

> 这里的 `uchTag` 是循环结束后残留的最后一个 tag，`GH3x2xHandleFrameData()` 会把它回写给调用者，
> 后续 `Gh2x2xUpload*()` 用 `puchTagArray` 而不是这个值来算通道掩码，二者不要混用。

### 2.6 算法结果注入

`GH3x2xHandleFrameData()` 末尾：

```c
pstFrameInfo->pstAlgoResult->uchUpdateFlag = 0;
HOOK_FUNC_CALL(g_pGh3x2xGetIoStructInfoHookFunc, (pstFrameInfo));
```

钩子实现在 `kernel/gh_demo_hook.c:163` `gh3x2x_algorithm_get_io_data_hook_func()`：

```c
#if (__DRIVER_LIB_MODE__ == __DRV_LIB_WITH_ALGO__)
    GH3X2X_AlgoCalculate(pstFrameInfo->unFunctionID);   // 算法内部回填 pstAlgoResult
#endif
```

`GH3X2X_AlgoCalculate()` 最终在 `demo_algo_code/goodix_algo_application/src/gh3x2x_demo_algo_call.c`
里写 `snResult[] / usResultBit / uchResultNum / uchUpdateFlag`，例如：

```c
pstFrameInfo->pstAlgoResult->usResultBit  = 0xFF;             // 哪些结果有效
pstFrameInfo->pstAlgoResult->uchResultNum = BitCount(0xFF);   // 有效个数
```

**协议侧对算法完全无感知**：它只读 `pstAlgoResult` 的 4 个字段，因此换算法库不需要改协议，
只要保证这四个字段语义一致。

### 2.7 上传触发与分包

调用点只有一处（`gh_drv_control.c:2926-2930`）：

```c
#if (__SUPPORT_ZIP_PROTOCOL__)
    Gh2x2xUploadZipDataToMaster(pstFrameInfo, usFrameCnt, usFrameNum, puchTagArray);
#else
    Gh2x2xUploadDataToMaster(pstFrameInfo, usFrameCnt, usFrameNum, puchTagArray);
#endif
```

三个入参的含义（很容易误用）：

| 参数 | 含义 | 注意 |
|---|---|---|
| `usFrameCnt` | 本批中**已上传**的帧序号（从 0 开始） | 下采样时 ≠ 硬件帧序号 |
| `usFrameNum` | 本批**完整帧总数**（由 FIFO 推演） | 下采样时仍按原始帧数计 |
| `puchTagArray` | tag→通道号 映射 | 用于生成 8B 包头的通道掩码 |

包的生命周期由两个全局变量控制（`g_uchGh3x2xUploadStatus` / `g_uchOddEvenChangeFlag`），
详见 `gh3220_uprotocol_reference.md` 5.5。这里补充一个**协议外部看不见的约束**：

> `usFrameCnt == usFrameNum-1` 是唯一的"批次结束"判据（TAIL）。它成立的前提是
> **每帧都上传**（`uchDownSampleFactor == 0`）。一旦开启下采样，该条件可能永不成立，
> 见第 8.2 节。

### 2.8 从 FIFO 到 BLE

```
Gh3x2x_HalSerialWriteDataToFifo()        // kernel/gh_demo_protocol.c:102
   ├─ 满（差值 ≥ 63）→ 丢弃 + 返回
   ├─ len > 251 → 截断
   ├─ 拷入 data_fifo[wp]，wp++
   └─ 定时器未运行 → Gh3x2xSerialSendTimerStart()

Gh3x2xSerialSendTimerHandle()            // :225，每 10ms
   ├─ 事件队列优先（带 ACK 重传）
   └─ 数据队列：发送 5 + buf[3] 字节（LEN+5）

Gh3x2x_HalSerialSendData()               // kernel/gh_demo_user.c:812
   └─ Gh3x2xUserSerialSendData()         // app：gh3220_ble_notify()
```

`gh3220_ble_notify()` 返回错误时只打 `LOG_WRN`，不重试、不回压——**协议层默认传输是可靠的**，
这在 BLE 上并不成立（notify 可能因 MTU/缓冲失败）。需要可靠传输时应当在下游实现队列，
或改用 GHRPC 的 secure 通道。

---

## 3. 下行控制链路

### 3.1 解析 → 命令 → 副作用

```
gh3220_protocol_receive()
  └─ gh3220_frame_feed()             // 凑齐 AA 11 CMD LEN payload CRC
       └─ Gh3x2xDemoProtocolProcess(frame, len)
            ├─ Gh3x2xDemoHandleRecvBleData(byte)   // 再凑一次（SDK 内部状态机）
            └─ Gh3x2xDemoOneFrameDataProcess()
                 ├─ GH3X2X_UprotocolParseHandler() // CRC 校验 + 分发 + 组响应
                 ├─ START/STOP → Gh3x2xDemoSamplingControl()
                 ├─ MPT 模式 → g_uchDemoWorkMode = MPT
                 └─ Gh3x2xDemoSendProtocolData(resp, len)
```

注意 app 层已经用 `gh3220_frame_feed()` 切好整帧，SDK 内部又跑了一遍自己的凑帧状态机
`Gh3x2xDemoHandleRecvBleData()`。两者同时存在不会出错（输入已是完整帧），但意味着
**两次 CRC 计算和两套缓冲**；如果要精简，可只保留一层。

### 3.2 命令副作用矩阵

| CMD | 直接改写的状态 | 间接动作 | 影响面 |
|---|---|---|---|
| 0x03 REG_RW | 芯片寄存器 / 虚拟寄存器 | 写虚拟寄存器可能改 `g_uchGsensorEnable`、`g_uchFifoPackageMode` | AGC、采样、FIFO 模式 |
| 0x05 PACKAGE_TEST | — | 回显 | 无 |
| 0x0C START_CTRL | `g_unTargetFuncMode`、`g_unFuncModeOpened` | `Gh3x2xDemoSamplingControl()` | 采样开关、GS/CAP/TEMP 缓存、水位、事件 |
| 0x10 WORK_MODE | `g_unAllFuncMode`、`g_unFuncModeOpened=0`、`g_unTargetFuncMode=0` | 切 MPT 时 `g_uchDemoWorkMode` | **START 的前置条件** |
| 0x12 FIFO_THR_CONFIG | 芯片水位寄存器 | `GH3X2X_FifoWatermarkThrConfig()` | 中断频率、延迟 |
| 0x13 REPORT_EVENT_SET | `g_usGh3x2xEventNeedReport` | — | 事件过滤 |
| 0x14 ECG_CLASSIFICATION | — | `GH3x2xEcgAlgoClassification()` | ECG 算法 |
| 0x16 (ACK) | 事件 ACK 状态 | 事件出队 | 事件通道 |
| 0x17 CHIP_CTRL | `g_uchResetFromProtocolFlag` | 复位/休眠；回调 `ResetByProtocolHook` | 全系统 |
| 0x19 GET_VER | — | 调用注册的版本函数 | 只读 |
| 0x1A GET_CONN_STA | — | `GH3X2X_CommunicateConfirm()` | 只读 + SPI 访问 |
| 0x1C SWITCH_SLOT_FUNC | 同 START_CTRL | 同 START_CTRL | 同 START_CTRL |
| 0x25 GET_OPENED_FUNC | — | 读 `g_unFuncModeOpened` | 只读 |
| 0x26 SET_ECG_CASCADE | 级联状态 | `Gh3x2xEnterCascadeMode()` 等 | ECG 双芯片 |
| 0x2B BP_DATA_TRANS | — | 空实现（只回状态 0） | 无 |
| 0xA0 GET_MAX_LEN | — | 读 `g_uchPacketMaxLenSupport` | 只读 |
| 0xA1 LOAD_REG_LIST | 芯片配置 | `GH3X2X_LoadNewRegConfigArr()` | 全功能 |
| 0xA2 DEBUG_STATUS | — | 读引脚/工作状态/通信确认 | 只读 |

### 3.3 启停：三个入口，一个终点

```
① 协议命令（主机工具）
   START_CTRL/SWITCH_SLOT_FUNC
     → 校验 g_unAllFuncMode（必须先发 WORK_MODE）
     → Gh3x2xDemoSamplingControl(target, START/STOP)

② 应用 API（本仓库健康业务）
   Gh3x2xDemoStartSampling(func)      gh_demo.c:3137
     → Gh3x2xDemoStartAlgoInner()     ← 初始化算法
     → Gh3x2xDemoStartSamplingInner() ← 必要时切换配置（复位+重init）
          → Gh3x2xDemoSamplingControl(func, START)

③ 内部配置切换
   StartSamplingInner 内部先 Gh3x2xDemoSamplingControl(0xFFFFFFFF, STOP) 停全部
```

三者最终都落到 `Gh3x2xDemoSamplingControl()`（`gh_demo.c:2634`）：

```c
if ((GH3X2X_GetConfigFuncMode() & unFuncMode) != unFuncMode)
    unFuncMode &= GH3X2X_GetConfigFuncMode();     // 配置不支持的功能被静默裁掉
if (unFuncMode == 0) return;                      // 静默返回，无错误码

for (每个已注册功能) {
    if (START && 该功能未开) {
        GH3X2X_FunctionStart(frameInfo);          // 写寄存器、开 slot
        ...
    } else if (STOP && 该功能已开) {
        GH3X2X_FunctionStop(frameInfo);
        ADT 停止时上报 WEAR_OFF 事件
    }
}
START: 起 GS/CAP/TEMP 缓存、时间戳同步、更新水位
STOP : 关缓存、置 NEED_FORCE_READ_FIFO
g_unDemoFuncMode |= / &= ~unFuncMode;
Gh3x2xSetCurrentSlotEnReg(g_unDemoFuncMode);
```

**协议入口与 app 入口的差异**（重要）：

| 行为 | 协议入口 | app 入口 |
|---|---|---|
| 校验配置是否支持该功能 | 是（静默裁剪） | 是，且不支持时会**搜索并切换配置表**（复位芯片） |
| 算法初始化 `GH3X2X_AlgoInit` | **否** | 是 |
| 失败反馈 | 响应字节 1 | 无（仅日志） |
| 停机时上报 wear-off | 是 | 是 |

结论：**主机工具通过协议启动采样时不会初始化算法**。在 `__DRV_LIB_WITH_ALGO__` 模式下，
如果算法未由应用侧提前初始化，`pstAlgoResult` 会一直是空，主机看到的是"有原始数据、无算法结果"。
EVK/产测场景可接受，产品形态下应当由应用先初始化，或改写协议入口。

### 3.4 START 的前置条件：WORK_MODE

`g_unAllFuncMode` 只在 `WORK_MODE(0x10)` 里被赋值（`gh_uprotocol.c:984`），初值 0。
`START_CTRL` 的校验是：

```c
if (g_unAllFuncMode == (g_unTargetFuncMode | g_unAllFuncMode))   // 子集判断
    status = 0;
else
    status = 1;                                                   // 失败
```

因此**主机必须先发 0x10 声明允许的功能位，再发 0x0C/0x1C 启停**，否则永远返回 1 且不会启动。
这是协议与 SDK 之间的一个"隐式握手"，文档/工具侧容易踩坑。

另外 `WORK_MODE` 会把 `g_unFuncModeOpened` 和 `g_unTargetFuncMode` 清零，
但**不会停掉正在运行的功能**——`g_unDemoFuncMode`（真正生效的位）不受影响，二者会失配。
下次 0x25 查询返回的已开功能位就会是 0，而硬件仍在采样。

### 3.5 复位命令的钩子链

```
CHIP_CTRL(0x5A/0xC2)
  → g_uchResetFromProtocolFlag = 1
  → GH3X2X_HardReset()/SoftReset()
       → 若 flag == 1：清零 + HOOK_FUNC_CALL(g_pGh3x2xResetByProtocolHookFunc)
```

钩子在 `GH3X2X_RegisterHookFunc()`（`gh_demo.c:1550`）注册，实现为
`gh3x2x_reset_by_protocol_hook`（`kernel/gh_demo_hook.c`）。

`g_uchResetFromProtocolFlag` 的定义与清除点：`gh_drv_control.c:56/234/498/1865`。
它同时被 `GH3X2X_SoftReset()`、`GH3X2X_HardReset()` 和初始化流程检查，
是"协议触发的复位"与"其他来源复位"的唯一区分手段。

### 3.6 版本信息链

```
GET_VER(0x19)
  → GH3X2X_GetVersion(type, buf)          gh_uprotocol.c:175
       ├─ FW/DEMO/PROTOCOL/DRV → 静态字符串
       ├─ CHIP   → 读芯片 ID + efuse 版本，格式化 "vXX_evYY"
       ├─ VIRTUAL_REG → GH3X2X_GetVirtualRegVersion()
       └─ ALGO   → GH3X2X_AlgoVersion(functionOffset, buf)
```

固件版本字符串来自注册的三个函数指针（`gh_demo.c:1628`）：

```c
GH3X2X_RegisterGetFirmwareVersionFunc(GH3X2X_GetFirmwareVersion,
                                      GH3X2X_GetDemoVersion,
                                      GH3X2X_GetEvkBootloaderVersion);
```

`GH3X2X_AlgoVersion()` 由算法层实现（`demo_algo_code/.../gh3x2x_demo_algo_call.c`），
按功能偏移返回各自库版本。若算法未编译，协议侧返回 `"algorithm disable!"`。

### 3.7 空实现与未接线命令

| 命令 | 现状 | 风险 |
|---|---|---|
| 0x26 SET_ECG_CASCADE | 有实现，但**响应被注释掉** | 主机收不到 ACK，只能靠超时 |
| 0x2B BP_DATA_TRANS | 只把状态置 0 并回包 | 形似成功，实际未做任何事 |
| 0x04/0x07/0x08/0x09/0x0A/0x15/0x29/0x2A | 未实现，走 default 回空包 | 工具若按"有响应即成功"判断会误判 |
| 0x05 PACKAGE_TEST | 回显 | 可用于链路自检 |

---

## 4. 共享状态与数据契约

耦合总览（箭头表示"改写/驱动"）：

```
        主机命令                      ┌────────────────────┐
   ──────────────────►               │ 协议层状态          │  g_unAllFuncMode
                                     │ gh_uprotocol.c      │  g_unFuncModeOpened
                                     │ gh_zip.c            │  g_unTargetFuncMode
                                     └─────┬───────┬──────┘  g_uchGh3x2xUploadStatus
                           启停命令        │       │  数据打包
                                           ▼       ▼
                     ┌──────────────────────────┐  ┌────────────────────┐
                     │ gh_demo.c                │  │ STGh3x2xFrameInfo  │
                     │ g_unDemoFuncMode         │◄─┤ (共享 scratch)      │
                     │ g_uchDemoWorkMode        │  └────────▲───────────┘
                     └────┬───────────────▲─────┘           │ 填充
                          │启停            │事件            │
                          ▼                │                │
                     ┌──────────────┐  ┌───┴────────┐  ┌────┴─────────────┐
                     │ 芯片/寄存器   │  │ IRQ 处理    │  │ 驱动帧重组        │
                     │ GH3X2X_*     │  │ gh_demo.c  │  │ gh_drv_control.c │
                     └──────────────┘  └────────────┘  └──────────────────┘
```

### 4.1 全局状态表（谁写、谁读、失配后果）

| 变量 | 定义 | 写入者 | 读取者 | 失配后果 |
|---|---|---|---|---|
| `g_unAllFuncMode` | `gh_uprotocol.c:84` | WORK_MODE | START/SWITCH 校验、GET_AVAILABLE | 未发 WORK_MODE 时 START 永远失败 |
| `g_unFuncModeOpened` | `:87` | START/STOP/WORK_MODE | GET_OPENED_FUNC | 与 `g_unDemoFuncMode` 可能不一致（复位、异常） |
| `g_unTargetFuncMode` | `:90` | START/STOP/WORK_MODE | `Gh3x2xDemoSamplingControl` 入参 | 每次命令覆盖，多命令并发会丢前一次 |
| `g_unDemoFuncMode` | `gh_demo.c` | SamplingControl | IRQ 处理、采样率、slot 使能 | 真正决定"哪些功能在跑" |
| `g_usGh3x2xEventNeedReport` | `gh_uprotocol.c:96` | REPORT_EVENT_SET | 事件过滤 | 事件被静默丢弃 |
| `g_uchPacketMaxLenSupport` | `:55` | Init 时配置 | 所有组包 | 改错会截断/越界 |
| `g_uchDemoWorkMode` | `gh_demo.c:88` | WORK_MODE(MPT)、app | IRQ 处理、恢复策略 | MPT 下 IRQ 处理直接 return（不采样） |
| `g_uchGh3x2xStatus` | `gh_drv_control.c:59` | Init/Start/Stop | DEBUG_STATUS | 查询结果失真 |
| `g_uchResetFromProtocolFlag` | `:56` | CHIP_CTRL | Reset 流程 | 钩子不触发或误触发 |
| `g_uchGsensorEnable` | `:48` | 虚拟寄存器 | 上传包头 | GS 段长度与包头位不一致 |
| `g_uchFifoPackageMode`/`ID` | `:31/30` | 虚拟寄存器、读 FIFO | 上传包头/rawdata 段 | 主从解析错位 |
| `g_uchOddEvenChangeFlag` | `:42` | zip 上传 | 包头 bit1 | 主机差分基准错 |
| `g_uchGh3x2xUploadStatus` | `gh_uprotocol.c:68` | 上传函数 | 上传函数 | 见 8.4 |
| `g_stZipLastData` | `gh_zip.c:24` | zip 上传 | zip 上传 | 差分基准，跨功能共用 |

### 4.2 共享 scratch buffer（协议之外最容易踩的坑）

`gh_demo.c:297-305`：

```c
GU32 g_punGh3x2xFrameRawdata[GH3X2X_FUNC_CHNL_NUM_MAX];
GS16 g_psGh3x2xFrameGsensorData[6];
GU32 g_punGh3x2xFrameAgcInfo[GH3X2X_FUNC_CHNL_NUM_MAX];
GU32 g_punGh3x2xFrameFlag[8];
STGh3x2xAlgoResult g_stGh3x2xAlgoResult;
```

**所有功能共用这几块缓冲**（`g_stHR_FrameInfo`、`g_stSPO2_FrameInfo` 等都指向同一组地址）。
由此产生三条隐含契约：

1. 一个功能的帧必须**在处理完立刻上传**，不能把 `pstFrameInfo` 或它指向的数组存起来延后使用；
2. 功能之间不能并行处理（`Gh3x2xDemoFunctionProcess` 是串行 for 循环，正好满足）；
3. 任何在 IRQ 之外缓存帧数据的改造（比如投递到队列）都必须**深拷贝**，否则会被下一个功能覆盖。

每个功能**私有**的状态只有：通道映射、通道数、未完成帧标志、未完成原始值、帧计数、last gain、
下采样计数、slot 位。

但这些"私有"状态是否真的独立，取决于 `__FUNC_RUN_SIMULTANEOUSLY_SUPPORT__`（默认 1）：

| 取值 | 未完成缓冲 / 帧计数 / last gain 的下采样信息 |
|---|---|
| 1（默认） | 每功能一套（`..._HR_...`、`..._SPO2_...`） |
| 0 | 全部退化为 `..._MAIN_FUNC_...` 共享一套 |

取 0 时，两个功能同时运行会共用帧计数与"未完成帧"缓冲，先处理的功能会把另一个功能的半帧状态冲掉。
当前默认值 1，安全；如果为了省 RAM 把它改成 0，必须保证同一时刻只有一个功能在采样。

### 4.3 协议内部状态

| 状态 | 作用域 | 复位时机 |
|---|---|---|
| `g_emPacketParseStatus` 等解析状态 | 全局单例 | 每次帧结束或错误 |
| `g_uchPacketPayloadArr` | 全局单例（`STGh3x2xProtocolData`） | 每次命令覆盖 |
| 上传状态机（status/oddEven/bufUse） | 全局 | 每次 flush |
| zip 历史（`g_stZipLastData`） | 全局 | 仅 odd/even 绝对帧时重建 |
| 数据 FIFO / 事件 FIFO | 全局 | `Gh3x2x_HalSerialFifoInit()` |

**解析器是单例**：如果同时存在两条下行通道（例如 BLE + UART 同时喂字节），状态机会互相踩。
本仓库只有 BLE 一条，安全。

---

## 5. 编译期配置体系

### 5.1 宏的三层来源

```
Zephyr Kconfig（zephyr/Kconfig.protocol / Kconfig.kernel）
        │  CONFIG_GH3220_*
        ▼
port/zephyr/include/gh3220_kernel_config.h
        │  __XXX__ 形式的 SDK 宏
        ▼
demo_kernel_code/kernel/gh_demo_config.h
        │  若未被覆盖则给默认值（#ifndef）
        ▼
源码 #if
```

例：

```c
// zephyr/Kconfig.protocol
config GH3220_SUPPORT_ZIP_PROTOCOL
    bool "Enable compressed protocol"
    default y

// gh3220_kernel_config.h
#ifdef CONFIG_GH3220_SUPPORT_ZIP_PROTOCOL
#define __SUPPORT_ZIP_PROTOCOL__ 1
#else
#define __SUPPORT_ZIP_PROTOCOL__ 0
#endif

// gh_drv_control.c
#if (__SUPPORT_ZIP_PROTOCOL__)
    Gh2x2xUploadZipDataToMaster(...);
#endif
```

实际结构是 `gh_demo_config.h:16-192` 的一个二选一：

```c
#ifdef CONFIG_GH3220
#include "gh3220_kernel_config.h"     // Zephyr：全部由 Kconfig 决定
#else
  ... 70 多个 demo 默认值（含 __SUPPORT_ZIP_PROTOCOL__ 等无条件 #define）...
#endif /* CONFIG_GH3220 */
... 之后才是少量 #ifndef 兜底（intercept / FIFO package 等）
```

因此：

- Zephyr 构建下，`gh_demo_config.h` 里的默认值整块被跳过，**改它不生效**，必须改 Kconfig
  或 `gh3220_kernel_config.h`；
- 裸机/EVK 构建下才是这些默认值生效；
- 末尾的 `#ifndef` 兜底两部分都适用，但只覆盖少数几个宏。

协议相关配置一览（`zephyr/Kconfig.protocol`）：

| Kconfig | 宏 | 默认 | 影响 |
|---|---|---|---|
| `GH3220_SUPPORT_PROTOCOL_ANALYZE` | `__SUPPORT_PROTOCOL_ANALYZE__` | y | 总开关；关闭后上传函数退化为空实现 |
| `GH3220_PROTOCOL_SERIAL_UART/BLE` | `__PROTOCOL_SERIAL_TYPE__` | BLE | 是否额外包 0x47/0x44 外壳（仅 demo 平台） |
| `GH3220_SUPPORT_ZIP_PROTOCOL` | `__SUPPORT_ZIP_PROTOCOL__` | y | 选 zip / 非 zip 上报 |
| `GH3220_UPLOAD_ALGO_RESULT` | `__UPLOAD_ALGO_RESULT__` | y | **定义但未被引用**，算法结果恒上传 |
| `GH3220_FIFO_PACKAGE_SEND_ENABLE` | `__FIFO_PACKAGE_SEND_ENABLE__` | n | rawdata 段是否只发 package id |
| `GH3220_PROTOCOL_SEND_TIMER_PERIOD_MS` | `__GH3X2X_PROTOCOL_SEND_TIMER_PERIOD__` | 10 | 发送定时器周期 |
| `GH3220_PROTOCOL_DATA_FIFO_LEN` | `__GH3X2X_PROTOCOL_DATA_FIFO_LEN__` | 64 | 数据 FIFO 槽数 |
| `GH3220_PROTOCOL_EVENT_FIFO_LEN` | `__GH3X2X_PROTOCOL_EVENT_FIFO_LEN__` | 16 | 事件 FIFO 槽数 |
| `GH3220_PROTOCOL_EVENT_WAITING_ACK_TIME_MS` | `__GH3X2X_PROTOCOL_EVENT_WAITING_ACK_TIME__` | 500 | 事件 ACK 超时 |
| `GH3220_PROTOCOL_EVENT_RESEND_NUM` | `__GH3X2X_PROTOCOL_EVENT_RESEND_NUM__` | 255 | 事件重传上限（255=无限） |
| `GH3220_PROTOCOL_DATA_FUNCTION_INTERCEPT` | `__GH3X2X_PROTOCOL_DATA_FUNCTION_INTERCEPT__` | 0 | **当前未接线**，见 5.3 |

### 5.2 功能裁剪如何决定"命令可见性"

`__FUNC_TYPE_HR_ENABLE__`、`__FUNC_TYPE_SPO2_ENABLE__` 等决定 `g_pstGh3x2xFrameInfo[]` 对应槽位是
指针还是 0（`gh_demo.c:821`）。协议层不看这些宏，只看运行期：

- `g_unAllFuncMode`（WORK_MODE 写入）决定哪些位"允许"
- `g_pstGh3x2xFrameInfo[i] != NULL` 决定哪些位"存在"
- `GH3X2X_GetConfigFuncMode()` 决定当前配置表实际支持哪些位

三者都满足才会真正启动。因此同一份协议代码在不同功能配置下行为不同，
**主机工具不能假设某个功能一定能启动**，必须读 0x25/0x19 判断。

补充：关闭 `__SUPPORT_PROTOCOL_ANALYZE__` 时，`gh_uprotocol.c` / `gh_zip.c` 会用
`#if` 的另一分支把 `Gh2x2xUpload*DataToMaster()` 编译成空函数——采样与算法照常运行，
只是不上报。也就是说"关协议"不影响数据链路本身，只影响出口。

### 5.3 死宏与死接口（本次核查结果）

| 名称 | 状态 | 说明 |
|---|---|---|
| `__UPLOAD_ALGO_RESULT__` | **定义但从未使用** | 算法结果永远上传，无法通过该宏关闭 |
| `__GH3X2X_PROTOCOL_DATA_FUNCTION_INTERCEPT__` | 定义，但 `Gh3x2xGetProtocolDataFunctionIntercept()` **无调用者** | 拦截功能未接线 |
| `GH3X2X_ReportIrqStatus()` | 声明 + 实现，**无调用者** | 事件上报走 FIFO 路径，此函数是旧接口 |
| `GH3X2X_ProtocolFrameIdClean()` | 声明 + 实现，**无调用者** | 帧 id 清理已无意义（帧 id 来自硬件计数） |
| `GH3X2X_GetFuncModeAvailable()` | 仅内部使用 | 对外无意义 |
| `GH3X2X_FillGsensorData()` / `GH3X2X_FillAlgoData()` | 仅在 `GH3X2X_DUMP_MODE_EN` 下编译 | dump 模式专用 |

---

## 6. 运行期配置体系

### 6.1 配置表与功能支持

- 配置表：`kernel/gh_demo_reg_array.c` 的 `g_stGh3x2xCfgListArr[]`（`{addr,val}` 数组集）
- 索引：`g_uchGh3x2xRegCfgArrIndex`，`DRV_LIB_REG_CFG_EMPTY` 表示未选
- 能力查询：`GhDrvConfigManagerGetCurFunctionSupprort()` 返回当前表支持的功能位
- 切换：`GH3x2xDemoSearchCfgListByFunc()` 找到匹配表 → 停采样 → 复位 → `GH3X2X_Init()` → 重开

切换过程会：

1. `Gh3x2xDemoSamplingControl(0xFFFFFFFF, STOP)` 停掉全部功能
2. 复位芯片、重新下载配置
3. `Gh3x2xFunctionCtrlModuleInit()` 把 `g_unDemoFuncMode` 清零
4. 重新启动原来的功能

对协议的影响：切换期间 `g_unFuncModeOpened`（协议侧）没有被清零，会与实际的
`g_unDemoFuncMode` 再次失配；主机用 0x25 查询会得到过期结果。

### 6.2 虚拟寄存器对协议可见状态的影响

`driver/src/gh_drv_config.c` 处理虚拟寄存器写时，会同步维护若干协议依赖的全局量：

| 虚拟寄存器（示例） | 影响变量 | 影响协议字段 |
|---|---|---|
| GS 使能 | `g_uchGsensorEnable` | 包头 `data_type.bit0`、GS 段长度 |
| FIFO package 模式 | `g_uchFifoPackageMode` | 包头 `package_type.bit2..4`、rawdata 段长度 |
| 采样率/AGC | `g_pstGh3x2xAgcInfo` 等 | agc 段内容 |

这意味着 **0x03 REG_RW 写虚拟寄存器可以在不改协议代码的情况下改变上行报文结构**。
工具侧修改配置后必须重新解析包头，不能假设格式固定。

### 6.3 配置切换对差分包的影响

`GH3X2X_FunctionStart()`（`driver/src/gh_drv_control.c:3002`）第一件事就是 `GH3X2X_ZipmodeInit()`，
把 `g_uchOddEvenChangeFlag` 置 1。因此**正常启停**之后，下一个包的首帧会被写成绝对帧，
配置切换（停止→复位→重下配置→重启）不会污染差分基准。

但绝对帧的判定是**两个条件**：

```c
if (g_uchOddEvenChangeFlag && g_uchGh3x2xUploadBufUse <= GH3X2X_UPLOAD_PACKAGE_HEAD_SIZE)
    /* 写绝对值 */;
else
    /* 走差分 */;
```

如果停机时还留着一个半满的包（`g_uchGh3x2xUploadStatus == HAVE_STORE_DATA`、`bufUse > 8`），
`oddEven` 虽然被置 1，`bufUse` 条件却不满足，仍然走差分——基准是"上一个配置/上一次运行"的最后一帧。
默认配置下每批 FIFO 都会以 TAIL flush 收尾，`status` 归位为 NULL，所以不会出现；
一旦出现 8.2 的下采样场景，或增加了"中途停止采样"的路径，就必须显式清空上传状态：

```c
/* 建议在 STOP / 复位前执行 */
g_uchGh3x2xUploadStatus = GH3X2X_UPLOAD_STATUS_NULL;
g_uchGh3x2xUploadBufUse = 0;
g_uchOddEvenChangeFlag  = 1;
```

> 另注：`g_stZipLastData`（历史值）不会被清空，靠 `oddEven` 的首帧绝对值覆盖，这与上面的分析一致。

---

## 7. 时序、并发与重入

### 7.1 互斥点清单

| 资源 | SDK 内部保护 | 本仓库外部保护 |
|---|---|---|
| 数据/事件 FIFO 读写指针 | 无（仅 demo 平台用信号量） | `gh3220_sdk_mutex` |
| 协议解析器单例 | 无 | `gh3220_sdk_mutex` |
| 上传状态机 | 无 | `gh3220_sdk_mutex` |
| 共享 scratch buffer | 无 | `gh3220_sdk_mutex` |
| SPI/I2C 访问 | `gh_hal_mutex`（平台实现） | 同上 |

因此**不要在其他上下文直接调用** `Gh3x2xDemoProtocolProcess()` 或 `Gh3x2x_HalSerialWriteDataToFifo()`，
必须走 app 提供的加锁入口。

### 7.2 定时器 vs 采样处理的竞态点

发送定时器与采样处理可能在不同 work 上并发（若不做全局锁）。会产生三个竞态：

1. `data_fifo` 的 wp/rp 非原子更新 → 丢帧或重复发送；
2. `Gh3x2xSerialSendTimerStart()` 在入队时被调用，可能与 `Stop()` 交错 → 定时器状态位错乱；
3. `g_uchGh3x2xUploadStatus` 在上传过程中被另一端 flush 修改。

本仓库用一把锁规避了全部三点。

### 7.3 中断上下文约束

`Gh3x2xDemoReportEvent()` 内部会操作队列并可能启动定时器，**不能在 ISR 中调用**
（app 的 ISR 只置标志/投递 work）。同理 `Gh2x2xUpload*()` 会做浮点运算和 `memcpy`，
也不适合放中断。

---

## 8. 本次分析发现的缺陷与风险

> 以下均为阅读当前代码得出，标注了具体位置；建议实测确认后再决定是否修复。

### 8.1 0x2C 功能信息包发送偏移错误

`gh_uprotocol.c:1397`：

```c
GU16 usRespondLen = GH3X2X_UprotocolPacketFormat(GH3X2X_UPROTOCOL_CMD_FUNCTION_INFO_UPDATE,
                                                 puchUploadBuf - 4, puchUploadBuf, len);
Gh3x2xDemoSendProtocolData(puchUploadBuf, usRespondLen);   // ← 应为 puchUploadBuf - 4
```

对比同文件的正常路径（`gh_uprotocol.c:1596`、`gh_zip.c:765`）都是 `SendProtocolData(puchUploadBuf - 4, ...)`。
当前写法把整包从 payload 位置开始发送，长度却是整包长度，导致：

- 主机收到的是 `[function_id][function info]...` 而不是 `AA 11 2C ...`；
- 末尾会多发 4 字节残留数据（`puchPacketPayloadArr` 有 +8 余量，不会越界）。

主机侧凑帧器找不到 `AA 11` 会丢弃这 4+len 字节；若载荷里恰好出现 `AA 11`，还可能被误当成帧头，
最终由 CRC 挡掉。**表现是"每次功能启动后第一包功能信息丢失"**，而不是崩溃。

触发条件：功能启动后的第一帧（`*punFrameCnt == 0`）时 `GH3x2xSendFunctionInfo()` 会发这个包。

### 8.2 TAIL flush 依赖帧计数，下采样时可能失效

`gh_zip.c:807` / `gh_uprotocol.c:1839`：

```c
if (uchFramePosi & GH3X2X_FRAME_POSI_TAIL)   // usFrameCnt == usFrameNum-1
    flush();
```

而 `usFrameCnt` 只在"上传了一帧"时自增（`gh_drv_control.c:2940`），
`usFrameNum` 是按 FIFO 原始数据算出的完整帧数（`:2649`）。
当 `uchDownSampleFactor > 0`（PPG/ECG 下采样，或开启 `__SUPPORT_FUNCTION_SAMPLE_RATE_MODIFY__`
后帧率高于最慢 slot）时：

```
usFrameCnt(最终) ≈ usFrameNum / (factor+1)  <  usFrameNum - 1
```

TAIL 永不成立 → 包不会在批次结束时 flush，只能等累计到 233 字节才发。后果：

1. 数据延迟变大（低速率功能可能几秒才凑满一包）；
2. 结合 8.4，跨功能/跨批次时包头与内容可能不匹配。

默认配置下 `CONFIG_GH3220_PPG_DOWNSAMP_CHANNEL_NUM=0`、采样率修改默认关闭，所以**默认不会触发**。
一旦打开这些选项，必须实测。

### 8.3 共享 scratch buffer 的隐含约束

见 4.2。任何"缓存帧指针稍后上传"的重构都会读到下一个功能的数据。
协议代码本身是同步消费，所以当前无 bug，但这是改造时最容易踩的坑。

### 8.4 `g_uchGh3x2xUploadStatus` 是全局单例

它不属于任何功能，但 `Gh2x2xUploadZipDataToMaster()` 只在 `status == NULL` 时重建包头。
如果一个功能结束时没有 flush（8.2），下一个功能的帧会被追加到**上一个功能的包头**下：

```
[func=A][len=...][frame A0][frame A1][frame B0 ...]   ← B 的数据被标成 A
```

同时 B 的首帧因为 `bufUse > 8` 不会走"绝对帧"分支，差分基准还是 A 的最后一帧。
修复方向：把 status/oddEven/bufUse 改成按功能索引的数组。

### 8.5 事件 id 与 FIFO 的边界行为

- `report_id` 是 `GU8`，每次入队自增，255→0 回绕。队列深度只有 16，同一时刻在队的 id
  互不相同，因此回绕本身安全；但**主机若缓存了旧的 id 并延迟 ACK**，回绕后可能误确认。
- 队列满时 `Gh3x2x_HalSerialWriteEventToFifo()` 直接丢弃并只打日志，**没有错误码上报**。
- 重传上限 `__GH3X2X_PROTOCOL_EVENT_RESEND_NUM__` 默认 255（无限），事件可能永久占用队首，
  阻塞后续事件（因为队首未确认时不会发送后续事件）。

### 8.6 其他代码级观察

| 位置 | 现象 | 影响 |
|---|---|---|
| `gh_demo_protocol.c:383` | 只拒绝 `len > 238`，传入的是整帧长度 | 边界写法，改动常量时先崩 |
| `gh_uprotocol.c:46`、`:1213` | `g_uchPacketCmd` 是全局，`ParseHandler` 支持一个缓冲里多命令 | 多条命令时 `g_uchPacketCmd` 被后续命令覆盖，若 handler 内部异步使用会串命令（当前都是同步） |
| `gh_uprotocol.c:1931` | 事件 id 使用 `g_uchEventReportId`（与 FIFO 里的 `g_uchGh3x2xProtocolEventReportId` 不是同一个） | `GH3X2X_ReportIrqStatus()` 一旦被启用，两套 id 会冲突 |
| `gh_zip.c:306-317` | 每次调用都分配 `uchZipRawdataArr[238]`、`uchZipAgcdataArr[238]` 等栈数组 | 深调用栈下需确认线程栈余量 |
| `gh_zip.c:306` | `puchTempFrameDataBuf[2*(238+8)]` ≈ 492B 栈 | 同上 |

---

## 9. 改造与移植检查清单

### 9.1 新增一条主机命令

1. 在 `gh_uprotocol.h` 定义 CMD 号与 payload 偏移宏；
2. 在 `GH3X2X_UprotocolCmdHandler()` 增加 `case`，写 handler；
3. handler 里**先用 `pusRespondLen` 回包**再返回，否则主机超时；
4. 若命令需要改采样状态，务必同步更新 `g_unFuncModeOpened` 与 `g_unDemoFuncMode`（两边都改）；
5. 若命令会改配置/复位，置 `g_uchResetFromProtocolFlag` 或清空 zip 历史；
6. 在 `gh_protocol_cmd.c`（RPC）或工具文档同步命令表。

### 9.2 新增一个上报字段

1. 确认数据来源在 `GH3x2xHandleFrameData()` 里已被填充（否则要在那里加）；
2. 在 `data_type` 里分配一个空位（bit7 起需要扩字节，注意兼容）；
3. 非压缩路径与 zip 路径**都要改**（两个函数是独立实现）；
4. 修 `uchOneFrameByteNum` 与结果块长度计算；
5. 主机的解析顺序必须与写入顺序严格一致。

### 9.3 接入另一套协议（GHRPC）时的注意点

- `gh_protocol/` 不在构建内，且缺 `gh_hal_*.h`/`gh_data_common.h`（见 `gh3220_ghrpc_reference.md` 14.1）；
- 数据生产者契约是 `gh_func_frame_t`，需要把 `STGh3x2xFrameInfo` 逐字段映射过去；
  GH3220 没有绝对时间戳与独立 IPD 数组，`timestamp/phy_value` 需要另行定义；
- 控制通道不能复用 `GH3X2X_UprotocolParseHandler()`，要接 `GHRPC_process()`；
- 两套协议的帧头都是 `AA 11`，**不能同时挂在同一条字节流上**，必须在构建期二选一。

### 9.4 安全改造建议（按优先级）

| 优先级 | 项 | 说明 |
|---|---|---|
| 高 | 0x2C 包偏移 | 一行修复，影响工具可见性 |
| 高 | 上传状态改成 per-function | 消除跨功能串包 |
| 中 | TAIL 改用"是否本批最后一帧"而不是计数比较 | 兼容下采样 |
| 中 | `Gh3x2xDemoReportEvent` 溢出返回错误 | 便于上层统计 |
| 中 | 事件重传上限改为有限值 | 避免永久阻塞 |
| 低 | 清理死接口/死宏 | 降低误用 |

---

## 10. 附录

### 10.1 关键结构体

```c
/* STGh3x2xFrameInfo —— 协议与驱动的唯一数据契约（gh_drv.h:663） */
typedef struct {
    GU32  unFunctionID;
    STGh3x2xFunctionInfo *pstFunctionInfo;   // 通道数/采样率/通道使能/slot 位
    STGh3x2xDownSampleInfo *pstDownSampleInfo;
    GU8   uchFuntionChnlLimit;
    GU8  *pchChnlMap;                        // slot|adc
    GU32 *punFrameRawdata;                   // 共享
    GS16 *pusFrameGsensordata;               // 共享
    STCapRawdata  *pstFrameCapdata;
    STTempRawdata *pstFrameTempdata;
    GU32 *punFrameAgcInfo;                   // 共享
    GU32 *punFrameFlag;                      // 共享，5 个
    GU32 *punIncompleteRawdata;              // 私有
    GU32 *punIncompleteChnlMapBit;           // 私有
    GU8  *puchFrameLastGain;                 // 私有
    GU32 *punFrameCnt;                       // 私有
    STGh3x2xAlgoResult *pstAlgoResult;       // 共享
    STGh3x2xAlgoResult *pstAlgoRecordResult;
} STGh3x2xFrameInfo;
```

### 10.2 功能位定义

| 位 | 宏 | 位 | 宏 |
|---|---|---|---|
| 0 | ADT | 10 | BT |
| 1 | HR | 11 | RESP |
| 2 | HRV | 12 | AF |
| 3 | HSM | 13 | TEST1 |
| 4 | FPBP | 14 | TEST2 |
| 5 | PWA | 15 | SOFT_ADT_IR |
| 6 | SPO2 | 16..18 | RS0..RS2 |
| 7 | ECG | 19 | LEAD_DET |
| 8 | PWTT | — | `GH3X2X_FUNC_OFFSET_MAX = 20` |
| 9 | SOFT_ADT_GREEN | | |

### 10.3 上报包头 8 字节速查

```
[0] function_id      （最高置位对应的功能序号）
[1] data_type        bit0 GS | bit1 ALGO | bit2 AGC | bit3 AMB | bit4 GYRO | bit5 CAP | bit6 TEMP
[2..5] channel_mask  （大端，来自 puchTagArray + unChnlEnForUserSetting）
[6] package_type     bit0 ZIP | bit1 ODD/EVEN | bit2..4 FIFO模式 | bit3..4 分包序号 | bit5 分包结束
[7] data_len
```

### 10.4 上行字段写入顺序（压缩/非压缩共用的骨架）

```
frame_id
GS(6/12B) → CAP(12B) → TEMP(12B)
rawdata   非压缩: chnl*4B / 压缩: zip 块 / FIFO 模式: 1B
agc       非压缩: chnl*4B / 压缩: zip 块
amb       当前不写
result    [len] + 子块( type + 4B )
```

### 10.5 调用图（精简）

```
gh3220_interrupt_process (app, 加锁)
└─ Gh3x2xDemoInterruptProcess                    gh_demo.c:1699
   ├─ GH3X2X_GetIrqStatus
   ├─ GH3X2X_ReadFifodata                         gh_drv_control.c:4011
   ├─ Gh3x2xDemoReportEvent                       gh_demo_protocol.c:614
   │   └─ Gh3x2x_HalSerialWriteEventToFifo        :182
   └─ Gh3x2xDemoFunctionProcess                   gh_demo.c:1405
      └─ GH3x2xFunctionProcess                    gh_drv_control.c:2962
         └─ GH3x2xGetFrameDataAndProcess          :2871
            ├─ GH3x2xHandleFrameData              :2748
            │   └─ hook → GH3X2X_AlgoCalculate
            └─ Gh2x2xUploadZipDataToMaster        gh_zip.c:290
               ├─ GH3X2X_GetRawDataDiff           gh_zip.c:60
               ├─ GH3X2X_GetDataDiff              :167
               └─ Gh3x2xDemoSendProtocolData      gh_demo_protocol.c:381
                  └─ Gh3x2x_HalSerialWriteDataToFifo  :102
                     └─ Gh3x2xSerialSendTimerStart

gh3220_protocol_receive (app, 加锁)
└─ gh3220_frame_feed
   └─ Gh3x2xDemoProtocolProcess                   gh_demo_protocol.c:589
      └─ Gh3x2xDemoOneFrameDataProcess            :404
         ├─ GH3X2X_UprotocolParseHandler          gh_uprotocol.c:1308
         │   ├─ GH3X2X_UprotocolParseBuffer       :333
         │   └─ GH3X2X_UprotocolCmdHandler        :1208
         ├─ Gh3x2xDemoSamplingControl             gh_demo.c:2634
         │   └─ GH3X2X_FunctionStart/Stop          gh_drv_control.c:3002/3084
         └─ Gh3x2xDemoSendProtocolData
```

### 10.6 相关文件速查

| 主题 | 文件:行 |
|---|---|
| 中断处理主循环 | `demo_kernel_code/kernel/gh_demo.c:1699` |
| 采样启停 | `:2634`、`:2951`、`:3137` |
| 帧组装 | `demo_kernel_code/driver/src/gh_drv_control.c:2628/2649/2748/2871` |
| 上传调用点 | `:2926` |
| 功能信息上报（缺陷） | `demo_kernel_code/module/gh_protocol/gh_uprotocol.c:1373` |
| zip 编码 | `demo_kernel_code/module/gh_protocol/gh_zip.c:60/167/290` |
| 传输 FIFO/定时器 | `demo_kernel_code/kernel/gh_demo_protocol.c:102/182/225` |
| 事件上报 | `:614` |
| 算法钩子 | `demo_kernel_code/kernel/gh_demo_hook.c:163/387` |
| 共享 scratch | `demo_kernel_code/kernel/gh_demo.c:297` |
| 应用加锁入口 | `l-evk-t2-gh3220/app_cardiff_a/src/gh3220_framework.c:67` |
| 应用协议适配 | `l-evk-t2-gh3220/app_cardiff_a/src/gh3220_protocol_user.c` |
