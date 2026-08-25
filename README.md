# Rustcast

一款使用 **Rust + [iced](https://iced.rs)** 构建的现代化 RSS 播客阅读器。当前处于 **M1 里程碑**：以 RSS 音频（播客）流式播放为核心功能。

![status](https://img.shields.io/badge/version-0.1-blue) ![milestone](https://img.shields.io/badge/milestone-M1%20%E6%B5%81%E5%AA%92%E4%BD%93%E6%92%AD%E6%94%BE-green)

## ✨ 功能特性（M1）

- 📡 **RSS 订阅解析** — 支持 RSS/Atom，自动提取单集、时长、封面与 show notes
- 🔊 **纯流式播放** — 基于 HTTP Range 边下边播，点击即听，无需等待下载
- ⏯️ **完整播放控制** — 播放/暂停、任意位置拖拽跳转（前向 & 后向）、音量百分比调节
- 🎨 **现代化深色 UI** — 双栏布局 + 底部常驻播放条，深色主题配琥珀橙强调色
- 🖼️ **封面 Artwork** — 订阅源列表 / 单集卡片 / 播放条三处封面展示
- 📝 **Show notes 阅读面板** — 播放中的单集在卡片内展示 content:encoded 正文（可滚动）
- 🚄 **分页渲染** — 大订阅源（600+ 集）流畅不卡顿

## 🛠️ 技术栈

| 层 | 选型 | 说明 |
|---|---|---|
| GUI | `iced 0.14` | Elm 架构，Rust 2024 edition |
| 音频输出 | `rodio 0.22` | 新版 `DeviceSinkBuilder` + `Player` API |
| 解码 | rodio 内置 symphonia 后端 | MP3 / AAC |
| 网络流 | `reqwest 0.13 (blocking)` | 流式 body + HTTP Range seek |
| RSS 解析 | `feed-rs 2.4` | 统一 RSS/Atom/JSON Feed 数据模型 |

## 🏗️ 架构

```
┌────────────────────────────────────────────────┐
│ iced 主线程 (Elm: update / view / subscription) │
│   每 250ms 轮询 Arc<Mutex<Snapshot>> 刷新 UI    │
└───────────────┬────────────────────────────────┘
                │ mpsc Command (Load/Seek/Volume…)
┌───────────────▼────────────────────────────────┐
│ audio-engine 线程                               │
│   Player ── Decoder(DecoderBuilder, byte_len)   │
│              └─ BufReader(512KB)                │
│                  └─ HttpStreamSource            │
│                      ├─ Read: 流式响应体         │
│                      └─ Seek: 前向=丢弃读         │
│                             后向=Range 206 重请求 │
└────────────────────────────────────────────────┘
```

关键设计：

- **HttpStreamSource**：实现 `Read + Seek` 的 HTTP 流适配器。前向 seek 用"读并丢弃"，后向 seek 发起新的 Range 请求（校验必须返回 206，防止 CDN 忽略导致错位）。
- **DecoderBuilder 必须声明 `byte_len` + `seekable(true)`**：symphonia 的 MP3 demuxer 做后向 seek 时需要字节总长度估算目标位置，否则报 `ForwardOnly / RandomAccessNotSupported`。
- **独立引擎线程 + 快照轮询**：UI 与音频完全解耦，命令走 mpsc，状态走 `Arc<Mutex<Snapshot>>`。
- **分页渲染**：首屏 60 张卡片，避免大列表每帧全量重建拖垮事件循环。

## 🚀 快速开始

```bash
# 运行（推荐 release 构建，渲染性能显著更好）
cargo run --release

# debug 构建
cargo run
```

默认加载测试订阅源 Syntax FM（`https://feed.syntax.fm/`），启动后点击任意单集即可播放。

### 引擎探针（无头回归工具）

修改音频链路代码后建议跑一遍：

```bash
cargo run --example engine_probe     # 全链路：加载→音量→前向→两次后向 seek
cargo run --example backward_probe   # 复刻生产管线，专测后向 seek
cargo run --example seek_probe       # 字节级验证 Range 正确性
```

## 📂 目录结构

```
src/
├── main.rs      # iced 应用：状态机、双栏布局、底部播放条
├── lib.rs       # 库入口（供 examples 复用）
├── feed.rs      # feed-rs 解析、HTML 剥离、数据模型
├── player.rs    # 音频引擎线程、HttpStreamSource、命令协议
└── theme.rs     # 设计令牌（深色系 + 琥珀橙 #FFB454）与组件样式
examples/        # 无头引擎探针
assets/icons/    # 内嵌 SVG 图标
```

## ⚠️ 已知限制

- 单集缩略图暂用频道 logo 兜底（仅播放中的单集拉取独立封面）
- 进度 seek 依赖服务器支持 HTTP Range（主流播客 CDN 均支持）
- 音量为线性标度，中低段听感变化不明显
- 描述为纯文本（HTML 已剥离），不支持富文本/图片渲染
- Windows 下首次编译依赖较多，需 MSVC Build Tools

## 🗺️ Roadmap

### M2 — 订阅管理（下一个里程碑）
- [ ] 界面内添加 / 删除订阅源
- [ ] SQLite 持久化（rusqlite bundled）：订阅、单集元数据
- [ ] 每集播放进度记忆，重启后"继续收听"
- [ ] 手动刷新订阅

### M3 — 播客体验增强
- [ ] 倍速播放（0.5x ~ 3x，rodio `set_speed`）
- [ ] 快进快退 ±15s 按钮
- [ ] OPML 导入导出
- [ ] 封面磁盘缓存；单集列表懒加载独立封面
- [ ] 音量对数标度（更符合听感）

### M4 — 平台化
- [ ] 系统托盘 / 最小化到托盘
- [ ] 全局媒体键控制
- [ ] 虚拟化长列表（替代分页按钮）
- [ ] 深浅双主题切换
- [ ] 多语言（中文 / English）
