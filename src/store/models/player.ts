import { createModel } from "@rematch/core";
import { audioPlayer } from "../../services/audio";
import {
  ensureAudioCache,
  listenAudioCacheProgress,
  mediaUrl,
  saveProgress,
} from "../../services/tauri";
import type { EpisodeDto } from "../../types";
import { store, type RootModel } from "../index";

export interface PlayerState {
  episode: EpisodeDto | null;
  isPlaying: boolean;
  buffering: boolean;
  recovering: boolean;
  finished: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  playbackRate: number;
  error: string | null;
  scrubbing: boolean;
  scrubValue: number;
  /** 当前集音频缓存进度（written 字节）；null = 未在缓存。 */
  cacheWritten: number | null;
  cacheTotal: number | null;
  cacheComplete: boolean;
}

const PLAYBACK_RATE_KEY = "rustcast.playbackRate";
const VOLUME_KEY = "rustcast.volume";

function readStoredNumber(key: string, fallback: number): number {
  try {
    const raw = window.localStorage.getItem(key);
    if (raw === null) {
      return fallback;
    }
    const value = Number(raw);
    return Number.isFinite(value) ? value : fallback;
  } catch {
    return fallback;
  }
}

function storeNumber(key: string, value: number): void {
  try {
    window.localStorage.setItem(key, String(value));
  } catch {
    // localStorage 不可用时静默降级。
  }
}

const initialVolume = Math.min(Math.max(readStoredNumber(VOLUME_KEY, 1), 0), 1);
const initialPlaybackRate = Math.min(Math.max(readStoredNumber(PLAYBACK_RATE_KEY, 1), 0.25), 4);

const initialState: PlayerState = {
  episode: null,
  isPlaying: false,
  buffering: false,
  recovering: false,
  finished: false,
  currentTime: 0,
  duration: 0,
  volume: initialVolume,
  playbackRate: initialPlaybackRate,
  error: null,
  scrubbing: false,
  scrubValue: 0,
  cacheWritten: null,
  cacheTotal: null,
  cacheComplete: false,
};

const PROGRESS_SAVE_INTERVAL_MS = 5_000;

interface SaveContext {
  episodeId: string;
  duration: number | null;
}

let saveContext: SaveContext | null = null;
let pendingPosition: number | null = null;
let saveTimer: number | null = null;
let completedSaved = false;

function clearSaveTimer(): void {
  if (saveTimer !== null) {
    window.clearTimeout(saveTimer);
    saveTimer = null;
  }
}

async function persistProgress(completed: boolean): Promise<void> {
  if (saveContext === null) {
    return;
  }

  const context = saveContext;
  const position = pendingPosition ?? 0;
  const duration = context.duration;

  try {
    await saveProgress({
      episodeId: context.episodeId,
      positionSecs: completed
        ? duration !== null
          ? duration
          : position
        : position,
      durationSecs: duration,
      completed,
    });
  } catch (error) {
    console.warn("保存播放进度失败", error);
  }
}

export const playerModel = createModel<RootModel>()({
  state: initialState,
  reducers: {
    episodeSelected(state, episode: EpisodeDto): PlayerState {
      return {
        ...state,
        episode,
        isPlaying: true,
        buffering: true,
        recovering: false,
        finished: false,
        currentTime: 0,
        duration: episode.durationSecs ?? 0,
        error: null,
        scrubbing: false,
        scrubValue: 0,
      };
    },
    playing(state): PlayerState {
      return { ...state, isPlaying: true, buffering: false, recovering: false };
    },
    paused(state): PlayerState {
      return { ...state, isPlaying: false };
    },
    buffering(state): PlayerState {
      return { ...state, buffering: true };
    },
    recoveryStarted(state): PlayerState {
      return {
        ...state,
        isPlaying: true,
        buffering: true,
        recovering: true,
        error: null,
      };
    },
    finished(state): PlayerState {
      return { ...state, isPlaying: false, buffering: false, finished: true };
    },
    timeUpdated(state, seconds: number): PlayerState {
      return { ...state, currentTime: seconds };
    },
    durationDiscovered(state, seconds: number): PlayerState {
      const duration =
        Number.isFinite(seconds) && seconds > 0 ? seconds : state.duration;
      return { ...state, duration };
    },
    volumeSet(state, volume: number): PlayerState {
      return { ...state, volume };
    },
    rateSet(state, rate: number): PlayerState {
      return { ...state, playbackRate: rate };
    },
    cacheProgressUpdated(
      state,
      payload: { episodeId: string; written: number; total: number | null; complete: boolean },
    ): PlayerState {
      // 只接受当前播放集的事件，切集后旧集后台下载事件被忽略（但仍在下载）。
      if (state.episode?.id !== payload.episodeId) {
        return state;
      }
      return {
        ...state,
        cacheWritten: payload.written,
        cacheTotal: payload.total,
        cacheComplete: payload.complete,
      };
    },
    cacheReset(state): PlayerState {
      return { ...state, cacheWritten: null, cacheTotal: null, cacheComplete: false };
    },
    errorRaised(state, error: string): PlayerState {
      return {
        ...state,
        isPlaying: false,
        buffering: false,
        recovering: false,
        error,
      };
    },
    scrubStarted(state): PlayerState {
      return { ...state, scrubbing: true, scrubValue: state.currentTime };
    },
    scrubMoved(state, seconds: number): PlayerState {
      const max = state.duration > 0 ? state.duration : Number.MAX_SAFE_INTEGER;
      return {
        ...state,
        scrubbing: true,
        scrubValue: Math.min(Math.max(seconds, 0), max),
      };
    },
    scrubCommitted(state): PlayerState {
      return { ...state, scrubbing: false };
    },
  },
  effects: (dispatch) => ({
    async playEpisode(episode: EpisodeDto): Promise<void> {
      if (store.getState().player.episode?.id === episode.id) {
        return;
      }

      if (episode.audioUrl === null) {
        return;
      }

      if (saveContext !== null) {
        clearSaveTimer();
        void persistProgress(false);
      }

      dispatch.player.episodeSelected(episode);
      dispatch.player.cacheReset();
      completedSaved = false;
      saveContext = { episodeId: episode.id, duration: episode.durationSecs };

      const progress = episode.progress;
      const resumeSeconds =
        progress !== null && !progress.completed && progress.positionSecs > 5
          ? Math.min(
              progress.positionSecs,
              progress.durationSecs ?? Number.POSITIVE_INFINITY,
            )
          : 0;
      pendingPosition = resumeSeconds > 0 ? resumeSeconds : null;

      if (resumeSeconds > 0) {
        dispatch.player.timeUpdated(resumeSeconds);
      }

      // 音频缓存：注册并启动下载，之后走 rustcast-media:// 协议播放。
      // 失败时回落远程 URL 直连，不影响播放。
      let sourceUrl = episode.audioUrl;
      try {
        const status = await ensureAudioCache(episode.id, episode.audioUrl);
        dispatch.player.cacheProgressUpdated({
          episodeId: episode.id,
          written: status.written,
          total: status.total,
          complete: status.complete,
        });
        sourceUrl = mediaUrl(episode.id);
      } catch (error) {
        console.warn("音频缓存不可用，回落远程直连", error);
      }

      try {
        await audioPlayer.load(sourceUrl, resumeSeconds);
      } catch (error) {
        if (error instanceof DOMException && error.name === "AbortError") {
          return;
        }
        dispatch.player.errorRaised(
          error instanceof Error ? error.message : "音频播放失败",
        );
      }
    },
    async toggle(): Promise<void> {
      try {
        if (audioPlayer.isPaused()) {
          completedSaved = false;
        }
        await audioPlayer.toggle();
      } catch (error) {
        if (error instanceof DOMException && error.name === "AbortError") {
          return;
        }
        dispatch.player.errorRaised(
          error instanceof Error ? error.message : "音频播放失败",
        );
      }
    },
    seek(seconds: number): void {
      audioPlayer.seek(seconds);
    },
    skip(deltaSeconds: number): void {
      audioPlayer.skip(deltaSeconds);
      dispatch.player.timeUpdated(audioPlayer.getCurrentTime());
    },
    setVolume(volume: number): void {
      audioPlayer.setVolume(volume);
      storeNumber(VOLUME_KEY, volume);
    },
    setPlaybackRate(rate: number): void {
      audioPlayer.setPlaybackRate(rate);
      storeNumber(PLAYBACK_RATE_KEY, rate);
    },
    scheduleProgressSave(seconds: number): void {
      if (completedSaved) {
        return;
      }

      pendingPosition = seconds;
      if (saveTimer !== null) {
        return;
      }

      saveTimer = window.setTimeout(() => {
        saveTimer = null;
        void persistProgress(false);
      }, PROGRESS_SAVE_INTERVAL_MS);
    },
    flushProgress(): void {
      clearSaveTimer();
      if (completedSaved || pendingPosition === null || pendingPosition < 1) {
        return;
      }
      void persistProgress(false);
    },
    markCompleted(): void {
      clearSaveTimer();
      completedSaved = true;
      void persistProgress(true);
    },
    durationObserved(seconds: number): void {
      if (saveContext !== null && Number.isFinite(seconds) && seconds > 0) {
        saveContext = { ...saveContext, duration: seconds };
      }
    },
    attachCacheListener(): void {
      // 进度事件由 App 层挂载一次；这里作为 effect 供外部显式调用。
      if (cacheListenerUnlisten !== null) {
        return;
      }
      cacheListenerUnlisten = listenAudioCacheProgress((event) => {
        dispatch.player.cacheProgressUpdated(event);
      });
    },
  }),
});

let cacheListenerUnlisten: (() => void) | null = null;
