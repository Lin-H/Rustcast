# AGENTS.md — AI 协作开发指南

本文件面向在此仓库工作的 AI 编码助手（及新成员），记录项目现状、构建方式、架构决策与踩坑记录。

## 项目概览

- **名称**：Rustcast — Rust + iced 的 RSS 播客阅读器
- **当前里程碑**：M1（硬编码 Syntax FM 测试源 + 流式播放全链路）已完成并发布 v0.1
- **UI 语言**：中文；代码注释保持最少必要

## 构建与验证命令

```bash
cargo build                 # debug 构建
cargo run                   # 运行 GUI（默认加载 https://feed.syntax.fm/）
cargo run --release         # release 构建（UI 明显更流畅，验证性能问题时必须用它）
cargo build --examples      # 编译引擎探针

cargo run --example engine_probe     # 音频全链路回归（加载/音量/前向/后向 seek）
cargo run --example backward_probe   # 后向 seek 专项（复刻生产 Player 管线）
cargo run --example seek_probe       # HTTP Range 字节正确性专项
```

**规则：任何触碰 `src/player.rs` 的改动，提交前必须跑通 `engine_probe` 且全部阶段无 err。**

## 架构地图

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 库入口，导出 feed/player/theme 供 examples 复用 |
| `src/main.rs` | iced 0.14 应用：`App` 状态机、双栏布局、底部播放条、分页逻辑 |
| `src/feed.rs` | feed-rs 解析 → `Feed`/`Episode` 模型；HTML 剥离（strip_html）；封面/正文提取 |
| `src/player.rs` | 音频引擎线程；`Command` 命令协议；`Snapshot` 状态快照；`HttpStreamSource` |
| `src/theme.rs` | 设计令牌常量 + button/container 样式工厂 |

### 数据流

1. UI 线程通过 `PlayerHandle::send(Command)` 下发命令（mpsc）
2. 引擎线程 `recv_timeout(80ms)` 循环处理命令并 `publish_status()` 写入 `Arc<Mutex<Snapshot>>`
3. UI 通过 `iced::time::every(250ms)` 订阅 Tick，克隆快照进 App 状态驱动重绘

### 关键类型

- `HttpStreamSource`（player.rs）：`Read+Seek` over HTTP。前向 seek=丢弃读；后向 seek=新 Range 请求且**校验 206**（防 CDN 忽略 Range 导致字节错位）。字段 `content_length` 为 pub，供 DecoderBuilder 使用。
- `Episode.article`：content:encoded 剥离 HTML 后的全文，播放中卡片内滚动展示。

## ⚠️ 版本陷阱记录（升级依赖前必读）

### rodio 0.22（破坏性大改）
- `OutputStream` / `Sink` / `OutputStreamHandle` 已删除 → 用 `DeviceSinkBuilder::open_default_sink()` + `Player::connect_new(mixer)`
- **后向 seek 必须用 builder**：
  ```rust
  let mut b = rodio::decoder::DecoderBuilder::new()
      .with_data(reader)
      .with_seekable(true);
  if let Some(len) = content_length { b = b.with_byte_len(len); }
  b.build()?
  ```
  直接 `Decoder::try_from(reader)` 时 MP3 只能前向 seek（报 `SymphoniaDecoder(RandomAccessNotSupported)`）。这是 M1 调试最久的坑，探针三件套就是为此而写。
- `Player` 方法：`append/pause/play/is_paused/set_volume/get_pos/try_seek/empty/stop`

### iced 0.14
- 入口签名：`iced::application(boot, update, view)`；boot 可返回 `(State, Task<Message>)`；update 返回 `Into<Task>`。传**自由函数**而非闭包/method path 可避开 HRTB "not general enough" 报错。
- **Slider 默认步长是 1**（`slider.rs`: `step: T::from(1)`）：f32 范围 `0..=1` 会退化成两档开关！必须显式 `.step(0.01)`。
- `font::Font` 没有 size 字段 → 用 `text(...).size(f32)`；加粗用 `Font { weight: Weight::Bold, ..Font::DEFAULT }`
- 无 `horizontal_space/vertical_space` → main.rs 内自建 `vgap/hfill/vfill` 助手
- `button::Style.text_color` 是 `Color` 非 Option，且必须提供 `snap: bool` 字段
- `container::Style` 无 `clip` 字段
- `Color::from_rgb` 参数是 f32：写 `0x1D as f32 / 255.0`
- padding 不接受 `[a,b,c,d]` 数组 → 用 `Padding { top, right, bottom, left }`
- 窗口尺寸用 `Size::new(w, h)`；`.exit_on_close_request(true)` 需显式设置
- view 返回类型需 HRTB 兼容：已用自由函数 `view_root(&App) -> Element` 包装

### feed-rs 2.4
- `Entry` 没有 `enclosures` 和 `duration` 字段！音频在 `entry.media[].content[]`（`MediaContent.url: Option<url::Url>`、`content_type: Option<MediaTypeBuf>`），时长在 `MediaObject.duration` 或 `MediaContent.duration`（均为 std Duration）
- 缩略图：`entry.media[].thumbnails[].image.uri`
- `parser::parse(R: Read)` 已无 uri 参数（2.x 移除）
- `Content.body` 是 `Option<String>`

### reqwest 0.13
- 使用 `--no-default-features --features blocking,native-tls,system-proxy`：Windows 走 schannel 免额外构建依赖（默认的 rustls/aws-lc-sys 在 Windows 可能要求 cmake/NASM）
- blocking `Response` 实现 Read，可直接作为流源

## 设计决策记录

| 决策 | 理由 |
|---|---|
| 纯流式播放而非先下载 | 点击即听；seek 靠 Range 重请求，播客 CDN 全支持 |
| 独立引擎线程而非 iced task 内嵌 | try_seek 会阻塞数秒（网络往返），不能卡 UI 执行器 |
| 快照轮询而非事件推送 | 实现简单；250ms 对进度显示足够 |
| 分页渲染（60 + 步进150） | 600 卡片全量重建曾把 debug 帧率拖到秒级 |
| reqwest native-tls | Windows schannel 零外部依赖 |
| UI 文案中文、设计令牌集中 theme.rs | 用户偏好深色琥珀橙主题 |

## 当前已知问题 / 待办

见 README.md Roadmap。近期优先级：M2 订阅管理 + SQLite + 进度记忆。

## Git 工作流

- 远程：`git@github.com:Lin-H/Rustcast.git`（分支 main）
- 版本标签：`v0.1` 起，发布时打 tag 并推送
