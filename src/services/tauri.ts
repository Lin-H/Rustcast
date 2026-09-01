import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  AddFeedResult,
  AppStateDto,
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
