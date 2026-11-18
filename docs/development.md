# 开发与构建

本文档收录技术栈、开发、测试、CI 与项目结构信息；产品功能说明见根目录 [README](../README.md)。

技术栈：**Rust + Tauri 2 + Vue 3 + TypeScript + Tailwind CSS v4 + shadcn-vue + uPlot**。协议层位于 `crates/modbus-core`，不依赖 Tauri/串口/网络。

## 本地开发

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
src-tauri/             Tauri 应用（会话 Actor、传输、扫描、从站服务端、定义/写入、事件）
src/                   Vue 3 前端（仪器风 UI、Pinia、uPlot、i18n）
scripts/               本地与 CI 构建脚本
.github/workflows/     Windows CI + Release
docs/architecture.md   协议矩阵、Actor 与事件流
```

## 安全

- Tauri 仅启用 `core:default`、窗口主题与 `dialog:default`，不引入 opener/fs 插件
- 生产 CSP 限制为 `self` + Tauri IPC；开发 CSP 单独放开以支持 Vite HMR
- 不包含硬编码密钥
- 设备报文、扫描输入、定义参数在 Rust 边界校验长度、功能码、地址与数量范围
- 会话配置在工作区/全局配置导入时校验字段与版本，恢复后默认保持断开

## 相关文档

- [架构说明](architecture.md)
- [设计文档](../设计文档.txt)
