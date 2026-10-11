# MiniMax Code learning client package

日期：2026-09-02

## 决策

- 复用既有 `/api/v1/agent` 作为唯一远程边界；不让外部 Agent 访问数据库或服务器 Shell。
- 使用公开 HTTPS 地址 `https://de.alan-studio.uk/api/v1`，不依赖 Tailscale 内网域名。
- CLI 继续是零第三方依赖的 Node 脚本，不为当前需求构建二进制发行版。
- MiniMax 端随包保存独立 `deai.env`，仅含 API URL 与 Token；实际密钥不进入仓库或 skill 正文。

## 已落地

- `tools/deai` 新增 `--env-file PATH`，用于读取便携客户端旁的 API 配置。
- 新增 `tools/minimax-deai-learning-import/`，包含适配远程 CLI 的 skill、公开 URL 模板与安装说明。
- 打包时从当前 `tools/deai` 注入 `bin/deai`，避免在仓库内维护第二份 CLI 源码。
