import type { Language } from "../store/models/settings";

type Dict = Record<string, string>;

const zh: Dict = {
  // 顶栏
  appSubtitle: "RSS 音频播放器",
  themeSystem: "跟随系统",
  themeLight: "浅色",
  themeDark: "深色",
  languageZh: "中文",
  languageEn: "English",
  settingsTheme: "主题",
  settingsLanguage: "语言",

  // 侧栏
  subscriptions: "订阅源",
  feedUrlPlaceholder: "RSS / Atom 地址",
  addFeed: "添加订阅源",
  noFeedsHint: "暂无订阅源，粘贴地址添加",
  episodesCount: " 集",
  refreshFeed: "刷新订阅",
  refreshAllSubscriptions: "刷新全部订阅源",
  deleteFeed: "删除订阅",
  confirmDeleteFeed: "确定删除该订阅源吗？相关单集和播放进度将一并删除。",
  importOpml: "导入 OPML",
  exportOpml: "导出 OPML",
  importOpmlTitle: "从 OPML 文件导入订阅源",
  exportOpmlTitle: "导出订阅源为 OPML 文件",
  opmlBusy: "处理中…",
  opmlExportedTo: "已导出到",
  opmlImportedCount: "个订阅源",
  opmlSkippedCount: "个已存在",
  opmlFailedCount: "个失败",
  opmlFirstFailure: "首个失败",
  localDbNote: "订阅和播放进度保存在本地数据库",

  // 单集列表
  allEpisodes: "全部单集",
  loadingFeed: "正在加载订阅…",
  loadingShort: "加载中…",
  refreshFailed: "刷新失败",
  noFeedsPrompt: "暂无订阅源，请在左侧添加 RSS / Atom 地址",
  noEpisodes: "没有可显示的单集",
  prevPage: "上一页",
  nextPage: "下一页",
  pageOf: "页 · 共",
  episodesTotal: "集",
  pageLabel: "第",

  // 单集卡片
  noAudioTitle: "该单集没有可播放的音频",
  durationUnknown: "时长未知",
  finishedBadge: "已播完",
  playedPercent: "已播",
  lastPlayedAt: "上次听到",
  badgeUnplayable: "无法播放",
  badgePlaying: "播放中",
  badgePaused: "已暂停",

  // 播放条
  playerPickHint: "在上方选择一集，即可开始流式收听",
  statusRecovering: "网络恢复中…",
  statusBuffering: "缓冲中…",
  statusFinished: "已播完",
  statusPlaying: "正在播放",
  statusPaused: "已暂停",
  progressLabel: "播放进度",
  back15: "快退 15 秒",
  forward15: "快进 15 秒",
  togglePause: "暂停",
  togglePlay: "播放",
  speedToggle: "切换倍速",
  speedLabel: "播放倍速（点击切换）",
  volumeLabel: "音量",
  offlineAvailable: "离线可用",
  offlineAvailableTitle: "本集已完整缓存到本地，可离线播放",

  // 其他
  dateUnknown: "日期未知",
  artworkPlaceholder: "播",

  updateCheck: "检查更新",
  updateAvailable: "发现新版本",
  updateNow: "立即更新",
  updateLater: "稍后提醒",
  updateDownloading: "正在下载更新…",
  updateInstalling: "安装中，即将重启…",
  updateCheckFailed: "检查更新失败",
};

const en: Dict = {
  appSubtitle: "RSS Audio Player",
  themeSystem: "System",
  themeLight: "Light",
  themeDark: "Dark",
  languageZh: "中文",
  languageEn: "English",
  settingsTheme: "Theme",
  settingsLanguage: "Language",

  subscriptions: "Subscriptions",
  feedUrlPlaceholder: "RSS / Atom URL",
  addFeed: "Add subscription",
  noFeedsHint: "No subscriptions yet, paste a URL to add",
  episodesCount: " eps",
  refreshFeed: "Refresh subscription",
  refreshAllSubscriptions: "Refresh all subscriptions",
  deleteFeed: "Delete subscription",
  confirmDeleteFeed: "Delete this subscription? Its episodes and playback progress will be removed too.",
  importOpml: "Import OPML",
  exportOpml: "Export OPML",
  importOpmlTitle: "Import subscriptions from an OPML file",
  exportOpmlTitle: "Export subscriptions as an OPML file",
  opmlBusy: "Working…",
  opmlExportedTo: "Exported to",
  opmlImportedCount: "subscription(s) imported",
  opmlSkippedCount: "already exist",
  opmlFailedCount: "failed",
  opmlFirstFailure: "first failure",
  localDbNote: "Subscriptions and progress are stored locally",

  allEpisodes: "All Episodes",
  loadingFeed: "Loading subscription…",
  loadingShort: "Loading…",
  refreshFailed: "Refresh failed",
  noFeedsPrompt: "No subscriptions yet. Add an RSS / Atom URL on the left",
  noEpisodes: "No episodes to display",
  prevPage: "Previous page",
  nextPage: "Next page",
  pageOf: "of",
  episodesTotal: "episodes",
  pageLabel: "Page",

  noAudioTitle: "This episode has no playable audio",
  durationUnknown: "Unknown duration",
  finishedBadge: "Finished",
  playedPercent: "Played",
  lastPlayedAt: "Last played at",
  badgeUnplayable: "Unplayable",
  badgePlaying: "Playing",
  badgePaused: "Paused",

  playerPickHint: "Pick an episode above to start streaming",
  statusRecovering: "Recovering…",
  statusBuffering: "Buffering…",
  statusFinished: "Finished",
  statusPlaying: "Playing",
  statusPaused: "Paused",
  progressLabel: "Playback progress",
  back15: "Back 15 seconds",
  forward15: "Forward 15 seconds",
  togglePause: "Pause",
  togglePlay: "Play",
  speedToggle: "Toggle playback speed",
  speedLabel: "Playback speed (click to toggle)",
  volumeLabel: "Volume",
  offlineAvailable: "Offline",
  offlineAvailableTitle: "This episode is fully cached locally and can be played offline",

  dateUnknown: "Unknown date",
  artworkPlaceholder: "P",

  updateCheck: "Check for updates",
  updateAvailable: "New version available",
  updateNow: "Update now",
  updateLater: "Later",
  updateDownloading: "Downloading update…",
  updateInstalling: "Installing, restarting…",
  updateCheckFailed: "Update check failed",
};

const dicts: Record<Language, Dict> = { zh, en };

/** 取当前语言的文案；缺失时回落中文。 */
export function t(language: Language, key: string): string {
  return dicts[language][key] ?? zh[key] ?? key;
}

export type Translator = (key: string) => string;

export function makeTranslator(language: Language): Translator {
  return (key: string) => t(language, key);
}

/** 日期格式化随语言切换。 */
export function formatDateByLang(timestampSeconds: number, language: Language): string {
  if (!Number.isFinite(timestampSeconds) || timestampSeconds <= 0) {
    return t(language, "dateUnknown");
  }
  const locale = language === "zh" ? "zh-CN" : "en-US";
  return new Intl.DateTimeFormat(locale, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date(timestampSeconds * 1000));
}
