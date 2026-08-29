import { createModel } from "@rematch/core";
import { audioPlayer } from "../../services/audio";
import { saveProgress } from "../../services/tauri";
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
  error: string | null;
  scrubbing: boolean;
  scrubValue: number;
}

const initialState: PlayerState = {
  episode: null,
  isPlaying: false,
  buffering: false,
  recovering: false,
  finished: false,
  currentTime: 0,
  duration: 0,
  volume: 1,
  error: null,
  scrubbing: false,
  scrubValue: 0,
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

      try {
        await audioPlayer.load(episode.audioUrl, resumeSeconds);
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
    setVolume(volume: number): void {
      audioPlayer.setVolume(volume);
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
  }),
});
