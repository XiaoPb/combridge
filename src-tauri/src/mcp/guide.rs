//! 工具使用指南：前置条件、注意事项、典型调用与错误码。
//!
//! 为什么放在服务端而不是只写进文档：模型拿到 `tools/list` 只有一句话描述，
//! 不知道"这个工具之前必须先做什么"、"失败该看哪个错误码"。这里把这类隐性
//! 知识做成**可查询的数据**（`combridge_tools_guide`），模型在动手前查一次即可，
//! 不必靠试错。
//!
//! `docs/mcp-guide.md` 是本表的人读版本；两者不一致时以本表为准。

/// 一条工具说明。
pub struct ToolGuide {
    /// 工具名，与 `tools/list` 中一致
    pub name: &'static str,
    /// 所属域：core | serial | ble | gh3036 | waveform | protocol | ota
    pub domain: &'static str,
    /// 一句话职责
    pub summary: &'static str,
    /// 调用前必须满足的条件（缺一项就会失败或得到无意义结果）
    pub prerequisites: &'static [&'static str],
    /// 使用注意点（坑、默认值、副作用）
    pub notes: &'static [&'static str],
    /// 最小可用调用示例
    pub example: &'static str,
    /// 风险等级：read（只读）| write（改设备/状态）| destructive（不可逆）| critical（烧录/擦除）
    pub risk: &'static str,
}

/// 全局约定：与具体工具无关、但每次调用都要遵守的规则。
pub const CONVENTIONS: &[&str] = &[
    "租约（lease）：凡是对设备产生副作用的操作都要带 lease_id。打开端口/连接 BLE/首次调用 GH3036 时自动签发，\
同一 owner 重复调用沿用旧租约；被别的 owner 占着会返回 *_BUSY，确认对方已放弃后再 force=true 抢占。",
    "owner：默认 ai-client，建议填你的客户端名（如 claude-code），便于在 GUI 的租约面板里认出是谁占着设备。",
    "档位：standard（默认）下写设备、产测、OTA 需人工在 GUI 弹窗确认，OTA start 直接 TIER_BLOCKED；\
dev（研发档）免确认。改档位在设置页，改完立即生效、无需重启服务。",
    "游标：serial_read / serial_expect / ble_gatt(notifications) 都是游标语义。第一次省略 since 读历史，\
之后必须回传上次的 next_cursor，否则会重复读到同一批数据。",
    "编码：文本用 encoding=text；二进制（固件帧、寄存器值）用 encoding=hex，可写 \"AA BB 01\" 也可写 \"AABB01\"。",
    "错误处理：返回 isError=true 时先看 structuredContent.error_code，再看 error 文案与 suggestion。\
拿不到线索时调 combridge_about(action=logs) 读应用日志尾部。",
    "不要猜地址与 UUID：串口名来自 combridge_ports，BLE 地址来自 combridge_ble_scan，\
GATT 特征 UUID 来自 combridge_ble_gatt(action=discover_services)。",
];

/// 错误码说明。按 error_code 查，避免把"参数错"当成"设备坏"。
pub const ERROR_CODES: &[(&str, &str)] = &[
    ("INVALID_ARGUMENT", "参数缺失或取值不在允许范围，按 suggestion 修正后重试，不要重试原参数。"),
    ("PORT_NOT_FOUND", "端口不存在或已被拔掉，重新 combridge_ports 确认。"),
    ("PORT_BUSY", "端口被其他客户端或 GUI 占用，检查 active_leases 或 force=true 抢占。"),
    ("PORT_NOT_OWNED_BY_MCP", "端口由 GUI 打开，MCP 不允许关闭。需要独占请先在 GUI 里关闭。"),
    ("LEASE_REQUIRED", "缺少 lease_id，先做对应的 open / connect / 首次 GH3036 调用取得租约。"),
    ("LEASE_EXPIRED", "租约已过期，重新 open 或 connect 换新租约。"),
    ("BLE_NOT_CONFIGURED", "BLE 后端未初始化，先在 GUI 里启用 BLE 或检查蓝牙适配器。"),
    ("BLE_BUSY", "设备被其他客户端连接着，确认后可 force=true 抢占。"),
    ("BLE_CONNECT_FAILED", "连接失败：设备可能已被系统配对占用、距离过远或未广播。"),
    ("CHAR_NOT_FOUND", "特征 UUID 不对或未先 discover_services，先发现服务再按返回的 UUID 引用。"),
    ("GH3036_INIT_FAILED", "协议栈初始化失败，先确认应用运行正常；重试 action=init。"),
    ("GH3036_BUSY", "GH3036 被其他客户端持有（单芯片会话），可 force=true 抢占。"),
    ("GH3036_CHANNEL_FAILED", "通道配置失败，串口名/BLE 地址与特征 UUID 是否有效。"),
    ("GH3036_RPC_FAILED", "RPC 执行失败：通道没配好、设备没响应或参数格式不对（见 action=catalog）。"),
    ("GH3036_NO_TX_CHANNEL", "还没配 TX 通道，先 action=set_channel。"),
    ("TIER_BLOCKED", "当前档位禁止该操作（OTA 烧录在 standard 档被拦），改档或走 GUI。"),
    ("OTA_CONFIRM_REQUIRED", "缺少 confirm_token，先 dry-run 拿到令牌再带回来。"),
    ("OTA_CONFIRM_INVALID", "令牌无效/过期/设备或固件不匹配，重新 dry-run。"),
    ("CONFIRM_DENIED", "用户在确认弹窗里拒绝了，不要立刻重试同一操作。"),
    ("RATE_LIMITED", "超过档位限流，降低调用频率。"),
];

/// 全部工具指南（按域聚拢，便于顺序阅读）。
pub const TOOL_GUIDES: &[ToolGuide] = &[
    ToolGuide {
        name: "combridge_tools_guide",
        domain: "core",
        summary: "查工具使用方式、前置条件、注意事项与错误码。动手前先查一次，比试错快。",
        prerequisites: &["无。这是唯一无需任何前置的工具。"],
        notes: &[
            "不传参数返回全部工具；传 tool 只返回该工具的详细说明（省 token）。",
            "返回值与本文件（docs/mcp-guide.md）同源，可作为权威依据。",
            "第一次连上服务时建议调一次不带参数的版本，把全局约定与错误码表读进上下文。",
        ],
        example: r#"{"tool": "combridge_gh3036_rpc"}"#,
        risk: "read",
    },
    ToolGuide {
        name: "combridge_about",
        domain: "core",
        summary: "服务概况（版本/档位/已连接设备/工具域），以及读应用日志尾部。",
        prerequisites: &["无。"],
        notes: &[
            "action=summary（默认）用于会话开始时确认服务版本与档位。",
            "action=logs 是排障主力：任何工具返回错误而 suggestion 不够具体时，读日志尾部往往能直接看到原因。",
            "lines 默认 100，上限 500。",
        ],
        example: r#"{"action": "logs", "lines": 200}"#,
        risk: "read",
    },
    ToolGuide {
        name: "combridge_ports",
        domain: "serial",
        summary: "列出系统串口及其占用状态。",
        prerequisites: &["无。"],
        notes: &[
            "任何串口操作前先调一次，拿准确端口名（COM3 这类），不要凭印象猜。",
            "open=true 表示已被打开（可能是 GUI），此时仍能读，但 close 会被拒绝。",
            "lease 字段能看出被哪个 AI 客户端占着。",
        ],
        example: r#"{}"#,
        risk: "read",
    },
    ToolGuide {
        name: "combridge_serial_open",
        domain: "serial",
        summary: "打开串口并取得租约。",
        prerequisites: &["端口名来自 combridge_ports。"],
        notes: &[
            "返回的 lease_id 必须保存，write / close 都要带。",
            "返回的 cursor 是后续 read 的起点；already_open=true 时只能读不能关。",
            "默认 115200/8N1；AT 模块常见 115200，调试口可能是 921600，先确认再开。",
            "端口被其他 AI 客户端持有时返回 PORT_BUSY，force=true 会抢断对方。",
        ],
        example: r#"{"port_name": "COM6", "baud_rate": 115200, "owner": "claude-code"}"#,
        risk: "write",
    },
    ToolGuide {
        name: "combridge_serial_close",
        domain: "serial",
        summary: "关闭串口并释放租约。",
        prerequisites: &["持有该端口的 lease_id，且端口由 MCP 打开。"],
        notes: &[
            "GUI 打开的端口会被拒绝（PORT_NOT_OWNED_BY_MCP）——这是保护，不是 bug。",
            "用完设备就关，否则下一个客户端会被 PORT_BUSY 挡住。",
        ],
        example: r#"{"port_name": "COM6", "lease_id": "<open 返回的 lease_id>"}"#,
        risk: "write",
    },
    ToolGuide {
        name: "combridge_serial_write",
        domain: "serial",
        summary: "向串口写数据（文本或十六进制）。",
        prerequisites: &["已 open 并持有该端口的 lease_id。"],
        notes: &[
            "发 AT 指令记得 append_crlf=true（多数模块要 \\r\\n 才响应）。",
            "二进制协议用 encoding=hex，例如 {\"data\": \"AA 55 01\"}。",
            "write 成功只代表写进了系统缓冲，不代表设备收到；要确认响应就紧跟 combridge_serial_expect。",
        ],
        example: r#"{"port_name": "COM6", "lease_id": "...", "data": "AT+RST", "append_crlf": true}"#,
        risk: "write",
    },
    ToolGuide {
        name: "combridge_serial_read",
        domain: "serial",
        summary: "按游标读取串口接收数据。",
        prerequisites: &["无（读取不需要租约）。"],
        notes: &[
            "首次省略 since 会吐缓冲区全部历史，容易一次拿到几万字节，建议先 expect 或只看尾部。",
            "之后必须回传 next_cursor，否则重复读同一段。",
            "缓冲区上限 1MB，久不读会丢数据并置 truncated=true。",
            "lagged=true 表示内部消费滞后，区间内可能有缺口，需要重读或延长等待。",
        ],
        example: r#"{"port_name": "COM6", "since": 1024, "encoding": "hex"}"#,
        risk: "read",
    },
    ToolGuide {
        name: "combridge_serial_expect",
        domain: "serial",
        summary: "阻塞等待串口出现指定内容（服务端等待，避免反复轮询）。",
        prerequisites: &["端口已打开（由本会话或 GUI 均可）。"],
        notes: &[
            "这是与设备交互的首选方式：write 之后立刻 expect，比 read 轮询稳。",
            "默认只看 since 之后的新数据；要匹配等待前已收到的内容，传 read 拿到的 since。",
            "超时返回 matched=false 但 buffer 里带全部已收文本——直接看 buffer 判断设备到底回了什么。",
            "regex=true 时 pattern 是正则，注意转义；普通文本保持 regex=false 更快。",
            "disconnected=true 表示等待期间设备掉线，不要去重试 expect。",
        ],
        example: r#"{"port_name": "COM6", "pattern": "OK", "timeout_ms": 2000}"#,
        risk: "read",
    },
    ToolGuide {
        name: "combridge_devices",
        domain: "ble",
        summary: "汇总 BLE 后端状态、当前连接、App 设备快照与全部租约。",
        prerequisites: &["无。"],
        notes: &[
            "多设备场景开工前先调一次，确认哪条链路归你、哪些被别人占着。",
            "ble_configured=false 时所有 BLE 工具都不可用（BLE_BUSY/NOT_CONFIGURED），先去 GUI 启用。",
            "subscriptions 列出已订阅特征，判断通知链路是否已通。",
        ],
        example: r#"{}"#,
        risk: "read",
    },
    ToolGuide {
        name: "combridge_ble_scan",
        domain: "ble",
        summary: "扫描 BLE 设备（阻塞到时间结束）。",
        prerequisites: &["BLE 后端已配置（combridge_devices 里 ble_configured=true）。"],
        notes: &[
            "默认 3000ms；设备不广播时需要更长或先让设备进入广播态。",
            "已连接的设备通常不再出现在扫描结果里——用 combridge_devices 看已连接列表。",
            "扫描期间独占适配器，会让正在进行的连接操作变慢，扫完就停（stop=true）。",
            "is_connectable=false 的设备不要尝试 connect。",
        ],
        example: r#"{"duration_ms": 5000}"#,
        risk: "read",
    },
    ToolGuide {
        name: "combridge_ble_connect",
        domain: "ble",
        summary: "BLE 连接管理：connect / disconnect / list。",
        prerequisites: &["地址来自 combridge_ble_scan 或 combridge_devices。"],
        notes: &[
            "connect 返回 lease_id，后续 write / subscribe / disconnect 都要带。",
            "连上只是链路建立，读写特征前必须 discover_services。",
            "被动断连后设备可能不再广播：此时直接再 connect 同一地址即可，服务端会从系统\"已连接设备\"列表里找回，无需复位器件。",
            "BLE_BUSY 说明有别的客户端连着，force=true 会抢占，确认对方不用了再抢。",
        ],
        example: r#"{"action": "connect", "address": "AA:BB:CC:DD:EE:FF", "owner": "claude-code"}"#,
        risk: "write",
    },
    ToolGuide {
        name: "combridge_ble_gatt",
        domain: "ble",
        summary: "GATT 操作：发现服务/特征、读写、订阅、读通知、MTU、RSSI。",
        prerequisites: &[
            "已 connect 并持有 lease_id（write/subscribe/unsubscribe/mtu 需要）。",
            "发现特征前先 discover_services。",
        ],
        notes: &[
            "标准顺序：discover_services → discover_characteristics(service_uuid) → subscribe(char_uuid) → notifications 轮询。",
            "UUID 用返回的原字符串，不要自己补全 128 位；服务端已做大小写与连字符规范化。",
            "notifications 是游标读取：首次可省略 since，之后必须回传 next_cursor。",
            "高频通知（如 sensor 流）用 without_response 写更接近真机行为。",
            "MTU 在 Windows 后端可能无法真正协商，返回值以实际为准，别按 247 假设单包长度。",
        ],
        example: r#"{"action": "subscribe", "address": "AA:BB:CC:DD:EE:FF", "char_uuid": "00000003-0000-1000-8000-00805f9b34fb", "lease_id": "..."}"#,
        risk: "write",
    },
    ToolGuide {
        name: "combridge_gh3036_rpc",
        domain: "gh3036",
        summary: "GH3036 芯片协议：状态/初始化/配通道/命令目录/执行 RPC。",
        prerequisites: &[
            "已确定连接方式：串口名（combridge_ports）或 BLE 地址+写特征 UUID（combridge_ble_scan/ble_gatt）。",
            "首次调用任意 action 会自动签发 GH3036 租约。",
        ],
        notes: &[
            "协议栈在应用启动时已自动初始化（带 CSV 偏好），一般不需要手动 action=init；只有返回 GH3036_INIT_FAILED 时才补一次，init 幂等。",
            "顺序：set_channel（direction=both）→ catalog（拿 command_key）→ rpc。没配 TX 通道就发 RPC 会返回 GH3036_NO_TX_CHANNEL。",
            "command_key 必须来自 catalog，不要凭经验编；params 按目录里的顺序，十六进制可带 0x。",
            "chunk 的十六进制值不要加空格，服务端按字符串解析。",
            "GH3036 是单芯片会话：其他客户端持租约时返回 GH3036_BUSY，force=true 可抢占。",
            "CSV 是否落盘由 GUI 偏好决定（enabled 与输出目录），MCP 不修改偏好；采了数据却没有文件时先查偏好。",
        ],
        example: r#"{"action": "set_channel", "channel_type": "ble", "device_id": "AA:BB:CC:DD:EE:FF", "characteristic_uuid": "00000002-0000-1000-8000-00805f9b34fb", "direction": "both"}"#,
        risk: "write",
    },
    ToolGuide {
        name: "combridge_gh3036_factory_test",
        domain: "gh3036",
        summary: "GH3036 产测：一次 run 直接拿结论（pass / failed_item / items / csv_path）。",
        prerequisites: &[
            "GH3036 通道已配置（先 combridge_gh3036_rpc action=set_channel）。",
            "卡控配置目录可用（或先 validate_config 看配置文件是否齐全）。",
        ],
        notes: &[
            "run 内部完成 启动→轮询→等待→汇总，不要自己拆步骤轮询 status。",
            "max_wait_s 默认 180、上限 600；超时返回 status=running + job_id，用 action=status 续等（job 不会丢）。",
            "返回 waiting_environment_switch 时必须人工切环境再 action=continue，重复 run 会从头开始。",
            "stop 只中止当前 job，不释放租约。",
            "validate_config 不连设备，适合测前先自查卡控文件。",
        ],
        example: r#"{"action": "run", "max_wait_s": 240}"#,
        risk: "write",
    },
    ToolGuide {
        name: "combridge_waveform",
        domain: "waveform",
        summary: "波形缓冲与解析（与 GUI 波形面板共用同一份数据）。",
        prerequisites: &["除 list 外都需要 buffer_id，先 create 或 list。"],
        notes: &[
            "典型链路：create → configure_parser → parse(串口整段文本) → read。",
            "parse 的 data 可含多行，每行一条样本；解析失败的行会计入 rejected 并在 errors 里给前 5 条原因。",
            "read 默认 200 行、上限 5000；环形缓冲只保留最近 capacity 行。",
            "写进缓冲的数据 GUI 立即可见，反之亦然——不要假设 MCP 独占。",
        ],
        example: r#"{"action": "parse", "buffer_id": "ppg", "data": "1.2,3.4,5.6\n1.3,3.5,5.7"}"#,
        risk: "write",
    },
    ToolGuide {
        name: "combridge_protocol",
        domain: "protocol",
        summary: "Lua 协议插件：加载/启停/绑定设备。",
        prerequisites: &["plugin_id 来自 list；load 需要 Lua 脚本绝对路径。"],
        notes: &[
            "disable 只是停止响应事件，脚本与 hook 仍占资源；彻底移除用 unload。",
            "bind 到设备后插件才会处理该设备的数据，未绑定等于没生效。",
            "load 失败时 plugin.error 会带 Lua 报错行号，按行号改脚本。",
        ],
        example: r#"{"action": "bind", "plugin_id": "my_proto", "device_id": "COM6"}"#,
        risk: "write",
    },
    ToolGuide {
        name: "combridge_ota",
        domain: "ota",
        summary: "BLE OTA 固件升级（默认 dry-run，两段式确认）。",
        prerequisites: &[
            "目标设备已 BLE 连接（部分协议需要先 connect，部分自带连接，看 preflight 结果）。",
            "固件包路径为本地绝对路径。",
        ],
        notes: &[
            "start 不带 confirm 永远只做 dry-run：返回 plan、warnings 与 confirm_token，不会烧录。",
            "令牌一次性、5 分钟过期，且绑定 目标设备+固件路径+镜像 sha256——换设备或换包必须重新 dry-run。",
            "standard 档下 start 直接 TIER_BLOCKED，需在 GUI 操作或切 dev 档。",
            "升级过程中不要 disconnect 设备，也不要让机器休眠。",
            "validate 只校验包、不连设备，是最便宜的预检；preflight 会连设备读版本，能提前发现型号不匹配。",
        ],
        example: r#"{"action": "start", "address": "AA:BB:CC:DD:EE:FF", "firmware_path": "D:/fw/app.zip"}"#,
        risk: "critical",
    },
];

/// 按名精确查找（大小写不敏感，允许省略 `combridge_` 前缀）。
pub fn find(name: &str) -> Option<&'static ToolGuide> {
    let needle = name.trim().to_ascii_lowercase();
    let needle = needle.strip_prefix("combridge_").unwrap_or(&needle);
    TOOL_GUIDES.iter().find(|g| {
        let candidate = g.name.strip_prefix("combridge_").unwrap_or(g.name);
        candidate.eq_ignore_ascii_case(needle)
    })
}

/// 按域过滤。
pub fn by_domain(domain: &str) -> Vec<&'static ToolGuide> {
    let needle = domain.trim().to_ascii_lowercase();
    TOOL_GUIDES
        .iter()
        .filter(|g| g.domain == needle)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_tool_has_a_guide() {
        // 工具清单来自 server.rs 的 #[tool] 注册：新增工具却忘了写指南时，
        // 模型会在工具列表里看到一个沉默的工具，这个测试负责拦住。
        let registered = [
            "combridge_about",
            "combridge_ports",
            "combridge_serial_open",
            "combridge_serial_close",
            "combridge_serial_write",
            "combridge_serial_read",
            "combridge_serial_expect",
            "combridge_devices",
            "combridge_ble_scan",
            "combridge_ble_connect",
            "combridge_ble_gatt",
            "combridge_gh3036_rpc",
            "combridge_gh3036_factory_test",
            "combridge_waveform",
            "combridge_protocol",
            "combridge_ota",
            "combridge_tools_guide",
        ];
        for name in registered {
            assert!(find(name).is_some(), "缺少工具指南: {name}");
        }
        assert_eq!(TOOL_GUIDES.len(), registered.len(), "指南数量与注册工具数不一致");
    }

    #[test]
    fn find_tolerates_prefix_and_case() {
        assert!(find("GH3036_RPC").is_some());
        assert!(find("combridge_ble_scan").is_some());
        assert!(find("nope").is_none());
    }
}
