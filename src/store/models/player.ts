import { createModel } from "@rematch/core";
import { audioPlayer } from "../../services/audio";
import type { EpisodeDto } from "../../types";
import type { RootModel } from "../index";

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
      const duration = Number.isFinite(seconds) && seconds > 0 ? seconds : state.duration;
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
      return { ...state, scrubbing: true, scrubValue: Math.min(Math.max(seconds, 0), max) };
    },
    scrubCommitted(state): PlayerState {
      return { ...state, scrubbing: false };
    },
  },
  effects: (dispatch) => ({
    async playEpisode(episode: EpisodeDto): Promise<void> {
      if (episode.audioUrl === null) {
        return;
      }

      dispatch.player.episodeSelected(episode);

      try {
        await audioPlayer.load(episode.audioUrl);
      } catch (error) {
        if (error instanceof DOMException && error.name === "AbortError") {
          return;
        }
        dispatch.player.errorRaised(error instanceof Error ? error.message : "音频播放失败");
      }
    },
    async toggle(): Promise<void> {
      try {
        await audioPlayer.toggle();
      } catch (error) {
        if (error instanceof DOMException && error.name === "AbortError") {
          return;
        }
        dispatch.player.errorRaised(error instanceof Error ? error.message : "音频播放失败");
      }
    },
    seek(seconds: number): void {
      audioPlayer.seek(seconds);
    },
    setVolume(volume: number): void {
      audioPlayer.setVolume(volume);
    },
  }),
});
