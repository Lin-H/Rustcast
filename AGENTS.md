# AGENTS.md — AI 协作开发指南

本文件面向在 Rustcast 仓库工作的 AI 编码助手和新成员，记录当前架构、构建方式与协作规则。

## 项目概览

- **名称**：Rustcast — Tauri + Preact 的 RSS 播客阅读器
- **当前里程碑**：M1 Tauri 重构（默认 Syntax FM + WebView 流式播放）
- **主要平台**：Windows；Linux/macOS 构建留到后续里程碑验证
- **UI 语言**：中文；代码注释保持最少必要

## 构建与验证命令

在仓库根目录执行：

```bash
pnpm install
pnpm tauri dev              # 启动桌面应用
pnpm typecheck              # TypeScript 检查
pnpm build                  # TypeScript + Vite production build
pnpm tauri build            # 生产桌面包
```

Rust 命令在 `src-tauri/` 内执行：

```bash
cargo check
cargo test
```

提交涉及 `src-tauri/src/feed.rs`、Tauri command、权限或 CSP 的改动前，至少运行 `cargo test`。涉及 UI、状态或音频服务的改动，运行 `pnpm typecheck`、`pnpm build`，并手动跑一次 `pnpm tauri dev`。

## 架构地图

| 文件 / 目录 | 职责 |
|---|---|
| `src/App.tsx` | 应用布局、初始加载和 audio 事件绑定 |
| `src/components/` | 顶栏、订阅源侧栏、单集列表/卡片、播放条 |
| `src/services/audio.ts` | 单例 `HTMLAudioElement`，唯一音频控制入口 |
| `src/services/tauri.ts` | Tauri IPC 与系统浏览器外链 |
| `src/store/` | Rematch store；`feed` 和 `player` 两个模型 |
| `src/lib/sanitize.ts` | DOMPurify 白名单净化 show notes |
| `src/index.css` | Tailwind v4 与深色琥珀设计令牌 |
| `src-tauri/src/main.rs` | Tauri builder 与 `load_default_feed` command |
| `src-tauri/src/feed.rs` | RSS 抓取、解析、DTO、URL 规范化 |
| `src-tauri/capabilities/` | WebView capability 和 opener 限制 |
| `src-tauri/tauri.conf.json` | 窗口、CSP、dev/build 流程与打包配置 |

## 数据流

1. App 启动时调用 `dispatch.feed.load()`。
2. Feed effect 通过 `invoke("load_default_feed")` 调用 Rust command。
3. Rust 使用 reqwest 下载 XML，feed-rs 解析成 `FeedDto` / `EpisodeDto`。
4. Preact Rematch store 保存 feed 和分页状态。
5. 点击可播放单集时，`player` effect 调用 `audioService.load()`。
6. 单例 `<audio>` 事件回写 player 状态，驱动列表徽标和播放条。

## 关键规则与决策

- **不要重新引入 rodio 或 Rust 音频线程**；M1 重构目标是让播放状态和内存压力留在 WebView 的 `<audio>` 生命周期内。
- **不要让前端传订阅 URL 给 IPC**；M1 只有 Rust 常量里的 Syntax FM。
- **媒体 URL 只接受 HTTP/HTTPS**；HTTP 会升级为 HTTPS，其他协议返回 `null`。
- **无音频单集必须保留在列表中**，卡片禁用并显示“无法播放”。
- **不要直接 `dangerouslySetInnerHTML` 原始 RSS HTML**；必须经过 `sanitizeShowNotes()`。
- **WebView 内不要发生页面级导航**；show notes 链接用 `openExternal()` 交给系统浏览器。
- **不要把 `HTMLAudioElement` 放进 Rematch state**；Rematch state 保持可序列化。
- 播放器同一时间只有一个 current episode；选中、展开和播放语义沿用 M1。

## 版本陷阱记录

### Tauri 2

- Windows capability 的 opener scope 放在 `src-tauri/capabilities/default.json`。
- Rust 侧注册 `tauri_plugin_opener::init()`，前端使用 `@tauri-apps/plugin-opener`。
- CSP 必须显式允许 `img-src https:` 和 `media-src https:`，否则远程封面/音频会被 WebView 拦截。

### Preact + Rematch

- TSX 使用 `jsxImportSource: "preact"`。
- Rematch state 不保存 DOM 节点或类实例。
- 组件通过 `useAppSelector` 订阅 store，避免为了 Redux 绑定额外引入 React runtime。

### Tailwind CSS v4

- 通过 `@tailwindcss/vite` 插件接入。
- 设计令牌写在 `src/index.css` 的 `@theme` 中，优先使用语义化 class，不在组件里散落硬编码色值。

## Git 工作流

- 远程：`git@github.com:Lin-H/Rustcast.git`
- 旧 iced/rodio 实现保存在历史分支/提交中，当前 Tauri 重构分支不应保留死代码。
- 发布时打 tag 并推送；当前重构从 `0.2.0` 开始。
