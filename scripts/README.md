# 构建脚本

开发机没有 Rust 工具链时，Rust 编译、测试与安装包统一交给 GitHub Actions。

## 本地（仅前端 Demo）

```powershell
./scripts/dev-web.ps1
```

```bash
./scripts/dev-web.sh
```

## 本地（已安装 Rust + MSVC Build Tools）

```powershell
./scripts/dev.ps1      # pnpm tauri dev
./scripts/check.ps1    # 前端 + Rust 全量检查
./scripts/build.ps1    # 生成 Windows NSIS + MSI
```

## CI

1. **CI**：push / PR / 手动触发，Windows runner 执行 lint、typecheck、Vitest、Vite build、rustfmt、clippy、cargo test。
2. **Release Windows**：手动触发或推送 `v*` 标签，产出 NSIS + MSI 安装包。
3. 勾选 `offline_webview` 可生成内置离线 WebView2 的安装包（适合 Win10 LTSC / 无外网机器）。

> `tauri::generate_context!` 在编译期嵌入 `dist/`，所以 CI 先执行 `pnpm build` 再运行 Rust 检查。
