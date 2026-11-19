# 架构说明

## 目标

同时满足轻量与现代化：Tauri 使用系统 WebView，前端 Vue 3 负责仪器风交互；Rust 负责协议、时序、扫描与数据流，原始帧统一采集后推送 UI。

## 分层

```
Vue 3 (UI / 图表 / 扫描 / 工作区 / 工具箱)
   │  Tauri commands + events（帧带单调 seq）
   ▼
src-tauri (会话 Actor / 轮询 / 扫描 / 网络助手 / 工具箱 / 定义与写入)
   │  纯 Rust API
   ▼
crates/modbus-core (PDU / 帧 / 值解析 / 模拟引擎，无 I/O 依赖)
```

`modbus-core` 不依赖 tokio、serialport、Tauri 或文件系统，可独立执行 `cargo test -p modbus-core`（39 个测试，含属性测试）。

## 链路 × 帧格式

| Link | Framing | 说明 |
|---|---|---|
| Serial | RTU / ASCII | ASCII 7/8 数据位，CRLF 终止；RTU t3.5 仅用于超时排空 |
| TCP | MBAP / RTU / ASCII | MBAP 事务 ID 匹配；RTU/ASCII over TCP 按流式期望长度读取 |
| UDP | MBAP / RTU / ASCII | connected socket 过滤来源；一报文一数据报 |

已实现的关键细节：

- RTU：先读 2 字节，异常帧 5 字节；读响应 `5 + byteCount`，写响应 8 字节；超时后 t3.5 排空并重同步
- ASCII：`:` 起始、`\n` 结束，最大 513 字符，LRC 校验
- MBAP：事务 ID/协议 ID/长度校验，长度包含 unit id + PDU；TCP/UDP 事务按 TID 过滤陈旧响应
- 写操作默认不自动重试；读支持重试；广播 ID 0 只写、不等待响应
- 串口使用专用阻塞 IO 线程 + tokio oneshot 桥接，读超时 15ms 循环并受整体 deadline 约束
- 局域网扫描使用 CIDR × 端口列表 + 有界并发，单次最多 65,536 次探测；仅 502 端口可选发送 FC03 验证 Modbus；可被动读取 Banner 并标注常见服务

## 会话 Actor

每个主站会话一个 tokio 任务，独占链路并串行化事务：

```
loop {
  select! {
    command = rx.recv() => handle(command),       // 写/单次读/定义 CRUD/轮询开关
    _ = sleep_until(next_due) => run_due_polls(),  // 周期读取
  }
}
```

- 写命令优先于周期读取
- 每次事务将 TX 与 RX（或超时/错误帧）作为一个 `session:frames` 批量事件推送
- 数据事件同时返回 `words` / `formatted` / `numeric`，Rust 是解析唯一事实来源
- 点位按 `原始值 × 缩放 + 偏置` 格式化；告警在成功读取后按规则去抖，触发/恢复通过 `alert:event` 推送。每条规则带描述和等级（info/warning/critical），并可带可选恢复条件与恢复保持时间：未配置恢复条件时触发条件消失即恢复；配置后必须触发条件消失且恢复条件连续成立达到等待时间才恢复；事件携带恢复操作符/阈值/等待时间供记录与导出
- 传输错误按配置重试（读）并触发 1s→2s 退避重连
- 扫描引擎独立于会话运行，使用自己的链路；串口扫描要求端口空闲
- 从站模拟器由独立 TCP/串口服务任务持有一个 `Simulator`；新建从站携带空白稀疏映射，未配置点位时拒绝启动监听；点位发生器在监听前写入初值，并按各自刷新周期更新保持/输入寄存器或线圈/离散输入。映射更新会重建模拟器状态，避免已删除地址残留旧值。

## 网络助手

`network.rs` 提供独立于 Modbus 会话的原始 TCP/UDP 工具。`NetworkManager` 以会话 ID 为键保存多个并发链路，每个链路一个后台任务，通过命令通道发送字节：

| 模式 | 行为 |
|---|---|
| TCP 服务端 | 监听并为每个客户端启动读写任务，发送广播到全部客户端 |
| TCP 客户端 | 主动连接远端，双向原始字节 |
| UDP 服务端 | 绑定端口，回复最近一次数据来源 |
| UDP 客户端 | 本地绑定后连接远端 UDP |
| TCP 中继 | 监听一个本地客户端并转发到上游 TCP；客户端接入前也可手动向上游发送 |

状态与收发分别通过 `network:state`、`network:data` 推送，载荷带 `sessionId`，前端按会话归档日志并支持 CSV 导出。新建网络会话按模式只展示必要字段：客户端只有目标地址/端口，服务端只有监听地址/端口，中继同时需要本地监听与上游参数。

## 工具箱

`src-tauri/src/tools/` 是与 Modbus 会话、网络助手并列的本地工具后端。`ToolsManager` 负责有状态任务（Ping、持续 Ping、路由跟踪、Hash、搜索）的任务 ID、取消标志与事件；DNS、Telnet、HTTP、ARP 使用按请求返回的命令。`FileServerManager` 与 `MqttManager` 独立持有服务/连接生命周期，并在窗口关闭时停止。

| 工具 | 实现要点 |
|---|---|
| 批量 Ping | Windows `IcmpSendEcho`，IPv4，主机/CIDR 展开，最多 256 主机；按主机汇总丢包与 min/avg/max；前端默认显示存活 Tab，可切换全部结果 |
| 持续 Ping | 单目标按间隔持续 ICMP 探测，逐采样推送 `latencySample`，前端用 uPlot 绘制延迟曲线并以红色竖线标记丢包 |
| 路由跟踪 | 同一 ICMP 封装递增 TTL，逐跳发射 `tracerouteHop`，可选反向解析 |
| ARP | 原生使用 ipconfig 枚举网卡 + `SendARP`；增强模式通过 `libloading` 运行时加载 Npcap `wpcap.dll`，发送/捕获 ARP 请求并按 IP 聚合多个 MAC 检测冲突；需要管理员权限 |
| DNS | `hickory-resolver` 0.24，系统或自定义 UDP/TCP 服务器，支持 A/AAAA/CNAME/MX/TXT/NS/SOA/SRV/PTR/CAA，缓存关闭 |
| Telnet | TCP 建链 + Banner/可选 ASCII/HEX 发送；不做 IAC 协商 |
| HTTP | `reqwest` + native-tls，方法/请求头/超时/重定向/CA；支持 JSON、x-www-form-urlencoded、multipart 文件上传（≤32MB），响应体限制 2MB；不提供跳过证书校验 |
| Web 文件服务 | `axum` 内置 HTTP 服务，共享根目录、监听地址与端口（默认 80）；文件列表、下载流式响应、多文件流式上传、重名自动后缀、文件名消毒与路径穿越防护；请求日志内存保存最近 200 条 |
| MQTT | `rumqttc` + native-tls；多 Broker 并行，支持 MQTT 3.1.1/5、用户名密码、可选 CA；每 Broker 独立事件循环与重连；消息日志在 UI 侧按 Broker 保存最近 2000 条 |
| Hash | 单次分块读取同时计算 MD5/SHA-1/SHA-256/SHA-512/CRC32，可取消；前端在任务 ID 返回前缓冲工具事件，避免小文件瞬间完成导致进度停在 0% |
| 文件夹搜索 | walkdir + globset + regex + encoding_rs，跳过二进制、隐藏目录与常见构建目录；结果上限与批量事件 |
| 美化/校验/转换 | 纯前端；Prettier standalone 按需加载，XML 使用 xml-formatter，YAML 使用 `yaml` 解析并输出错误行列 |

事件统一为 `tool:event`，载荷用 `event` 字段区分 `pingProgress`、`pingResult`、`pingDone`、`latencySample`、`latencyDone`、`tracerouteHop`、`tracerouteDone`、`hashProgress`、`hashDone`、`searchProgress`、`searchHits`、`searchDone`；高频结果在 Rust 侧批量推送。MQTT 使用独立的 `mqtt:state` 与 `mqtt:message` 事件。ARP/Ping/持续 Ping/路由跟踪为 Windows IPv4 实现，非 Windows 返回明确错误。Web 文件服务通过命令轮询快照，不占用事件总线。

## 配置持久化

Tauri 进程内的会话/定义存储仍由 Rust 持有；前端在非演示模式把可恢复配置（会话参数、定义、点位、别名、告警规则、模拟器发生器）写入 WebView localStorage，并在后端工作区为空时逐个重建。重建时固定 `autoConnect: false`，因此应用重启后所有会话默认断开，由用户手动连接。网络助手参数使用独立的 localStorage 键；界面主题、语言、表格偏好由 settings store 保存。告警历史与收发日志属于运行时数据，不持久化。

全局配置导入/导出把 settings、工作区会话、网络助手会话、工具箱选择与非敏感工具默认参数封装为 `kind: "modbus-tool-config"`、`version: 1` 的 JSON。导入文件先校验 kind/version/shape，再经确认对话框执行：替换工作区会话（重建后保持断开）、替换网络会话（停止现有链路）、应用界面偏好、工具箱选择与默认参数并清空当前告警记录；扫描任务、工具运行结果/日志属于临时运行态，不随配置导入导出。MQTT Broker 配置与订阅同时保存在独立的 `mt.mqtt.v1`，并随全局配置导出/导入（不含密码）。

## 扫描

| 类型 | 实现 |
|---|---|
| 从站 ID | FC01–04 顺序探测 0–255（ID 0 跳过），异常响应视为设备存在 |
| 寄存器地址 | 分块读取，记录可读块、非零数量与首个非零地址 |
| 波特率 | 串口逐波特率重开端口探测，报告响应与延时 |
| 局域网 / 端口 | IPv4 CIDR（≤ /16）× 端口列表（≤ 256），总探测 ≤ 65,536；并发 TCP、被动 Banner、服务名；502 可选 Modbus 探测 |

事件：`scan:progress`（done/total/found/phase/finished）、`scan:result`（逐条）、`scan:done`（汇总/错误）。取消通过 `AtomicBool` 在探测间检查。`ScanManager` 以 scanId 为键可同时运行多个扫描任务；前端也按 scanId 将进度和结论路由到各自的扫描会话，局域网扫描自身还使用有界并发。

## 前端结构

- `lib/ipc.ts`：真实 Tauri 与浏览器 Demo 的统一接口
- `lib/demo-bridge.ts`：无 Rust 时的演示数据流（轮询、写入、帧、四类扫描、曲线）
- `stores/sessions.ts`：会话、定义、快照、迷你趋势历史、曲线时间序列、工作区恢复与自动保存
- `stores/alerts.ts`：告警触发/恢复记录、活动计数与未读状态
- `stores/network.ts`：多网络会话、参数持久化、按会话归档收发日志
- `stores/scans.ts`：多扫描任务、按后端 scanId 路由进度与结论
- `stores/toolbox.ts`：工具箱当前选择与全局配置导出/恢复
- `stores/mqtt.ts`：MQTT Broker 配置（不含密码）、订阅、连接状态与消息日志；配置持久化在独立 localStorage 键
- `components/tools/`：网络/开发工具分组、二级工具导航与具体工具面板
- `components/chart/`：uPlot 实时曲线（分类色板经 dataviz 校验）、跨线提示、图例、数据表
- `components/scan/`：扫描工作台，左侧任务会话、右侧进度与结论
- `components/network/`：网络工作台，多 TCP/UDP/中继会话，原始收发与日志导出
- `components/alerts/`：告警工作台，左侧复用工作区会话、右侧按会话显示记录
- 设计系统：OKLCH tokens、毛玻璃仅用于静态 chrome、等宽数值、motion-v + CSS 微动效

## IPC 事件

| 事件 | 载荷 |
|---|---|
| `session:status` | 状态、轮询开关、TX/RX/错误/延时 |
| `session:data` | 定义快照、原始 words、格式化值、数值、延时 |
| `session:frames` | TX/RX 帧（seq、方向、原始字节、解码字段、延时、状态） |
| `debug:data` / `debug:state` | 原始调试收发字节与链路状态 |
| `alert:event` | 告警触发/恢复（规则、点位、阈值、当前值、时间） |
| `network:state` / `network:data` | 网络助手链路状态与原始收发 |
| `workspace:updated` | 会话/定义配置快照 |
| `scan:progress` | 扫描类型、进度、发现数、阶段、错误 |
| `scan:result` | 单条扫描结果 |
| `scan:done` | 扫描汇总与错误 |
| `tool:event` | 网络/开发工具进度、结果、完成或错误（按 event/taskId 区分） |
| `mqtt:state` | Broker 连接状态与错误详情 |
| `mqtt:message` | 订阅收到的 MQTT 主题、QoS、retain 与文本/HEX 载荷 |

命令包含会话创建/轮询、定义 CRUD、单次读、写寄存器、原始调试、网络助手、四类扫描与取消、串口枚举、定义导入/导出文件读写，以及工具箱的 DNS/Telnet/HTTP/Ping/持续 Ping/路由跟踪/ARP/Hash/搜索、Web 文件服务启停/状态与 MQTT 启停/发布/订阅命令。

## 里程碑状态

- P1 脚手架 / 设计系统 / Demo / Windows CI — 完成，CI 已通过
- P2 协议核心与测试 — 完成
- P3 串口/TCP/UDP 传输 + 主站 Actor — 完成（串口未用真实硬件验证）
- P4 Tauri 命令/事件 + 工作区持久化 — 完成（localStorage 恢复，默认断开）
- P5 主站 UI 交互 — 定义编辑、写入、基址、检查器、点位缩放/告警完成
- P6 报文导出 + CSV 日志 — 报文/告警/调试/网络日志导出完成；周期数据日志待实现
- P7 从站模拟器 — TCP/串口服务端、空白稀疏映射、映射增删改、逐点位发生器与初值种子完成
- P8 四类扫描 — 完成；扫描工作台支持多任务并行与会话化结论
- P9 uPlot 曲线 + Windows 发布 — 曲线完成；Windows 安装包 artifact 已验证
- P10 网络助手 — 多会话工作台、TCP/UDP 客户端/服务端、TCP 中继、按模式动态参数、日志与参数持久化完成
- P11 页面化工作台 — 网络助手、设备扫描、告警记录由模态框迁移为左侧会话 + 右侧内容的全页面
- P12 工具箱第一批 — 网络诊断、IP/端口扫描与开发工具完成
- P13 工具箱第二批 — 持续 Ping 曲线、YAML 校验、HTTP 表单/文件上传、Web 文件服务与 MQTT 完成；CAN（第三批）待实现

## 后续优化

- 帧事件按 ~100ms 聚合、UI 侧回查序号（当前按事务批量）
- 按轮询周期持续写入的 CSV 数据日志
- 从站模拟 UDP 服务端与自定义映射容量
- 真实串口设备验证与 macOS/Linux CI 矩阵
