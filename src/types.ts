export interface FeedSummaryDto {
  id: string;
  url: string;
  title: string;
  description: string | null;
  logoUrl: string | null;
  episodeCount: number;
  lastRefreshedAt: number | null;
  lastError: string | null;
}

export interface ProgressDto {
  positionSecs: number;
  durationSecs: number | null;
  completed: boolean;
  updatedAt: number;
}

export interface EpisodeDto {
  id: string;
  feedId: string;
  entryId: string;
  title: string;
  description: string;
  articleHtml: string;
  publishedTs: number;
  durationSecs: number | null;
  audioUrl: string | null;
  imageUrl: string | null;
  progress: ProgressDto | null;
}

export interface FeedDetailDto {
  feed: FeedSummaryDto;
  episodes: EpisodeDto[];
}

export interface AppStateDto {
  feeds: FeedSummaryDto[];
  selectedFeedId: string | null;
  selectedFeed: FeedDetailDto | null;
}

export interface AddFeedResult {
  feed: FeedDetailDto;
  alreadyExists: boolean;
}

export interface RefreshFeedResult {
  feed: FeedDetailDto;
  error: string | null;
}

export interface SaveProgressInput {
  episodeId: string;
  positionSecs: number;
  durationSecs: number | null;
  completed: boolean;
}

export interface ImportOpmlResult {
  imported: number;
  skipped: number;
  failed: Array<{ url: string; error: string }>;
}

export interface AudioCacheStatus {
  written: number;
  total: number | null;
  complete: boolean;
}
