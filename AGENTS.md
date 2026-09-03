# AGENTS.md — AI 协作开发指南

本文件面向在 Rustcast 仓库工作的 AI 编码助手和新成员，记录当前架构、构建方式与协作规则。

## 项目概览

- **名称**：Rustcast — Tauri + Preact 的 RSS 播客阅读器
- **当前状态**：全部里程碑（M1–M5）完成，当前版本 v0.5.2；新功能按需追加
- **主要分支**：`feat/m2-turso-local-db` 为长期开发分支，阶段性 `--no-ff` 合并回 master；发布时在 master 上打 `v*` 标签
- **主要平台**：Windows 主开发；Linux/macOS 构建已由 `platform-check` workflow 验证
- **UI 语言**：中文优先（i18n 字典含 zh/en）；代码注释保持最少必要

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
| `src/components/` | 顶栏、订阅源侧栏、单集列表/卡片、播放条、更新横幅 |
| `src/services/audio.ts` | 单例 `HTMLAudioElement`，唯一音频控制入口 |
| `src/services/tauri.ts` | Tauri IPC 与系统浏览器外链 |
| `src/store/` | Rematch store；`feed`、`player`、`settings`、`update` 四个模型 |
| `src/lib/sanitize.ts` | DOMPurify 白名单净化 show notes |
| `src/lib/i18n.ts` | zh/en 文案字典与翻译函数 |
| `src/hooks/useTranslator.ts` | 组件内取文案 hook |
| `src/index.css` | Tailwind v4、深浅双套设计令牌 |
| `src-tauri/src/main.rs` | Tauri builder、turso 数据库注入、asset scope 注册与全部 14 个 command |
| `src-tauri/src/feed.rs` | RSS 抓取、解析、DTO、URL 规范化 |
| `src-tauri/src/opml.rs` | OPML 解析/渲染与封面磁盘缓存实现（含单元测试） |
| `src-tauri/src/artwork.rs` | ArtworkState 注入与 cache_artwork command |
| `src-tauri/src/audio_cache.rs` | 分块下载、Range 协议与本地音频缓存（含单元测试） |
| `src-tauri/src/db.rs` | turso 迁移与订阅、单集、播放进度读写 |
| `src-tauri/capabilities/` | WebView capability 和 opener 限制 |
| `src-tauri/tauri.conf.json` | 窗口、CSP、asset 协议与打包配置 |
| `.github/workflows/build.yml` | `v*` 标签触发的三平台构建与 GitHub Release 发布 |
| `.github/workflows/platform-check.yml` | 手动触发的三平台构建验证（仅 artifact，不发布） |

## 数据流

1. App 启动时调用 `dispatch.feed.load()`，经 `invoke("load_initial_state_command")` 读取订阅列表、上次选中订阅及其单集。
2. 首次启动数据库为空时，Rust 自动订阅内置 Syntax FM；添加 / 刷新 / 删除分别走 `add_feed_command` / `refresh_feed_command` / `delete_feed_command`；切换订阅时 `set_selected_feed_command` 持久化选中项；一键刷新走前端 `refreshAllSubscriptions`（Promise.allSettled 并行，`refreshingFeedIds` 集合驱动 spinner）。
3. Rust 使用 reqwest 下载 XML，feed-rs 解析后写入 turso（SQLite 兼容）数据库，再以 DTO 返回给 WebView；feed-rs 不识别的命名空间封面（libsyn:widescreen-image 等）由 quick-xml 预扫兑底（`extract_extension_item_images`，按 item 文档顺序对齐，media 缩略图优先）。
4. Preact Rematch store 保存订阅列表、选中订阅和分页状态（每页 60 集，页码窗口导航，刷新后自动夹住页码）。
5. 点击可播放单集时，`player.playEpisode` 先做重播保护（同一集直接返回），再从 `progress.positionSecs` 续播（进度 >5 秒且未播完）并调用 `audioService.load()`；切集前先把上一集进度落库。
6. 播放进度由前端节流保存（约 5 秒间隔），暂停、出错与播完时即时调用 `save_progress_command` 持久化。
7. 单例 `<audio>` 事件回写 player 状态，驱动列表徽标和播放条。
8. 封面由 `Artwork` 组件请求 `cache_artwork_command`：命中则用 `convertFileSrc` 走 asset 协议加载本地文件，未命中时 Rust 下载入 `artwork-cache/`；失败回落远程 URL。
9. OPML 导入/导出经 `import_opml_command` / `export_opml_command`：Rust 用系统对话框选路径，quick-xml 解析/渲染，逐条复用 add_feed 去重。
10. 倍速（0.75–2x 循环）与音量保存在 localStorage，启动时应用到 audio 元素（音量经平方曲线映射为感知标度）。
11. 主题（system/light/dark）与语言（zh/en）由 settings 模型管理：写入 localStorage 并同步 `html` class（`theme-light` / `theme-dark`），system 模式监听 `prefers-color-scheme` 变化；文案统一从 `lib/i18n.ts` 字典取。
12. 播放前 `ensure_audio_cache_command` 注册并启动音频缓存，`<audio>` 的 src 指向 `rustcast-media://localhost/{episodeId}`；协议命中本地段读文件，未命中按需拉取；后台顺序任务每块完成时发 `audio-cache-progress` 事件驱动进度条区间与「离线可用」徽标；缓存失败自动回落远程 URL 直连。
13. 更新由 update 模型管理：启动后 4 秒自动检查（每 6 小时复查），发现新版本在顶栏下方横幅提示，下载进度可视化，`downloadAndInstall` + `relaunch` 完成安装重启；手动检查入口在顶栏刷新图标。
14. 顶栏版本徽标由 `getVersion()`（`@tauri-apps/api/app`，`core:app:allow-version` 已含于 core:default）运行时读取，自动随发版更新，无需手动同步。

## 关键规则与决策

- **不要重新引入 rodio 或 Rust 音频线程**；M1 重构目标是让播放状态和内存压力留在 WebView 的 `<audio>` 生命周期内。
- **订阅 URL 由前端经 IPC 传入**；Rust 端负责规范化、哈希去重与 SQLite 持久化，首次启动自动订阅内置 Syntax FM。
- **媒体 URL 只接受 HTTP/HTTPS**；HTTP 会升级为 HTTPS，其他协议返回 `null`。
- **无音频单集必须保留在列表中**，卡片禁用并显示“无法播放”。
- **不要直接 `dangerouslySetInnerHTML` 原始 RSS HTML**；必须经过 `sanitizeShowNotes()`。
- **WebView 内不要发生页面级导航**；show notes 链接用 `openExternal()` 交给系统浏览器。
- **不要把 `HTMLAudioElement` 放进 Rematch state**；Rematch state 保持可序列化。
- 播放器同一时间只有一个 current episode；选中、展开和播放语义沿用 M1。
- **重复点击正在播放的单集不重新加载音频**（`playEpisode` 开头的同 id guard）；切换单集时先 flush 上一集进度。
- **播放进度由前端驱动并落库**；通过 `save_progress_command` 写入 SQLite，Rematch state 保持可序列化。
- **数据库文件 `rustcast.db` 位于可执行文件同目录**（便携式布局，非系统 app data 目录）；迁移按名字记录在 `schema_migrations` 表，新增迁移追加到 `db.rs` 的 `MIGRATIONS`。
- **持久化引擎是 `turso` crate（本地模式），不是 rusqlite**；API 为 async（`db.rs` 全部函数是 async），迁移 SQL 与 SQLite 语法兼容。
- **封面缓存目录 `artwork-cache/` 必须同时进入 asset protocol scope**（启动时 `allow_directory`）且 CSP `img-src` 允许 `asset: http://asset.localhost`，否则 WebView 拒绝加载。
- **tauri crate 必须开 `protocol-asset` feature** 才有 `app.asset_protocol_scope()`。
- **OPML 解析用 quick-xml 0.42**：`local_name()` 返回 `LocalName`（内部 `&str`），属性解码用 `normalized_value(XmlVersion::Implicit1_0)`。
- **快进/快退走 `audioService.skip(±15)`**，夹在 `[0, duration]`；倍速重连后由 `resetSource` 重新应用。
- **音频播放 src 是 `rustcast-media://localhost/{episodeId}` 自定义协议**：`register_asynchronous_uri_scheme_protocol` 注册，闭包内不能让 `ctx` 逃逸——State 要先 `.inner().clone()` 成 `Arc` 再 spawn；CSP `media-src` 必须同时允许 `rustcast-media:` 与 `http://rustcast-media.localhost`（Windows WebView2 的自定义协议走 http 形式）。
- **音频缓存失败自动回落远程直连**（playEpisode 内 try/catch），播放不中断是底线。
- **自动更新用 tauri-plugin-updater + tauri-plugin-process**：`tauri.conf.json` 里 `createUpdaterArtifacts: true` + `plugins.updater`（pubkey/endpoints）；构建时需 `TAURI_SIGNING_PRIVATE_KEY` 环境变量（GitHub Secrets 配置，本地 `pnpm tauri build` 前也要 export，`tauri dev` 不需要）；CI release job 生成 `latest.json`（windows-x86_64/linux-x86_64/darwin-aarch64 + 签名 + URL）附到 Release，updater 端点指向 `releases/latest/download/latest.json`；私钥在 `C:\Users\<user>\.tauri\rustcast.key`，丢失则老用户收不到新更新。Windows 更新包是 NSIS `.exe`（updater 不支持 MSI，且 WiX 的 ProductVersion 不接受 semver 预发布号，msi/rpm target 已从 bundle.targets 移除），macOS 用 `.app.tar.gz`，Linux 复用 AppImage。
- **CI latest.json 的 URL 用 `REPO_URL/{产物文件名}` 直拼**：产物收集阶段已改名 `{tag}-{platform}-{文件名}`，URL 模板不能再拼一次前缀，否则下载 404（v0.5.1 曾因此翻车）。
- **主题切换只改 `html` class 与 CSS 变量**：浅色令牌在 `index.css` 的 `html.theme-light` 选择器下整组覆盖，`prefers-color-scheme: light` 媒体查询处理 system 模式；新增颜色令牌时两套都要维护。
- **UI 文案不硬编码**：新增文案先加进 `lib/i18n.ts` 的 zh/en 字典，组件里用 `useTranslator()` 取；Rust 侧错误消息仍为中文（服务端语义）。

## 版本陷阱记录

### Tauri 2

- Windows capability 的 opener scope 放在 `src-tauri/capabilities/default.json`，dialog 权限需 `dialog:allow-open` / `dialog:allow-save`。
- Rust 侧注册 `tauri_plugin_dialog::init()` 与 `tauri_plugin_opener::init()`；OPML 命令在 Rust 内部调用 `blocking_pick_file` / `blocking_save_file`（async command 中不阻塞主线程）。
- CSP 必须显式允许 `img-src https:` 和 `media-src https:`，否则远程封面/音频会被 WebView 拦截。
- turso 数据库在 `.setup()` 中用 `tauri::async_runtime::block_on(db::open_database())` 打开并 `app.manage()` 注入，所有 command 通过 `State<'_, Database>` 获取。

### Preact + Rematch

- TSX 使用 `jsxImportSource: "preact"`。
- Rematch state 不保存 DOM 节点或类实例。
- 组件通过 `useAppSelector` 订阅 store，避免为了 Redux 绑定额外引入 React runtime。

### Tailwind CSS v4

- 通过 `@tailwindcss/vite` 插件接入。
- 设计令牌写在 `src/index.css` 的 `@theme` 中，优先使用语义化 class，不在组件里散落硬编码色值。

## Git 工作流

- 远程：`git@github.com:Lin-H/Rustcast.git`；默认分支 master，M2 功能分支为 `feat/m2-turso-local-db`。
- 旧 iced/rodio 实现保存在历史分支/提交中，当前 Tauri 重构分支不应保留死代码。
- 发布时打 `v*` 标签并推送：CI 自动构建 Windows / Linux / macOS 三平台产物并发布 GitHub Release；含 `-alpha` / `-beta` / `-rc` 的标签自动标记为预发布。
- 当前重构从 `0.2.0` 开始（M2）；M3 对应 `0.3.0`，M4 对应 `0.4.0`。
