import { createModel } from "@rematch/core";
import {
  addFeed as addFeedRequest,
  deleteFeed as deleteFeedRequest,
  exportOpml as exportOpmlRequest,
  importOpml as importOpmlRequest,
  loadFeed as loadFeedRequest,
  loadInitialState,
  refreshFeed as refreshFeedRequest,
  setSelectedFeed,
} from "../../services/tauri";
import type {
  AddFeedResult,
  AppStateDto,
  FeedDetailDto,
  FeedSummaryDto,
  ImportOpmlResult,
} from "../../types";
import { store } from "../index";
import type { RootModel } from "../index";

export interface FeedState {
  feeds: FeedSummaryDto[];
  selectedFeedId: string | null;
  selectedFeed: FeedDetailDto | null;
  loading: boolean;
  error: string | null;
  visibleCount: number;
  adding: boolean;
  addError: string | null;
  refreshingFeedId: string | null;
  refreshError: string | null;
  opmlBusy: boolean;
  opmlError: string | null;
}

const initialState: FeedState = {
  feeds: [],
  selectedFeedId: null,
  selectedFeed: null,
  loading: true,
  error: null,
  visibleCount: 60,
  adding: false,
  addError: null,
  refreshingFeedId: null,
  refreshError: null,
  opmlBusy: false,
  opmlError: null,
};

export const feedModel = createModel<RootModel>()({
  state: initialState,
  reducers: {
    setLoading(state, loading: boolean): FeedState {
      return { ...state, loading };
    },
    setInitial(state, appState: AppStateDto): FeedState {
      return {
        ...state,
        feeds: appState.feeds,
        selectedFeedId: appState.selectedFeedId,
        selectedFeed: appState.selectedFeed,
        loading: false,
        error: null,
        refreshError: null,
        visibleCount: 60,
      };
    },
    setError(state, error: string): FeedState {
      return { ...state, loading: false, error };
    },
    selectingFeed(state, feedId: string): FeedState {
      return {
        ...state,
        selectedFeedId: feedId,
        selectedFeed: null,
        loading: true,
        error: null,
        refreshError: null,
        visibleCount: 60,
      };
    },
    feedLoaded(state, feed: FeedDetailDto): FeedState {
      return { ...state, selectedFeed: feed, loading: false, error: null, visibleCount: 60 };
    },
    addStarted(state): FeedState {
      return { ...state, adding: true, addError: null };
    },
    addFinished(state, result: AddFeedResult): FeedState {
      const summary = result.feed.feed;
      const exists = state.feeds.some((feed) => feed.id === summary.id);
      const feeds = exists
        ? state.feeds.map((feed) => (feed.id === summary.id ? summary : feed))
        : [...state.feeds, summary];
      return {
        ...state,
        feeds,
        adding: false,
        addError: null,
        selectedFeedId: summary.id,
        selectedFeed: result.feed,
        loading: false,
        error: null,
        visibleCount: 60,
      };
    },
    addFailed(state, error: string): FeedState {
      return { ...state, adding: false, addError: error };
    },
    refreshStarted(state, feedId: string): FeedState {
      return { ...state, refreshingFeedId: feedId, refreshError: null };
    },
    opmlStarted(state): FeedState {
      return { ...state, opmlBusy: true, opmlError: null };
    },
    opmlFinished(state): FeedState {
      return { ...state, opmlBusy: false };
    },
    opmlFailed(state, error: string): FeedState {
      return { ...state, opmlBusy: false, opmlError: error };
    },
    refreshFinished(
      state,
      payload: { feed: FeedDetailDto | null; error: string | null },
    ): FeedState {
      if (payload.feed === null) {
        return { ...state, refreshingFeedId: null, refreshError: payload.error };
      }

      const summary = payload.feed.feed;
      const feeds = state.feeds.map((feed) =>
        feed.id === summary.id ? { ...summary, episodeCount: payload.feed!.episodes.length } : feed,
      );
      const selectedFeed =
        state.selectedFeedId === summary.id ? payload.feed : state.selectedFeed;
      return {
        ...state,
        feeds,
        selectedFeed,
        refreshingFeedId: null,
        refreshError: payload.error,
        loading: false,
      };
    },
    feedRemoved(state, feedId: string): FeedState {
      const feeds = state.feeds.filter((feed) => feed.id !== feedId);
      const wasSelected = state.selectedFeedId === feedId;
      return {
        ...state,
        feeds,
        selectedFeedId: wasSelected ? null : state.selectedFeedId,
        selectedFeed: wasSelected ? null : state.selectedFeed,
      };
    },
    showMore(state): FeedState {
      const total = state.selectedFeed?.episodes.length ?? 0;
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
        const appState = await loadInitialState();
        dispatch.feed.setInitial(appState);
      } catch (error) {
        dispatch.feed.setError(error instanceof Error ? error.message : String(error));
      }
    },
    async selectFeed(feedId: string): Promise<void> {
      dispatch.feed.selectingFeed(feedId);

      try {
        await setSelectedFeed(feedId);
        const feed = await loadFeedRequest(feedId);
        dispatch.feed.feedLoaded(feed);
      } catch (error) {
        dispatch.feed.setError(error instanceof Error ? error.message : String(error));
      }
    },
    async addSubscription(url: string): Promise<void> {
      dispatch.feed.addStarted();

      try {
        const result = await addFeedRequest(url);
        dispatch.feed.addFinished(result);
      } catch (error) {
        dispatch.feed.addFailed(error instanceof Error ? error.message : String(error));
      }
    },
    async refreshSubscription(feedId: string): Promise<void> {
      dispatch.feed.refreshStarted(feedId);

      try {
        const result = await refreshFeedRequest(feedId);
        dispatch.feed.refreshFinished({ feed: result.feed, error: result.error });
      } catch (error) {
        dispatch.feed.refreshFinished({
          feed: null,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    },
    async removeSubscription(feedId: string): Promise<void> {
      try {
        await deleteFeedRequest(feedId);
        const wasSelected = store.getState().feed.selectedFeedId === feedId;
        dispatch.feed.feedRemoved(feedId);

        if (wasSelected) {
          const next = store.getState().feed.feeds[0];
          if (next !== undefined) {
            await dispatch.feed.selectFeed(next.id);
          }
        }
      } catch (error) {
        dispatch.feed.setError(error instanceof Error ? error.message : String(error));
      }
    },
    async importOpml(): Promise<ImportOpmlResult | null> {
      dispatch.feed.opmlStarted();
      try {
        const result = await importOpmlRequest();
        if (result.imported > 0) {
          // 重新拉取订阅列表并选中原选中项（或第一个）。
          const appState = await loadInitialState();
          dispatch.feed.setInitial(appState);
        }
        dispatch.feed.opmlFinished();
        return result;
      } catch (error) {
        dispatch.feed.opmlFailed(error instanceof Error ? error.message : String(error));
        return null;
      }
    },
    async exportOpml(): Promise<string | null> {
      dispatch.feed.opmlStarted();
      try {
        const path = await exportOpmlRequest();
        dispatch.feed.opmlFinished();
        return path;
      } catch (error) {
        dispatch.feed.opmlFailed(error instanceof Error ? error.message : String(error));
        return null;
      }
    },
  }),
});
