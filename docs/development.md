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
- 工具箱：Ping/持续 Ping/路由跟踪/ARP 使用 Windows iphlpapi；ARP 增强模式运行时加载 Npcap `wpcap.dll` 并需要管理员权限；非 Windows 返回明确错误
- HTTP/MQTT TLS 使用 native-tls（Windows SChannel），不引入 OpenSSL/AWS-LC 构建链；MQTT 密码不持久化
- Web 文件服务默认只监听回环地址，界面在切换 0.0.0.0 时持续提示无访问密码风险；上传文件名消毒、路径穿越防护、重名自动后缀、请求体上限 512MB；共享根目录 canonicalize 后校验为目录
- 美化器使用 Prettier standalone + xml-formatter 动态加载，避免进入首屏 bundle；YAML 校验使用 `yaml` 包在前端解析，不上传文件内容
- 工具长任务使用前端事件缓冲与后端 taskId 关联，避免任务在 invoke 返回前完成导致 UI 停在 0%

## 相关文档

- [架构说明](architecture.md)
- [设计文档](../设计文档.txt)
