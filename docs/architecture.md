# 架构说明

## 目标

同时满足轻量与现代化：Tauri 使用系统 WebView，前端 Vue 3 负责仪器风交互；Rust 负责协议、时序、扫描与数据流，原始帧统一采集后推送 UI。

## 分层

```
Vue 3 (UI / 图表 / 扫描 / 工作区视图)
   │  Tauri commands + events（帧带单调 seq）
   ▼
src-tauri (会话 Actor / 轮询 / 扫描 / 定义与写入)
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
- 局域网扫描使用 CIDR + 有界并发，可选发送 FC03 验证 Modbus

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
- 传输错误按配置重试（读）并触发 1s→2s 退避重连
- 扫描引擎独立于会话运行，使用自己的链路；串口扫描要求端口空闲

## 扫描

| 类型 | 实现 |
|---|---|
| 从站 ID | FC01–04 顺序探测 0–255（ID 0 跳过），异常响应视为设备存在 |
| 寄存器地址 | 分块读取，记录可读块、非零数量与首个非零地址 |
| 波特率 | 串口逐波特率重开端口探测，报告响应与延时 |
| 局域网 | IPv4 CIDR（≤ /16）、并发 TCP 连接、可选 Modbus 探测 |

事件：`scan:progress`（done/total/found/phase/finished）、`scan:result`（逐条）、`scan:done`（汇总/错误）。取消通过 `AtomicBool` 在探测间检查。

## 前端结构

- `lib/ipc.ts`：真实 Tauri 与浏览器 Demo 的统一接口
- `lib/demo-bridge.ts`：无 Rust 时的演示数据流（轮询、写入、帧、四类扫描、曲线）
- `stores/sessions.ts`：会话、定义、快照、迷你趋势历史、曲线时间序列
- `stores/scans.ts`：扫描进度与结果
- `components/chart/`：uPlot 实时曲线（分类色板经 dataviz 校验）、跨线提示、图例、数据表
- `components/scan/`：四类扫描配置与实时结果
- 设计系统：OKLCH tokens、毛玻璃仅用于静态 chrome、等宽数值、motion-v + CSS 微动效

## IPC 事件

| 事件 | 载荷 |
|---|---|
| `session:status` | 状态、轮询开关、TX/RX/错误/延时 |
| `session:data` | 定义快照、原始 words、格式化值、数值、延时 |
| `session:frames` | TX/RX 帧（seq、方向、原始字节、解码字段、延时、状态） |
| `workspace:updated` | 会话/定义配置快照 |
| `scan:progress` | 扫描类型、进度、发现数、阶段、错误 |
| `scan:result` | 单条扫描结果 |
| `scan:done` | 扫描汇总与错误 |

命令包含会话创建/轮询、定义 CRUD、单次读、写寄存器、四类扫描与取消、串口枚举。

## 里程碑状态

- P1 脚手架 / 设计系统 / Demo / Windows CI — 完成，CI 待远端运行
- P2 协议核心与测试 — 完成
- P3 串口/TCP/UDP 传输 + 主站 Actor — 完成（串口未用真实硬件验证）
- P4 Tauri 命令/事件 + 工作区持久化 — 命令/事件完成，持久化待实现
- P5 主站 UI 交互 — 定义编辑、写入、基址、检查器完成
- P6 报文导出 + CSV 日志 — 待实现
- P7 从站模拟器 — 浏览器 Demo 完成，真实服务端待实现
- P8 四类扫描 — 完成
- P9 uPlot 曲线 + Windows 发布 — 曲线与发布工作流完成，安装包待远端验证

## 后续优化

- 帧事件按 ~100ms 聚合、UI 侧回查序号（当前按事务批量）
- 会话/定义持久化与恢复
- CSV 数据日志与报文导出
- TCP/UDP/串口从站模拟服务端
- 真实串口设备验证与 macOS/Linux CI 矩阵
