import { useEffect } from "preact/hooks";
import { TopBar } from "./components/TopBar";
import { Sidebar } from "./components/Sidebar";
import { EpisodeList } from "./components/EpisodeList";
import { PlayerBar } from "./components/PlayerBar";
import { bindAudioEvents } from "./services/audio";
import { dispatch, useAppSelector } from "./store";

export function App() {
  const feed = useAppSelector((state) => state.feed.feed);
  const loading = useAppSelector((state) => state.feed.loading);
  const error = useAppSelector((state) => state.feed.error);

  useEffect(() => {
    void dispatch.feed.load();
  }, []);

  useEffect(() => {
    return bindAudioEvents({
      onPlaying: () => dispatch.player.playing(),
      onPause: () => dispatch.player.paused(),
      onBuffering: () => dispatch.player.buffering(),
      onEnded: () => dispatch.player.finished(),
      onTimeUpdate: (seconds) => dispatch.player.timeUpdated(seconds),
      onDurationDiscovered: (seconds) => dispatch.player.durationDiscovered(seconds),
      onError: (message) => dispatch.player.errorRaised(message),
    });
  }, []);

  return (
    <div class="flex h-full min-h-0 flex-col bg-root text-primary">
      <TopBar />
      <div class="flex min-h-0 flex-1">
        <Sidebar feed={feed} loading={loading} error={error} />
        <EpisodeList fallbackImage={feed?.logoUrl ?? null} />
      </div>
      <PlayerBar fallbackImage={feed?.logoUrl ?? null} />
    </div>
  );
}
