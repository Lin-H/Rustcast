export interface EpisodeDto {
  id: string;
  title: string;
  description: string;
  articleHtml: string;
  publishedTs: number;
  durationSecs: number | null;
  audioUrl: string | null;
  imageUrl: string | null;
}

export interface FeedDto {
  title: string;
  description: string | null;
  logoUrl: string | null;
  episodes: EpisodeDto[];
}
