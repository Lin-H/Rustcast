# Rustcast

一款使用 **Tauri 2 + Preact + Tailwind CSS** 构建的轻量 RSS 播客阅读器。M3 播客体验增强已完成：倍速播放、±15 秒快进快退、OPML 导入导出、封面磁盘缓存与音量对数标度，并保留 M2 的多订阅源管理、SQLite 持久化与续播记忆。

![status](https://img.shields.io/badge/version-0.3.0-blue) ![milestone](https://img.shields.io/badge/milestone-M3%20done-brightgreen)

## ✨ 功能特性（M1）

- 📡 **RSS 订阅解析** — Rust 后端使用 `feed-rs` 解析 RSS/Atom，提取单集、时长、封面与 show notes
- 🔊 **WebView 流式播放** — Preact 使用单例 `<audio>` 播放远程音频，点击即听
- ⏯️ **完整播放控制** — 播放/暂停、进度 seek、音量百分比调节
- 🧾 **Show notes** — 播放中的单集展示受限 HTML 内容；链接通过系统浏览器打开
- 🖼️ **封面展示** — 订阅源、单集和播放条封面展示，加载失败时自动回落
- 🚄 **分页浏览** — 每页 60 集，页码导航显示总页数，避免长列表一次性渲染
- 🎨 **深色琥珀 UI** — Tailwind 设计令牌重建深色主题，中文界面

## ✨ 功能特性（M2）

- 📚 **多订阅源管理** — 侧栏添加 / 删除 / 切换订阅源，手动刷新；上次选中的订阅源重启后自动恢复
- 💾 **SQLite 持久化** — 订阅源、单集元数据与播放进度保存在本地 `rustcast.db`（turso 本地引擎，位于可执行文件同目录）
- ▶️ **续播记忆** — 播放中约每 5 秒节流保存，暂停 / 出错 / 切集即时落库；再次播放从上次位置继续（进度超过 5 秒且未播完才续播）
- 🔁 **重播保护** — 点击正在播放的单集不会重新加载音频，避免误触重启播放

## ✨ 功能特性（M3）

- ⏩ **倍速播放** — 0.75x / 1x / 1.25x / 1.5x / 1.75x / 2x 循环切换，设置持久化，重连后倍速不丢失
- ⏪⏩ **±15 秒快进快退** — 播放键两侧一键跳转，自动夹在有效范围内
- 📥 **OPML 导入导出** — 系统对话框选择文件；导入去重已订阅，导出标准 OPML 2.0
- 💿 **封面磁盘缓存** — 封面按 URL 哈希缓存在可执行文件同目录 `artwork-cache/`，二次加载不联网
- 🔍 **封面懒加载** — 图片 `loading="lazy"` + 异步解码，列表滚动更流畅
- 🔊 **音量对数标度** — 滑杆感知曲线（平方映射），低音量段调节更细腻，音量记忆

## ✨ 功能特性（M4）

- 🌗 **深浅主题切换** — 跟随系统 / 浅色 / 深色三档，令牌级双主题，即时生效，选择持久化
- 🌐 **中英双语** — 顶栏一键切换 中文 / English，全部 UI 文案与日期格式随语言切换，选择持久化

## 🛠️ 技术栈

| 层 | 选型 | 说明 |
|---|---|---|
| 桌面壳 | Tauri 2 | Windows 优先开发；保留跨平台打包能力 |
| 前端 | Preact + TypeScript | 轻量 UI 和类型化状态 |
| 样式 | Tailwind CSS v4 | 设计令牌集中在 `src/index.css` |
| 状态 | Rematch | Feed 与播放器两个模型 |
| 播放 | HTMLAudioElement | 播放状态由前端音频服务驱动 |
| RSS | feed-rs + reqwest | Rust command 抓取并解析任意订阅源 |
| 存储 | turso | SQLite 兼容本地单文件库，迁移记录在 `schema_migrations` |
| 外链 | tauri-plugin-opener | 仅授权 HTTP/HTTPS，通过系统浏览器打开 |
| CI | GitHub Actions | `v*` 标签触发三平台构建并发布 Release |

## 🏗️ 架构

```text
┌────────────────────────────────────────────┐
│ Preact WebView                             │
│ Rematch state → UI                         │
│ audioService → HTMLAudioElement            │
│ DOMPurify → 受限 show notes                │
└───────────────┬────────────────────────────┘
                │ Tauri IPC（11 个 command）
                │ load_initial_state / list_feeds / load_feed / set_selected_feed
                │ add_feed / refresh_feed / delete_feed / save_progress
                │ import_opml / export_opml / cache_artwork
┌───────────────▼────────────────────────────┐
│ Rust backend                               │
│ reqwest → feed-rs → turso (SQLite)         │
└────────────────────────────────────────────┘
```

关键决策：

- **Rust 负责 RSS 抓取、解析与 SQLite 持久化**，不代理音频；封面经磁盘缓存 + asset 协议本地回放，不再直接渲染远程图片。
- **订阅 URL 由前端经 IPC 传入**，Rust 端负责规范化、哈希去重并写入 SQLite。
- **音频完全由前端 `<audio>` 管理**，避免 Rust 音频引擎与解码栈常驻内存。
- **`http://` 音频和封面在 Rust 端升级为 `https://`**；播放失败时在播放条显示错误。
- **show notes 不直接信任 RSS HTML**：DOMPurify 白名单净化后渲染。
- **播放进度由前端节流写入 SQLite**（约 5 秒一次），暂停、出错、切集、播完时即时落库；再次播放自动续播。
- **同一单集重复点击不重新加载音频**；切换单集时先落库上一集进度。
- **数据库文件 `rustcast.db` 放在可执行文件同目录**（便携式布局），迁移按名字记录在 `schema_migrations` 表。
- **推送 `v*` 标签触发三平台 CI 构建并自动发布 GitHub Release**。
- **倍速与音量保存在 WebView localStorage**，重启后自动恢复；倍速在断线重连后自动重新应用。
- **封面缓存按 URL sha256 命名**，魔数识别 png/jpg/webp/gif/avif；下载失败时回落远程 URL，不阻塞展示。

## 🚀 开发与验证

```bash
pnpm install
pnpm tauri dev          # 启动桌面应用

pnpm typecheck          # TypeScript 检查
pnpm build              # TypeScript + Vite production build
cargo check             # 在 src-tauri/ 内执行
cargo test              # 在 src-tauri/ 内执行
pnpm tauri build        # 生产桌面包
```

默认订阅源定义在 `src-tauri/src/feed.rs`，首次启动且数据库为空时自动订阅：

```rust
pub const DEFAULT_FEED_URL: &str = "https://feed.syntax.fm/";
```

发布流程由 GitHub Actions 驱动：推送 `v*` 标签触发 `.github/workflows/build.yml`，在 Windows / Linux / macOS 矩阵执行 `pnpm tauri build`，产物重命名为 `<tag>-<platform>-<文件名>` 后自动发布 GitHub Release；标签含 `-alpha` / `-beta` / `-rc` 时自动标记为预发布。

## 📂 目录结构

```text
src/
├── App.tsx               # 应用布局与音频事件绑定
├── components/           # 顶栏、侧栏、列表、卡片、播放条
├── lib/                  # 时间格式化和 HTML 净化
├── services/             # Tauri IPC 与单例 audio 服务
├── store/                # Rematch store 和模型
└── types.ts              # Feed/Episode DTO 类型

src-tauri/
├── capabilities/         # Tauri capability、dialog 与 opener 权限
├── src/artwork.rs        # 封面缓存状态与 cache_artwork command
├── src/db.rs             # turso 迁移、订阅/单集/进度读写
├── src/feed.rs           # RSS 抓取、解析、DTO 和 URL 规范化
├── src/main.rs           # Tauri builder 与 command
├── src/opml.rs           # OPML 解析/渲染与封面缓存实现
└── tauri.conf.json       # 窗口、CSP、asset 协议与打包配置

.github/
└── workflows/build.yml   # v* 标签触发的三平台发布构建
```

## ⚠️ 已知限制

- 首次启动自动订阅 Syntax FM；可在侧栏添加更多订阅源。
- 无音频的单集会显示为“无法播放”，不会进入播放状态。
- 某些播客 CDN 的音频格式依赖系统 WebView 能力；不支持时显示中文错误。
- 进度 seek 依赖 WebView 的媒体 Range 请求实现。
- 倍速超过 2x 时部分 CDN 的音频时间戳会漂移；WebView 实现差异无法完全规避。
- 封面缓存目录 `artwork-cache/` 无自动清理，长期使用后可手动删除。
- Windows 是当前主要验证平台；Linux/macOS 由 CI 发布构建覆盖，本地验证留待 M4。

## 🗺️ Roadmap

### M2 — 订阅管理 ✅ 已完成（v0.2.0）

- [x] 界面内添加 / 删除订阅源
- [x] SQLite 持久化订阅和单集元数据
- [x] 每集播放进度记忆
- [x] 手动刷新订阅

### M3 — 播客体验增强 ✅ 已完成（v0.3.0）

- [x] 倍速播放
- [x] ±15 秒快进快退
- [x] OPML 导入导出
- [x] 封面磁盘缓存与懒加载
- [x] 音量对数标度

### M4 — 平台化

- [ ] Linux/macOS 构建验证（`platform-check` workflow 手动触发验证中）
- [x] 深浅主题切换与多语言