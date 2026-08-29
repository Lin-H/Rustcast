# Rustcast

一款使用 **Tauri 2 + Preact + Tailwind CSS** 构建的轻量 RSS 播客阅读器。当前处于 M2 订阅管理里程碑：多订阅源管理、SQLite 持久化与每集播放进度记忆，并通过 WebView 音频完成流式播放。

![status](https://img.shields.io/badge/version-0.2-blue) ![milestone](https://img.shields.io/badge/milestone-M2%20%E8%AE%A2%E9%98%85%E7%AE%A1%E7%90%86-green)

## ✨ 功能特性（M1）

- 📡 **RSS 订阅解析** — Rust 后端使用 `feed-rs` 解析 RSS/Atom，提取单集、时长、封面与 show notes
- 🔊 **WebView 流式播放** — Preact 使用单例 `<audio>` 播放远程音频，点击即听
- ⏯️ **完整播放控制** — 播放/暂停、进度 seek、音量百分比调节
- 🧾 **Show notes** — 播放中的单集展示受限 HTML 内容；链接通过系统浏览器打开
- 🖼️ **封面展示** — 订阅源、单集和播放条封面展示，加载失败时自动回落
- 🚄 **分页渲染** — 首屏 60 集，每次追加 150 集，避免长列表一次性渲染
- 🎨 **深色琥珀 UI** — Tailwind 设计令牌重建深色主题，中文界面

## ✨ 功能特性（M2）

- 📚 **多订阅源管理** — 侧栏添加 / 删除 / 切换订阅源，手动刷新
- 💾 **SQLite 持久化** — 订阅源、单集元数据与播放进度保存在本地数据库
- ▶️ **续播记忆** — 每集进度自动保存，再次播放从上次位置继续

## 🛠️ 技术栈

| 层 | 选型 | 说明 |
|---|---|---|
| 桌面壳 | Tauri 2 | Windows 优先开发；保留跨平台打包能力 |
| 前端 | Preact + TypeScript | 轻量 UI 和类型化状态 |
| 样式 | Tailwind CSS v4 | 设计令牌集中在 `src/index.css` |
| 状态 | Rematch | Feed 与播放器两个模型 |
| 播放 | HTMLAudioElement | 播放状态由前端音频服务驱动 |
| RSS | feed-rs + reqwest | Rust command 抓取并解析默认订阅源 |
| 外链 | tauri-plugin-opener | 仅授权 HTTP/HTTPS，通过系统浏览器打开 |

## 🏗️ 架构

```text
┌────────────────────────────────────────────┐
│ Preact WebView                             │
│ Rematch state → UI                         │
│ audioService → HTMLAudioElement            │
│ DOMPurify → 受限 show notes                │
└───────────────┬────────────────────────────┘
                │ Tauri IPC: load_initial_state / add_feed / refresh_feed / save_progress
┌───────────────▼────────────────────────────┐
│ Rust backend                               │
│ reqwest → feed-rs → SQLite                 │
└────────────────────────────────────────────┘
```

关键决策：

- **Rust 负责 RSS 抓取、解析与 SQLite 持久化**，不代理音频或图片。
- **M2 通过 IPC 接收订阅 URL**，Rust 端负责规范化、哈希去重并写入 SQLite。
- **音频完全由前端 `<audio>` 管理**，避免 Rust 音频引擎与解码栈常驻内存。
- **`http://` 音频和封面在 Rust 端升级为 `https://`**；播放失败时在播放条显示错误。
- **show notes 不直接信任 RSS HTML**：DOMPurify 白名单净化后渲染。
- **播放进度由前端定时、暂停与播完时写入 SQLite**，再次播放自动续播。

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
├── capabilities/         # Tauri capability 与 opener 权限
├── src/db.rs             # SQLite 迁移、订阅/单集/进度读写
├── src/feed.rs           # RSS 抓取、解析、DTO 和 URL 规范化
├── src/main.rs           # Tauri builder 与 command
└── tauri.conf.json       # 窗口、CSP、打包配置
```

## ⚠️ 已知限制

- 首次启动自动订阅 Syntax FM；可在侧栏添加更多订阅源。
- 无音频的单集会显示为“无法播放”，不会进入播放状态。
- 某些播客 CDN 的音频格式依赖系统 WebView 能力；不支持时显示中文错误。
- 进度 seek 依赖 WebView 的媒体 Range 请求实现。
- Windows 是当前主要验证平台；Linux/macOS 包可在后续里程碑再验证。

## 🗺️ Roadmap

### M2 — 订阅管理

- [x] 界面内添加 / 删除订阅源
- [x] SQLite 持久化订阅和单集元数据
- [x] 每集播放进度记忆
- [x] 手动刷新订阅

### M3 — 播客体验增强

- [ ] 倍速播放
- [ ] ±15 秒快进快退
- [ ] OPML 导入导出
- [ ] 封面磁盘缓存与懒加载
- [ ] 音量对数标度

### M4 — 平台化

- [ ] Linux/macOS 构建验证
- [ ] 系统托盘 / 最小化到托盘
- [ ] 全局媒体键控制
- [ ] 虚拟化长列表
- [ ] 深浅主题切换与多语言
