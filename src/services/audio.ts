export interface AudioEventHandlers {
  onPlaying: () => void;
  onPause: () => void;
  onBuffering: () => void;
  onEnded: () => void;
  onTimeUpdate: (seconds: number) => void;
  onDurationDiscovered: (seconds: number) => void;
  onError: (message: string) => void;
}

const audio = new Audio();
audio.preload = "metadata";

function mediaErrorMessage(): string {
  const { error } = audio;
  if (!error) {
    return "音频播放失败";
  }

  switch (error.code) {
    case MediaError.MEDIA_ERR_ABORTED:
      return "音频加载被中断";
    case MediaError.MEDIA_ERR_NETWORK:
      return "音频网络加载失败";
    case MediaError.MEDIA_ERR_DECODE:
      return "音频解码失败";
    case MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED:
      return "当前 WebView 不支持该音频格式";
    default:
      return "音频播放失败";
  }
}

export function bindAudioEvents(handlers: AudioEventHandlers): () => void {
  const listeners: Array<[string, EventListener]> = [
    ["play", () => handlers.onPlaying()],
    ["playing", () => handlers.onPlaying()],
    ["pause", () => handlers.onPause()],
    ["waiting", () => handlers.onBuffering()],
    ["ended", () => handlers.onEnded()],
    ["timeupdate", () => handlers.onTimeUpdate(audio.currentTime)],
    ["durationchange", () => handlers.onDurationDiscovered(audio.duration)],
    ["error", () => handlers.onError(mediaErrorMessage())],
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
  async load(url: string): Promise<void> {
    audio.pause();
    audio.src = url;
    audio.load();
    await audio.play();
  },
  async toggle(): Promise<void> {
    if (audio.paused) {
      await audio.play();
    } else {
      audio.pause();
    }
  },
  seek(seconds: number): void {
    if (Number.isFinite(audio.duration)) {
      audio.currentTime = Math.min(Math.max(seconds, 0), audio.duration);
    }
  },
  setVolume(volume: number): void {
    audio.volume = volume;
  },
};
