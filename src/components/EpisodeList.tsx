import { EpisodeCard } from "./EpisodeCard";
import { BrandIcon } from "./icons";
import { dispatch, useAppSelector } from "../store";
import type { EpisodeDto } from "../types";

interface EpisodeListProps {
  fallbackImage: string | null;
}

export function EpisodeList({ fallbackImage }: EpisodeListProps) {
  const feed = useAppSelector((state) => state.feed.feed);
  const loading = useAppSelector((state) => state.feed.loading);
  const visibleCount = useAppSelector((state) => state.feed.visibleCount);
  const currentEpisode = useAppSelector((state) => state.player.episode);
  const isPlaying = useAppSelector((state) => state.player.isPlaying);

  const episodes = feed?.episodes ?? [];
  const visibleEpisodes = episodes.slice(0, visibleCount);
  const remaining = episodes.length - visibleEpisodes.length;

  const playEpisode = (episode: EpisodeDto) => {
    dispatch.player.playEpisode(episode);
  };

  if (loading && episodes.length === 0) {
    return (
      <main class="flex h-full min-w-0 flex-1 items-center justify-center p-6">
        <div class="flex flex-col items-center gap-3">
          <BrandIcon className="h-12 w-12 animate-pulse text-accent" />
          <p class="text-[13px] text-secondary">正在拉取 Syntax FM 订阅…</p>
        </div>
      </main>
    );
  }

  return (
    <main class="h-full min-w-0 flex-1 overflow-y-auto pt-4 pl-2 pr-5">
      <div class="mb-3 flex items-center">
        <h2 class="text-base font-bold text-primary">全部单集</h2>
        <span class="ml-auto text-xs text-faint">
          {loading ? "加载中…" : `${episodes.length} 集`}
        </span>
      </div>

      <div class="flex flex-col gap-2.5 pb-5">
        {visibleEpisodes.map((episode) => (
          <EpisodeCard
            key={episode.id}
            episode={episode}
            fallbackImage={fallbackImage}
            isCurrent={currentEpisode?.id === episode.id}
            isPlaying={currentEpisode?.id === episode.id && isPlaying}
            onPlay={() => playEpisode(episode)}
          />
        ))}

        {remaining > 0 && (
          <button
            type="button"
            class="mx-auto cursor-pointer rounded-lg px-[18px] py-2.5 text-[13px] text-secondary transition-colors hover:bg-card-hover active:bg-elevated"
            onClick={() => dispatch.feed.showMore()}
          >
            显示更多单集（还有 {remaining} 集）
          </button>
        )}

        {!loading && episodes.length === 0 && (
          <div class="py-10 text-center text-sm text-faint">没有可显示的单集</div>
        )}
      </div>
    </main>
  );
}
