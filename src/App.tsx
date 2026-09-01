import { useEffect } from "preact/hooks";
import { TopBar } from "./components/TopBar";
import { Sidebar } from "./components/Sidebar";
import { EpisodeList } from "./components/EpisodeList";
import { PlayerBar } from "./components/PlayerBar";
import { bindAudioEvents } from "./services/audio";
import { dispatch, store, useAppSelector } from "./store";

export function App() {
  const selectedFeed = useAppSelector((state) => state.feed.selectedFeed);

  useEffect(() => {
    void dispatch.feed.load();
  }, []);

  useEffect(() => {
    // 启动时应用持久化的音量与倍速。
    const { volume, playbackRate } = store.getState().player;
    dispatch.player.setVolume(volume);
    dispatch.player.setPlaybackRate(playbackRate);
  }, []);

  useEffect(() => {
    // system 主题模式下跟随操作系统深浅切换。
    const media = window.matchMedia("(prefers-color-scheme: light)");
    const listener = () => dispatch.settings.syncSystemTheme();
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, []);

  useEffect(() => {
    return bindAudioEvents({
      onPlaying: () => dispatch.player.playing(),
      onPause: () => {
        dispatch.player.paused();
        dispatch.player.flushProgress();
      },
      onBuffering: () => dispatch.player.buffering(),
      onEnded: () => {
        dispatch.player.finished();
        dispatch.player.markCompleted();
      },
      onTimeUpdate: (seconds) => {
        dispatch.player.timeUpdated(seconds);
        dispatch.player.scheduleProgressSave(seconds);
      },
      onDurationDiscovered: (seconds) => {
        dispatch.player.durationDiscovered(seconds);
        dispatch.player.durationObserved(seconds);
      },
      onRecoveryStarted: () => dispatch.player.recoveryStarted(),
      onError: (message) => {
        dispatch.player.errorRaised(message);
        dispatch.player.flushProgress();
      },
    });
  }, []);

  return (
    <div class="flex h-full min-h-0 flex-col bg-root text-primary">
      <TopBar />
      <div class="flex min-h-0 flex-1">
        <Sidebar />
        <EpisodeList />
      </div>
      <PlayerBar fallbackImage={selectedFeed?.feed.logoUrl ?? null} />
    </div>
  );
}
