import { EpisodeCard } from "./EpisodeCard";
import { BrandIcon } from "./icons";
import { dispatch, useAppSelector } from "../store";
import { PAGE_SIZE } from "../store/models/feed";
import type { EpisodeDto } from "../types";

/** 生成页码窗口：当前页 ±1，越界部分用 0 占位表示省略号。 */
function pageWindow(current: number, total: number): number[] {
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i + 1);
  }

  const pages = new Set<number>([1, total, current - 1, current, current + 1]);
  const sorted = [...pages].filter((p) => p >= 1 && p <= total).sort((a, b) => a - b);

  const out: number[] = [];
  let prev = 0;
  for (const p of sorted) {
    if (p - prev > 1) {
      out.push(0);
    }
    out.push(p);
    prev = p;
  }
  return out;
}

export function EpisodeList() {
  const feeds = useAppSelector((state) => state.feed.feeds);
  const selectedFeed = useAppSelector((state) => state.feed.selectedFeed);
  const loading = useAppSelector((state) => state.feed.loading);
  const page = useAppSelector((state) => state.feed.page);
  const refreshError = useAppSelector((state) => state.feed.refreshError);
  const currentEpisode = useAppSelector((state) => state.player.episode);
  const isPlaying = useAppSelector((state) => state.player.isPlaying);

  const summary = selectedFeed?.feed ?? null;
  const fallbackImage = summary?.logoUrl ?? null;
  const episodes = selectedFeed?.episodes ?? [];
  const totalPages = Math.max(1, Math.ceil(episodes.length / PAGE_SIZE));
  const safePage = Math.min(page, totalPages);
  const visibleEpisodes = episodes.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE);

  const playEpisode = (episode: EpisodeDto) => {
    dispatch.player.playEpisode(episode);
  };

  if (loading && episodes.length === 0) {
    return (
      <main class="flex h-full min-w-0 flex-1 items-center justify-center p-6">
        <div class="flex flex-col items-center gap-3">
          <BrandIcon className="h-12 w-12 animate-pulse text-accent" />
          <p class="text-[13px] text-secondary">正在加载订阅…</p>
        </div>
      </main>
    );
  }

  return (
    <main class="h-full min-w-0 flex-1 overflow-y-auto pt-4 pl-2 pr-5">
      <div class="mb-3 flex items-center">
        <h2 class="truncate text-base font-bold text-primary">{summary?.title ?? "全部单集"}</h2>
        <span class="ml-auto shrink-0 text-xs text-faint">
          {loading ? "加载中…" : `${episodes.length} 集`}
        </span>
      </div>

      {refreshError !== null && (
        <div class="mb-3 rounded-lg border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger">
          刷新失败：{refreshError}
        </div>
      )}

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

        {episodes.length > 0 && (
          <div class="mt-2 flex flex-wrap items-center justify-center gap-1 pb-2">
            <button
              type="button"
              disabled={safePage <= 1}
              class="grid h-8 w-8 cursor-pointer place-items-center rounded-lg text-[12px] text-secondary transition-colors hover:bg-card-hover hover:text-accent disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent"
              onClick={() => dispatch.feed.setPage(safePage - 1)}
              aria-label="上一页"
              title="上一页"
            >
              ‹
            </button>

            {pageWindow(safePage, totalPages).map((p, index) =>
              p === 0 ? (
                <span key={`ellipsis-${index}`} class="px-1 text-[12px] text-faint">
                  …
                </span>
              ) : (
                <button
                  key={p}
                  type="button"
                  class={`h-8 min-w-8 cursor-pointer rounded-lg px-2 text-[12.5px] font-medium transition-colors ${
                    p === safePage
                      ? "bg-accent font-bold text-root"
                      : "text-secondary hover:bg-card-hover hover:text-accent"
                  }`}
                  onClick={() => dispatch.feed.setPage(p)}
                  aria-label={`第 ${p} 页`}
                  aria-current={p === safePage ? "page" : undefined}
                >
                  {p}
                </button>
              ),
            )}

            <button
              type="button"
              disabled={safePage >= totalPages}
              class="grid h-8 w-8 cursor-pointer place-items-center rounded-lg text-[12px] text-secondary transition-colors hover:bg-card-hover hover:text-accent disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent"
              onClick={() => dispatch.feed.setPage(safePage + 1)}
              aria-label="下一页"
              title="下一页"
            >
              ›
            </button>

            <span class="ml-2 shrink-0 text-[11px] text-faint">
              第 {safePage} / {totalPages} 页 · 共 {episodes.length} 集
            </span>
          </div>
        )}

        {!loading && feeds.length === 0 && (
          <div class="py-10 text-center text-sm text-faint">
            暂无订阅源，请在左侧添加 RSS / Atom 地址
          </div>
        )}

        {!loading && feeds.length > 0 && episodes.length === 0 && (
          <div class="py-10 text-center text-sm text-faint">没有可显示的单集</div>
        )}
      </div>
    </main>
  );
}
