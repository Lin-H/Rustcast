import { createModel } from "@rematch/core";
import { loadDefaultFeed } from "../../services/tauri";
import type { FeedDto } from "../../types";
import type { RootModel } from "../index";

export interface FeedState {
  feed: FeedDto | null;
  loading: boolean;
  error: string | null;
  visibleCount: number;
}

export const feedModel = createModel<RootModel>()({
  state: {
    feed: null,
    loading: true,
    error: null,
    visibleCount: 60,
  } satisfies FeedState as FeedState,
  reducers: {
    setLoading(state, loading: boolean): FeedState {
      return { ...state, loading };
    },
    setFeed(_state, feed: FeedDto): FeedState {
      return {
        feed,
        loading: false,
        error: null,
        visibleCount: 60,
      };
    },
    setError(state, error: string): FeedState {
      return { ...state, loading: false, error };
    },
    showMore(state): FeedState {
      const total = state.feed?.episodes.length ?? 0;
      return {
        ...state,
        visibleCount: Math.min(total, state.visibleCount + 150),
      };
    },
  },
  effects: (dispatch) => ({
    async load(): Promise<void> {
      dispatch.feed.setLoading(true);

      try {
        const feed = await loadDefaultFeed();
        dispatch.feed.setFeed(feed);
      } catch (error) {
        dispatch.feed.setError(error instanceof Error ? error.message : String(error));
      }
    },
  }),
});
