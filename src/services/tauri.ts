import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  AddFeedResult,
  AppStateDto,
  AudioCacheStatus,
  FeedDetailDto,
  FeedSummaryDto,
  ImportOpmlResult,
  RefreshFeedResult,
  SaveProgressInput,
} from "../types";

export async function loadInitialState(): Promise<AppStateDto> {
  return invoke<AppStateDto>("load_initial_state_command");
}

export async function listFeeds(): Promise<FeedSummaryDto[]> {
  return invoke<FeedSummaryDto[]>("list_feeds_command");
}

export async function loadFeed(feedId: string): Promise<FeedDetailDto> {
  return invoke<FeedDetailDto>("load_feed_command", { feedId });
}

export async function setSelectedFeed(feedId: string): Promise<void> {
  return invoke<void>("set_selected_feed_command", { feedId });
}

export async function reorderFeeds(feedIds: string[]): Promise<void> {
  return invoke<void>("reorder_feeds_command", { feedIds });
}

export async function addFeed(url: string): Promise<AddFeedResult> {
  return invoke<AddFeedResult>("add_feed_command", { url });
}

export async function refreshFeed(feedId: string): Promise<RefreshFeedResult> {
  return invoke<RefreshFeedResult>("refresh_feed_command", { feedId });
}

export async function deleteFeed(feedId: string): Promise<void> {
  return invoke<void>("delete_feed_command", { feedId });
}

export async function saveProgress(input: SaveProgressInput): Promise<void> {
  return invoke<void>("save_progress_command", { input });
}

export async function importOpml(): Promise<ImportOpmlResult> {
  return invoke<ImportOpmlResult>("import_opml_command");
}

export async function exportOpml(): Promise<string | null> {
  return invoke<string | null>("export_opml_command");
}

export async function cacheArtwork(url: string): Promise<string | null> {
  return invoke<string | null>("cache_artwork_command", { url });
}

export async function ensureAudioCache(
  episodeId: string,
  url: string,
): Promise<AudioCacheStatus> {
  return invoke<AudioCacheStatus>("ensure_audio_cache_command", { episodeId, url });
}

export async function audioCacheStatus(
  episodeId: string,
  url: string,
): Promise<AudioCacheStatus> {
  return invoke<AudioCacheStatus>("audio_cache_status_command", { episodeId, url });
}

export async function listCachedEpisodes(): Promise<string[]> {
  return invoke<string[]>("list_cached_episodes_command");
}

export interface AudioCacheProgressEvent {
  episodeId: string;
  written: number;
  total: number | null;
  complete: boolean;
}

export function listenAudioCacheProgress(
  handler: (event: AudioCacheProgressEvent) => void,
): () => void {
  let unlisten: (() => void) | null = null;
  let disposed = false;

  void import("@tauri-apps/api/event")
    .then(({ listen }) =>
      listen<AudioCacheProgressEvent>("audio-cache-progress", (event) => {
        handler(event.payload);
      }),
    )
    .then((fn) => {
      if (disposed) {
        fn();
      } else {
        unlisten = fn;
      }
    })
    .catch((error) => {
      console.warn("音频缓存进度监听失败", error);
    });

  return () => {
    disposed = true;
    unlisten?.();
  };
}

/**
 * 媒体协议 URL：WebView2（Windows/Android）只拦截 http://{scheme}.localhost 形式，
 * 其他平台用原生 {scheme}://。用运行时探测与 convertFileSrc 相同的规则。
 */
export function mediaUrl(episodeId: string): string {
  const encoded = encodeURIComponent(episodeId);
  const isWindowsOrAndroid =
    navigator.userAgent.includes("Windows") || navigator.userAgent.includes("Android");
  return isWindowsOrAndroid
    ? `http://rustcast-media.localhost/${encoded}`
    : `rustcast-media://localhost/${encoded}`;
}

export async function openExternal(rawUrl: string): Promise<boolean> {
  try {
    const url = new URL(rawUrl);
    if (url.protocol !== "http:" && url.protocol !== "https:") {
      return false;
    }

    await openUrl(url.toString());
    return true;
  } catch {
    return false;
  }
}
