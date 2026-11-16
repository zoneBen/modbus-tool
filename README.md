# Modbus Tool

对标 Modbus Poll 的轻量桌面调试工具：主站读写、原始报文监视、四类设备扫描、实时曲线。从站模拟器当前仅提供演示运行时，真实服务端与 CSV 日志在后续阶段接入。

技术栈：**Rust + Tauri 2 + Vue 3 + TypeScript + Tailwind CSS v4 + shadcn-vue + uPlot**。核心协议层自研，支持 RTU / ASCII / MBAP 三种帧格式与串口 / TCP / UDP 三种链路。

> 当前状态：**可用的主站调试版**。真实 TCP/UDP/串口主站引擎、报文监视、定义编辑、写入、四类扫描与 uPlot 曲线已接入 Rust 后端；浏览器 Demo（`pnpm dev`）无需 Rust 即可体验完整 UI。已通过本地 `cargo test --workspace`、TCP 回环集成测试、ESLint / vue-tsc / Vitest / Vite build；GitHub Actions 工作流已就绪，尚未在远端执行。

## 已实现

- **协议核心**（`crates/modbus-core`）：PDU 编解码、CRC16/LRC、RTU/ASCII/MBAP、异常码、INT8/16/32/64、UINT、FLOAT32/64、HEX/BIN/ASCII、ABCD/DCBA/BADC/CDAB、从站模拟引擎；39 个单元与属性测试
- **主站引擎**：TCP / UDP / 串口；RTU / ASCII / MBAP；周期轮询、单次读、FC05/06/0F/10 写、读重试、写不自动重试、自动重连、跨事务 ID 校验、t3.5 串口排空
- **定义编辑**：FC01–04、从站 ID、地址/数量、数据类型、字节/字序、轮询周期、启停；保存后即时生效
- **报文监控**：双向原始帧、分段着色 HEX、字段解码、时间戳、延时、搜索/方向过滤/跟随/清空、帧详情
- **四类扫描**：从站 ID、寄存器地址、串口波特率、局域网 IP/端口（CIDR + 并发 + 可选 Modbus 探测）；进度、取消、实时结果
- **实时曲线**：uPlot 多序列、1/5/15/30 分钟窗口、暂停、跨线提示、图例实时值、数据表视图；分类色板经 dataviz 校验（深色表面 6 色全部通过）
- **界面**：深色仪器风、毛玻璃静态面板、值变化闪烁、状态呼吸、可拖拽监控抽屉、快捷键预留、地址基址 0/1
- **演示运行时**：浏览器内生成真实结构的 MBAP/RTU 帧、主站/从站数据、扫描过程与曲线
- **工程化**：Windows CI（lint/typecheck/Vitest/build/rustfmt/clippy/cargo test）；Release Windows（NSIS/MSI，可选离线 WebView2）

## 已知限制 / 下一步

- **从站模拟器**：仅浏览器 Demo；真实 TCP/UDP/串口从站服务端待实现（P7）
- **CSV 数据日志**：UI 入口已预留，写入任务待实现（P6）
- **工作区持久化**：会话/定义当前驻留内存，重启丢失（P4）
- **报文导出**：入口已预留，导出任务待实现（P6）
- **串口**：代码与协议测试通过，尚未用真实 RS-485 / USB 串口设备验证
- **FC08/11/17/22/23/43-14**：后续阶段
- **CI**：Windows 工作流已编写，尚未在 GitHub 远端运行；安装包签名与自动更新待接入

## 开发

```bash
pnpm install
pnpm dev          # 浏览器 Demo，无需 Rust
```

有 Rust 工具链时：

```bash
pnpm tauri dev
```

常用检查：

```bash
pnpm lint
pnpm typecheck
pnpm test:unit
pnpm format:check
pnpm build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`scripts/` 提供 PowerShell / Bash 包装：`dev-web`、`dev`、`check`、`build`。

## CI 构建（当前仅 Windows）

仓库坚持 CI-first，本地不要求安装 Rust：

1. 推送代码 / 提交 PR → **CI** 自动运行前端与 Rust 检查。
2. **Actions → Release Windows → Run workflow** → 产出 NSIS + MSI 安装包，作为 artifact 下载（保留 14 天）。
3. 推送 `v*` 标签 → 自动创建 draft release 并附带安装包。
4. 目标机器无法联网或缺少 WebView2 时，运行同上工作流并勾选 **offline_webview**，产出内置离线 WebView2 的安装包。

安装包未签名，Windows SmartScreen 会提示；后续阶段再加入签名与自动更新。

## 项目结构

```
crates/modbus-core/    纯协议核心（PDU、帧、值解析、模拟引擎，无 I/O 依赖）
src-tauri/             Tauri 应用（会话 Actor、传输、扫描、定义/写入、事件）
src/                   Vue 3 前端（仪器风 UI、Pinia、uPlot、i18n）
scripts/               本地与 CI 构建脚本
.github/workflows/     Windows CI + Release
docs/architecture.md   协议矩阵、Actor 与事件流
```

## 安全

- Tauri 仅启用 `core:default` 与 `dialog:default`，不引入 opener/fs 插件
- 生产 CSP 限制为 `self` + Tauri IPC；开发 CSP 单独放开以支持 Vite HMR
- 不包含硬编码密钥
- 设备报文、扫描输入、定义参数在 Rust 边界校验长度、功能码、地址与数量范围
- 会话配置当前仅驻留内存，持久化实现时会在写入前校验 schema

## 文档

- [架构说明](docs/architecture.md)
- [设计文档](设计文档.txt)
