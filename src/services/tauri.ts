import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { FeedDto } from "../types";

export async function loadDefaultFeed(): Promise<FeedDto> {
  return invoke<FeedDto>("load_default_feed");
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
