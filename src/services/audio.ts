export interface AudioEventHandlers {
  onPlaying: () => void;
  onPause: () => void;
  onBuffering: () => void;
  onEnded: () => void;
  onTimeUpdate: (seconds: number) => void;
  onDurationDiscovered: (seconds: number) => void;
  onRecoveryStarted: () => void;
  onError: (message: string) => void;
}

const audio = new Audio();
audio.preload = "auto";

export const PLAYBACK_RATES = [0.75, 1, 1.25, 1.5, 1.75, 2] as const;

const MAX_RECOVERY_ATTEMPTS = 8;
const STALL_TIMEOUT_MS = 8_000;
const MAX_RECOVERY_DELAY_MS = 8_000;

let activeUrl: string | null = null;
let desiredPlaying = false;
let recovering = false;
let recoveryAttempts = 0;
let pendingResumeSeconds = 0;
let lastObservedSeconds = 0;
let lastStallCheckSeconds = 0;
let loadToken = 0;
let stallTimer: number | null = null;
let recoveryDelayTimer: number | null = null;
let eventHandlers: AudioEventHandlers | null = null;
let playbackRate = 1;
let volume = 1;

function clearTimer(timer: number | null): void {
  if (timer !== null) {
    window.clearTimeout(timer);
  }
}

function clearRecoveryTimers(): void {
  clearTimer(stallTimer);
  clearTimer(recoveryDelayTimer);
  stallTimer = null;
  recoveryDelayTimer = null;
}

function clampProgress(seconds: number): number {
  const max = Number.isFinite(audio.duration) ? Math.max(0, audio.duration - 0.25) : seconds;
  return Math.min(Math.max(seconds, 0), max);
}

function applyPendingResume(): void {
  if (pendingResumeSeconds > 0 && Number.isFinite(audio.duration) && audio.duration > 0) {
    audio.currentTime = clampProgress(pendingResumeSeconds);
    pendingResumeSeconds = 0;
  }
}

function mediaFailure(): { message: string; recoverable: boolean } {
  const { error } = audio;
  if (!error) {
    return { message: "音频连接中断", recoverable: true };
  }

  switch (error.code) {
    case MediaError.MEDIA_ERR_ABORTED:
      return { message: "音频加载被中断", recoverable: true };
    case MediaError.MEDIA_ERR_NETWORK:
      return { message: "音频网络加载失败", recoverable: true };
    case MediaError.MEDIA_ERR_DECODE:
      return { message: "音频解码失败", recoverable: false };
    case MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED:
      return { message: "当前 WebView 不支持该音频格式", recoverable: false };
    default:
      return { message: "音频播放失败", recoverable: true };
  }
}

function notePlaybackRestored(): void {
  recovering = false;
  recoveryAttempts = 0;
  clearRecoveryTimers();
  lastStallCheckSeconds = audio.currentTime;
}

function noteProgress(seconds: number): void {
  lastObservedSeconds = seconds;

  if (Math.abs(seconds - lastStallCheckSeconds) >= 0.25) {
    lastStallCheckSeconds = seconds;
    clearTimer(stallTimer);
    stallTimer = null;

    if (recovering && audio.readyState >= HTMLMediaElement.HAVE_FUTURE_DATA) {
      recovering = false;
      recoveryAttempts = 0;
    }
  }
}

function resetSource(url: string, resumeSeconds: number): void {
  const token = ++loadToken;
  pendingResumeSeconds = resumeSeconds;
  audio.pause();
  audio.src = url;
  audio.playbackRate = playbackRate;
  audio.volume = volume * volume;
  audio.load();

  if (resumeSeconds > 0) {
    try {
      // WebView 允许在 metadata 前设置默认播放位置；这样恢复重连时不会先落到 0。
      audio.currentTime = resumeSeconds;
      pendingResumeSeconds = 0;
    } catch {
      // 若当前 WebView 不支持该时机，loadedmetadata/durationchange 仍会补设位置。
    }
  }

  void audio.play().catch((error: unknown) => {
    if (token !== loadToken) {
      return;
    }

    if (error instanceof DOMException && error.name === "AbortError") {
      return;
    }

    desiredPlaying = false;
    recovering = false;
    clearRecoveryTimers();
    eventHandlers?.onError(
      error instanceof DOMException && error.name === "NotAllowedError"
        ? "浏览器阻止了音频自动播放，请再次点击播放"
        : "音频播放失败",
    );
  });
}

function beginRecovery(reason: "media-error" | "stall" | "manual"): boolean {
  if (activeUrl === null || !desiredPlaying) {
    return false;
  }

  if (reason === "media-error" && !mediaFailure().recoverable) {
    return false;
  }

  if (reason !== "manual") {
    if (recovering) {
      return true;
    }

    if (recoveryAttempts >= MAX_RECOVERY_ATTEMPTS) {
      desiredPlaying = false;
      clearRecoveryTimers();
      return false;
    }

    recoveryAttempts += 1;
  } else {
    recoveryAttempts = 0;
  }

  recovering = true;
  clearRecoveryTimers();

  const resumeSeconds = clampProgress(lastObservedSeconds);
  const delay =
    reason === "manual"
      ? 0
      : Math.min(MAX_RECOVERY_DELAY_MS, 1_000 * 2 ** (recoveryAttempts - 1));

  const reconnect = () => {
    if (activeUrl === null || !desiredPlaying) {
      return;
    }

    resetSource(activeUrl, resumeSeconds);
  };

  if (delay === 0) {
    reconnect();
  } else {
    recoveryDelayTimer = window.setTimeout(reconnect, delay);
  }

  eventHandlers?.onRecoveryStarted();
  return true;
}

function scheduleStallRecovery(): void {
  if (!desiredPlaying || activeUrl === null || recovering) {
    return;
  }

  clearTimer(stallTimer);
  stallTimer = window.setTimeout(() => {
    stallTimer = null;

    if (!desiredPlaying || activeUrl === null || recovering || audio.paused) {
      return;
    }

    if (audio.readyState >= HTMLMediaElement.HAVE_FUTURE_DATA) {
      return;
    }

    if (!beginRecovery("stall")) {
      eventHandlers?.onError("网络长时间不稳定，已停止自动恢复；再次点击播放会保留进度重试");
    }
  }, STALL_TIMEOUT_MS);
}

audio.addEventListener("loadedmetadata", applyPendingResume);
audio.addEventListener("durationchange", applyPendingResume);
audio.addEventListener("progress", () => {
  if (!recovering && desiredPlaying && audio.readyState < HTMLMediaElement.HAVE_FUTURE_DATA) {
    scheduleStallRecovery();
  }
});

export function bindAudioEvents(handlers: AudioEventHandlers): () => void {
  eventHandlers = handlers;
  const listeners: Array<[string, EventListener]> = [
    ["play", () => handlers.onPlaying()],
    ["playing", () => {
      notePlaybackRestored();
      handlers.onPlaying();
    }],
    ["pause", () => handlers.onPause()],
    ["waiting", () => {
      handlers.onBuffering();
      scheduleStallRecovery();
    }],
    ["stalled", () => {
      handlers.onBuffering();
      scheduleStallRecovery();
    }],
    ["ended", () => handlers.onEnded()],
    ["timeupdate", () => {
      noteProgress(audio.currentTime);
      handlers.onTimeUpdate(audio.currentTime);
    }],
    ["durationchange", () => handlers.onDurationDiscovered(audio.duration)],
    ["error", () => {
      const failure = mediaFailure();

      if (beginRecovery("media-error")) {
        handlers.onRecoveryStarted();
        return;
      }

      handlers.onError(failure.message);
    }],
  ];

  for (const [type, listener] of listeners) {
    audio.addEventListener(type, listener);
  }

  return () => {
    for (const [type, listener] of listeners) {
      audio.removeEventListener(type, listener);
    }
  };
}

export const audioPlayer = {
  async load(url: string, resumeSeconds = 0): Promise<void> {
    activeUrl = url;
    desiredPlaying = true;
    recovering = false;
    recoveryAttempts = 0;
    pendingResumeSeconds = resumeSeconds;
    lastObservedSeconds = resumeSeconds;
    lastStallCheckSeconds = resumeSeconds;
    clearRecoveryTimers();
    resetSource(url, resumeSeconds);
  },
  async toggle(): Promise<void> {
    if (audio.paused) {
      desiredPlaying = true;

      if (audio.error !== null || audio.readyState <= HTMLMediaElement.HAVE_NOTHING) {
        beginRecovery("manual");
        return;
      }

      await audio.play();
    } else {
      desiredPlaying = false;
      clearRecoveryTimers();
      audio.pause();
    }
  },
  isPaused(): boolean {
    return audio.paused;
  },
  getCurrentTime(): number {
    return audio.currentTime;
  },
  seek(seconds: number): void {
    if (Number.isFinite(audio.duration)) {
      audio.currentTime = Math.min(Math.max(seconds, 0), audio.duration);
      lastObservedSeconds = audio.currentTime;
      lastStallCheckSeconds = audio.currentTime;
    }
  },
  skip(deltaSeconds: number): void {
    const target = audio.currentTime + deltaSeconds;
    const clamped = Number.isFinite(audio.duration)
      ? Math.min(Math.max(target, 0), audio.duration)
      : Math.max(target, 0);
    this.seek(clamped);
  },
  setVolume(nextVolume: number): void {
    // 感知曲线：滑杆线性位置平方后作为线性增益，低音量段更细腻。
    volume = Math.min(Math.max(nextVolume, 0), 1);
    audio.volume = volume * volume;
  },
  setPlaybackRate(rate: number): void {
    playbackRate = Math.min(Math.max(rate, 0.25), 4);
    audio.playbackRate = playbackRate;
  },
};
