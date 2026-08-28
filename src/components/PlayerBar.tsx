import { Artwork } from "./Artwork";
import { PauseIcon, PlayIcon, VolumeIcon } from "./icons";
import { formatTime } from "../lib/format";
import { dispatch, useAppSelector } from "../store";

export function PlayerBar({ fallbackImage }: { fallbackImage: string | null }) {
  const episode = useAppSelector((state) => state.player.episode);
  const isPlaying = useAppSelector((state) => state.player.isPlaying);
  const buffering = useAppSelector((state) => state.player.buffering);
  const finished = useAppSelector((state) => state.player.finished);
  const currentTime = useAppSelector((state) => state.player.currentTime);
  const duration = useAppSelector((state) => state.player.duration);
  const volume = useAppSelector((state) => state.player.volume);
  const error = useAppSelector((state) => state.player.error);
  const scrubbing = useAppSelector((state) => state.player.scrubbing);
  const scrubValue = useAppSelector((state) => state.player.scrubValue);

  if (episode === null) {
    return (
      <footer class="shrink-0 bg-panel px-[26px] py-4">
        <p class="text-[12.5px] text-faint">在上方选择一集，即可开始流式收听</p>
      </footer>
    );
  }

  const displayDuration = duration > 0 ? duration : (episode.durationSecs ?? 1);
  const progressMax = Math.max(1, displayDuration);
  const progressValue = scrubbing
    ? Math.min(scrubValue, progressMax)
    : Math.min(currentTime, progressMax);
  const statusText = buffering
    ? "缓冲中…"
    : finished
      ? "已播完"
      : isPlaying
        ? "正在播放"
        : "已暂停";

  const commitScrub = () => {
    const target = scrubValue;
    dispatch.player.scrubCommitted();
    dispatch.player.seek(target);
  };

  return (
    <footer class="shrink-0 bg-panel px-[26px] pb-3 pt-3">
      {error !== null && (
        <p class="pb-1.5 text-xs text-danger">{error}</p>
      )}

      <div class="flex items-center gap-5">
        <Artwork
          src={episode.imageUrl}
          fallbackSrc={fallbackImage}
          alt={episode.title}
          className="h-[52px] w-[52px] rounded-[10px]"
        />

        <div class="w-[230px] shrink-0">
          <p class="truncate text-[13.5px] font-bold text-primary">{episode.title}</p>
          <p class="mt-0.5 truncate text-[11px] text-secondary">
            {statusText}
          </p>
        </div>

        <div class="min-w-0 flex-1">
          <input
            type="range"
            min={0}
            max={progressMax}
            step={1}
            value={progressValue}
            aria-label="播放进度"
            onInput={(event) => {
              if (!scrubbing) {
                dispatch.player.scrubStarted();
              }
              dispatch.player.scrubMoved(Number(event.currentTarget.value));
            }}
            onPointerUp={commitScrub}
            onTouchEnd={commitScrub}
            onKeyUp={commitScrub}
          />
          <div class="mt-1 flex text-[11px]">
            <span class="text-accent">{formatTime(progressValue)}</span>
            <span class="ml-auto text-faint">{formatTime(displayDuration)}</span>
          </div>
        </div>

        <button
          type="button"
          class="grid h-11 w-11 shrink-0 cursor-pointer place-items-center rounded-full bg-accent text-root shadow-[0_2px_14px_rgba(255,180,84,0.22)] transition-transform hover:scale-105 active:scale-95"
          onClick={() => dispatch.player.toggle()}
          aria-label={isPlaying ? "暂停" : "播放"}
        >
          {isPlaying ? <PauseIcon /> : <PlayIcon />}
        </button>

        <div class="flex w-[150px] shrink-0 items-center gap-2">
          <VolumeIcon className="h-[17px] w-[17px] text-secondary" />
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={volume}
            aria-label="音量"
            class="w-[112px]"
            onInput={(event) => {
              const nextVolume = Number(event.currentTarget.value);
              dispatch.player.volumeSet(nextVolume);
              dispatch.player.setVolume(nextVolume);
            }}
          />
          <span class="text-[11px] text-secondary">
            {Math.round(volume * 100)}%
          </span>
        </div>
      </div>
    </footer>
  );
}
