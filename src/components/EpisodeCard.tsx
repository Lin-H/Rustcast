import { Artwork } from "./Artwork";
import { ShowNotes } from "./ShowNotes";
import { formatDate, formatTime } from "../lib/format";
import type { EpisodeDto } from "../types";

interface EpisodeCardProps {
  episode: EpisodeDto;
  fallbackImage: string | null;
  isCurrent: boolean;
  isPlaying: boolean;
  onPlay: () => void;
}

export function EpisodeCard({
  episode,
  fallbackImage,
  isCurrent,
  isPlaying,
  onPlay,
}: EpisodeCardProps) {
  const disabled = episode.audioUrl === null;

  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onPlay}
      class={`w-full rounded-xl p-3 text-left transition-colors ${
        isCurrent
          ? "border border-accent bg-elevated"
          : disabled
            ? "cursor-not-allowed border border-transparent bg-card opacity-70"
            : "cursor-pointer border border-transparent bg-card hover:bg-card-hover active:bg-elevated"
      }`}
      title={disabled ? "该单集没有可播放的音频" : undefined}
    >
      <div class="flex items-center gap-3.5">
        <Artwork
          src={episode.imageUrl}
          fallbackSrc={fallbackImage}
          alt={episode.title}
          className="h-[58px] w-[58px] rounded-[10px]"
        />

        <div class="min-w-0 flex-1">
          <h3
            class={`truncate text-[15px] font-bold ${isCurrent ? "text-accent" : "text-primary"}`}
          >
            {episode.title}
          </h3>
          <div class="mt-1 flex gap-1.5 text-[11.5px] text-faint">
            <span>{formatDate(episode.publishedTs)}</span>
            <span>·</span>
            <span class="text-secondary">
              {episode.durationSecs === null
                ? "时长未知"
                : formatTime(episode.durationSecs)}
            </span>
          </div>
          <p class="mt-1 line-clamp-3 text-[13.5px] text-secondary">
            {episode.description}
          </p>
          {isCurrent && episode.articleHtml !== "" && (
            <div class="mt-1.5">
              <ShowNotes html={episode.articleHtml} />
            </div>
          )}
        </div>

        <span
          class={`shrink-0 rounded-full border px-2 py-1 text-[10.5px] ${
            disabled
              ? "border-white/10 bg-white/5 text-faint"
              : isPlaying
                ? "border-accent-dim bg-accent/15 text-accent"
                : "border-accent-dim bg-accent/8 text-accent"
          }`}
        >
          {disabled ? "无法播放" : isPlaying ? "播放中" : "已暂停"}
        </span>
      </div>
    </button>
  );
}
