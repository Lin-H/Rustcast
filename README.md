# Rustcast

一款使用 **Tauri 2 + Preact + Tailwind CSS** 构建的轻量 RSS 播客阅读器：多订阅源管理、SQLite 持久化、续播记忆、倍速、±15 秒、OPML 导入导出、封面/音频双磁盘缓存（边下边播）、深浅主题、中英双语、分页浏览、一键刷新与 GitHub Releases 自动更新，三平台（Windows/Linux/macOS）构建均经 CI 验证。

![version](https://img.shields.io/badge/version-0.5.2-blue) ![license](https://img.shields.io/badge/license-MIT-green) ![tauri](https://img.shields.io/badge/Tauri%202-v2.11-orange) ![platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey) ![i18n](https://img.shields.io/badge/i18n-%E4%B8%AD%E6%96%87%20%7C%20English-informational)

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

## ✨ 功能特性（M5）

- 💾 **边下边播音频缓存** — 正在听的单集自动下载到应用同目录 `audio-cache/`，播放优先走本地（rustcast-media 协议）；切集后后台继续把上一集下完
- 📊 **视频站式进度条** — 播放条叠加浅色已下载区间条；seek 未下载段自动按需拉取+缓冲
- 📴 **离线徽标** — 整集缓存完成后显示「离线可用」，单集列表与播放条同步展示，断网也能听
- 🔄 **自动更新** — 以 GitHub Releases 为更新源，minisign 签名验证，启动自动检查 + 顶栏手动检查，下载进度可视化，静默安装后自动重启

## ✨ 其他特性

- 🧲 **命名空间封面兼容** — libsyn:widescreen-image 等第三方扩展的单集封面自动识别（media 缩略图优先，扩展兜底）
- 🃏 **一键刷新** — 订阅源标题旁全量刷新按钮，并行刷新互不阻塞
- 🏷️ **版本徽标** — 顶栏实时显示当前应用版本
- 🇨🇳 **中文优先 UI** — 深色琥珀主题，中文/英文一键切换

## 🛠️ 技术栈

| 层 | 选型 | 说明 |
|---|---|---|
| 桌面壳 | Tauri 2 | Windows 优先开发；保留跨平台打包能力 |
| 前端 | Preact + TypeScript | 轻量 UI 和类型化状态 |
| 样式 | Tailwind CSS v4 | 设计令牌集中在 `src/index.css` |
| 状态 | Rematch | `feed` / `player` / `settings` / `update` 四个模型 |
| 播放 | HTMLAudioElement | 播放状态由前端音频服务驱动 |
| RSS | feed-rs + reqwest | Rust command 抓取并解析任意订阅源 |
| 存储 | turso | SQLite 兼容本地单文件库，迁移记录在 `schema_migrations` |
| 外链 | tauri-plugin-opener | 仅授权 HTTP/HTTPS，通过系统浏览器打开 |
| 更新 | tauri-plugin-updater + process | minisign 签名 + GitHub Releases 清单 |
| CI | GitHub Actions | `v*` 标签触发三平台构建并发布 Release |

## 🏗️ 架构

```text
┌────────────────────────────────────────────┐
│ Preact WebView                             │
│ Rematch state → UI                         │
│ audioService → HTMLAudioElement            │
│ DOMPurify → 受限 show notes                │
└───────────────┬────────────────────────────┘
                │ Tauri IPC（14 个 command）
                │ load_initial_state / list_feeds / load_feed / set_selected_feed
                │ add_feed / refresh_feed / delete_feed / save_progress
                │ import_opml / export_opml / cache_artwork
                │ ensure_audio_cache / audio_cache_status / list_cached_episodes
┌───────────────▼────────────────────────────┐
│ Rust backend                               │
│ reqwest → feed-rs → turso (SQLite)         │
│ rustcast-media:// 边下边播音频缓存        │
│ updater → GitHub Releases latest.json      │
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

发布流程由 GitHub Actions 驱动：推送 `v*` 标签触发 `.github/workflows/build.yml`，在 Windows / Linux / macOS 矩阵执行 `pnpm tauri build`（用 GitHub Secrets 中的 `TAURI_SIGNING_PRIVATE_KEY` 签名），产物重命名为 `<tag>-<platform>-<文件名>` 后自动发布 GitHub Release，并生成 `latest.json`（updater 清单）附到同一 Release；标签含 `-alpha` / `-beta` / `-rc` 时自动标记为预发布。已安装客户端启动后自动检查并提示更新（更新源指向最新 Release 的 latest.json，签名不匹配会被拒绝）。

> 发布前置：仓库 Settings → Secrets 中需配置 `TAURI_SIGNING_PRIVATE_KEY`（`pnpm tauri signer generate` 生成的私钥内容）。私钥丢失则已发版用户无法收到后续更新，请妥善备份。

## 📂 目录结构

```text
src/
├── App.tsx               # 应用布局与音频事件绑定
├── components/           # 顶栏、侧栏、列表、卡片、播放条、更新横幅
├── hooks/                # useTranslator 取文案 hook
├── lib/                  # i18n 字典、时间格式化和 HTML 净化
├── services/             # Tauri IPC 与单例 audio 服务
├── store/                # Rematch store：feed / player / settings / update
└── types.ts              # Feed/Episode DTO 类型

src-tauri/
├── capabilities/         # Tauri capability、dialog/updater/process 权限
├── src/artwork.rs        # 封面缓存状态与 cache_artwork command
├── src/audio_cache.rs    # 分块下载、Range 协议与本地音频缓存
├── src/db.rs             # turso 迁移、订阅/单集/进度读写
├── src/feed.rs           # RSS 抓取、解析、DTO 和 URL 规范化
├── src/main.rs           # Tauri builder 与 command
├── src/opml.rs           # OPML 解析/渲染与封面磁盘缓存实现
└── tauri.conf.json       # 窗口、CSP、updater 与打包配置

.github/
├── workflows/build.yml           # v* 标签触发的三平台发布构建
└── workflows/platform-check.yml # 手动触发的三平台构建验证
```

## ⚠️ 已知限制

- 首次启动自动订阅 Syntax FM；可在侧栏添加更多订阅源。
- 无音频的单集会显示为“无法播放”，不会进入播放状态。
- 某些播客 CDN 的音频格式依赖系统 WebView 能力；不支持时显示中文错误。
- 进度 seek 依赖 WebView 的媒体 Range 请求实现。
- 倍速超过 2x 时部分 CDN 的音频时间戳会漂移；WebView 实现差异无法完全规避。
- 封面缓存目录 `artwork-cache/` 与音频缓存目录 `audio-cache/` 无自动清理，长期使用后可手动删除。
- 不支持 Range 请求的 CDN 无法分块缓存，此时回落远程直连播放。
- Windows 是主要开发平台；Linux/macOS 构建产物已通过 CI 验证，运行时行为仍以用户反馈为准。

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

### M4 — 平台化 ✅ 已完成（v0.4.0）

- [x] Linux/macOS 构建验证（`platform-check` workflow 三平台全绿）
- [x] 深浅主题切换与多语言

### M5 — 缓存与更新 ✅ 已完成（v0.5.0–v0.5.2）

- [x] 边下边播音频缓存（rustcast-media:// 自定义协议 + 视频站式进度条）
- [x] 离线徽标与列表缓存状态
- [x] 自动更新（GitHub Releases + minisign 签名 + latest.json）
- [x] 一键刷新全部订阅源
- [x] libsyn 等命名空间扩展封面兼容
- [x] 顶栏版本徽标

> 后续新想法按需追加。