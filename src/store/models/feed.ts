import { createModel } from "@rematch/core";
import {
  addFeed as addFeedRequest,
  deleteFeed as deleteFeedRequest,
  exportOpml as exportOpmlRequest,
  importOpml as importOpmlRequest,
  listCachedEpisodes,
  loadFeed as loadFeedRequest,
  loadInitialState,
  refreshFeed as refreshFeedRequest,
  reorderFeeds as reorderFeedsRequest,
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
  page: number;
  adding: boolean;
  addError: string | null;
  refreshingFeedIds: string[];
  refreshError: string | null;
  /** 一键刷新全部进行中。 */
  refreshingAll: boolean;
  opmlBusy: boolean;
  opmlError: string | null;
  /** 已完整缓存到本地的单集 id 集合（离线可用徽标）。 */
  cachedEpisodeIds: string[];
}

export const PAGE_SIZE = 60;

function totalPagesOf(episodeCount: number): number {
  return Math.max(1, Math.ceil(episodeCount / PAGE_SIZE));
}

function clampPage(page: number, episodeCount: number): number {
  return Math.min(Math.max(1, page), totalPagesOf(episodeCount));
}

const initialState: FeedState = {
  feeds: [],
  selectedFeedId: null,
  selectedFeed: null,
  loading: true,
  error: null,
  page: 1,
  adding: false,
  addError: null,
  refreshingFeedIds: [],
  refreshError: null,
  refreshingAll: false,
  opmlBusy: false,
  opmlError: null,
  cachedEpisodeIds: [],
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
        page: 1,
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
        page: 1,
      };
    },
    feedLoaded(state, feed: FeedDetailDto): FeedState {
      return { ...state, selectedFeed: feed, loading: false, error: null, page: 1 };
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
        page: 1,
      };
    },
    addFailed(state, error: string): FeedState {
      return { ...state, adding: false, addError: error };
    },
    refreshStarted(state, feedId: string): FeedState {
      if (state.refreshingFeedIds.includes(feedId)) {
        return state;
      }
      return {
        ...state,
        refreshingFeedIds: [...state.refreshingFeedIds, feedId],
        refreshError: null,
      };
    },
    refreshAllStarted(state): FeedState {
      return { ...state, refreshingAll: true, refreshError: null };
    },
    refreshAllFinished(state): FeedState {
      return { ...state, refreshingAll: false };
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
    cachedEpisodesSet(state, ids: string[]): FeedState {
      return { ...state, cachedEpisodeIds: ids };
    },
    cachedEpisodeAdded(state, episodeId: string): FeedState {
      if (state.cachedEpisodeIds.includes(episodeId)) {
        return state;
      }
      return { ...state, cachedEpisodeIds: [...state.cachedEpisodeIds, episodeId] };
    },
    refreshFinished(
      state,
      payload: { feed: FeedDetailDto | null; error: string | null },
    ): FeedState {
      const summary = payload.feed?.feed;
      const refreshingFeedIds = summary
        ? state.refreshingFeedIds.filter((id) => id !== summary.id)
        : state.refreshingFeedIds;

      if (payload.feed === null) {
        return {
          ...state,
          refreshingFeedIds,
          refreshError: payload.error,
        };
      }

      const feeds = state.feeds.map((feed) =>
        feed.id === summary!.id
          ? { ...summary!, episodeCount: payload.feed!.episodes.length }
          : feed,
      );
      const selectedFeed =
        state.selectedFeedId === summary!.id ? payload.feed : state.selectedFeed;
      // 刷新后单集数可能减少，夹住当前页码。
      const page = selectedFeed === null
        ? state.page
        : clampPage(state.page, selectedFeed.episodes.length);
      return {
        ...state,
        feeds,
        selectedFeed,
        page,
        refreshingFeedIds,
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
        page: wasSelected ? 1 : state.page,
      };
    },
    /** 乐观重排：拖拽时立即按新顺序移动列表项，落库失败由 effect 回滚。 */
    feedsReordered(state, feedIds: string[]): FeedState {
      const byId = new Map(state.feeds.map((feed) => [feed.id, feed]));
      const feeds = feedIds
        .map((id) => byId.get(id))
        .filter((feed): feed is FeedSummaryDto => feed !== undefined);
      if (feeds.length !== state.feeds.length) {
        return state;
      }
      return { ...state, feeds };
    },
    setPage(state, page: number): FeedState {
      const total = state.selectedFeed?.episodes.length ?? 0;
      return { ...state, page: clampPage(page, total) };
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

      // 已缓存单集徽标集合（失败静默，徽标只是增强信息）。
      try {
        const cached = await listCachedEpisodes();
        dispatch.feed.cachedEpisodesSet(cached);
      } catch {
        // ignore
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
    /** 一键刷新全部订阅源：并行请求，逐个落状态；单个失败不中断其他。 */
    async refreshAllSubscriptions(): Promise<void> {
      const ids = store.getState().feed.feeds.map((feed) => feed.id);
      if (ids.length === 0 || store.getState().feed.refreshingAll) {
        return;
      }

      dispatch.feed.refreshAllStarted();
      for (const id of ids) {
        dispatch.feed.refreshStarted(id);
      }

      try {
        await Promise.allSettled(
          ids.map((id) => dispatch.feed.refreshSubscription(id)),
        );
      } finally {
        dispatch.feed.refreshAllFinished();
      }
    },
    /** 拖拽排序：先乐观更新列表，再持久化；失败时重拉恢复真实顺序。 */
    async reorderSubscriptions(feedIds: string[]): Promise<void> {
      const previous = store.getState().feed.feeds.map((feed) => feed.id);
      dispatch.feed.feedsReordered(feedIds);

      try {
        await reorderFeedsRequest(feedIds);
      } catch (error) {
        // 回滚到拖拽前的顺序，避免 UI 与数据库不一致。
        dispatch.feed.feedsReordered(previous);
        dispatch.feed.setError(error instanceof Error ? error.message : String(error));
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
