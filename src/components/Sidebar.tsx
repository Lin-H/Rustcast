import type { FormEvent } from "preact/compat";
import { useState } from "preact/hooks";
import { Artwork } from "./Artwork";
import { ExportIcon, ImportIcon, PlusIcon, RefreshIcon, TrashIcon } from "./icons";
import { dispatch, useAppSelector } from "../store";

export function Sidebar() {
  const feeds = useAppSelector((state) => state.feed.feeds);
  const selectedFeedId = useAppSelector((state) => state.feed.selectedFeedId);
  const loading = useAppSelector((state) => state.feed.loading);
  const error = useAppSelector((state) => state.feed.error);
  const adding = useAppSelector((state) => state.feed.adding);
  const addError = useAppSelector((state) => state.feed.addError);
  const refreshingFeedId = useAppSelector((state) => state.feed.refreshingFeedId);
  const opmlBusy = useAppSelector((state) => state.feed.opmlBusy);
  const [url, setUrl] = useState("");
  const [opmlNotice, setOpmlNotice] = useState<string | null>(null);

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = url.trim();
    if (trimmed === "" || adding) {
      return;
    }
    void dispatch.feed.addSubscription(trimmed);
    setUrl("");
  };

  const handleDelete = (feedId: string) => {
    if (!window.confirm("确定删除该订阅源吗？相关单集和播放进度将一并删除。")) {
      return;
    }
    void dispatch.feed.removeSubscription(feedId);
  };

  const handleImport = async () => {
    if (opmlBusy) {
      return;
    }
    setOpmlNotice(null);
    const result = await dispatch.feed.importOpml();
    if (result === null) {
      return;
    }
    const parts = [`导入 ${result.imported} 个订阅源`];
    if (result.skipped > 0) {
      parts.push(`${result.skipped} 个已存在`);
    }
    if (result.failed.length > 0) {
      parts.push(`${result.failed.length} 个失败`);
    }
    setOpmlNotice(
      parts.join("，") +
        (result.failed.length > 0
          ? `（首个失败：${result.failed[0]?.url ?? ""}）`
          : ""),
    );
  };

  const handleExport = async () => {
    if (opmlBusy) {
      return;
    }
    setOpmlNotice(null);
    const path = await dispatch.feed.exportOpml();
    if (path !== null) {
      setOpmlNotice(`已导出到 ${path}`);
    }
  };

  return (
    <aside class="flex h-full w-[290px] shrink-0 py-3 pl-4 pr-3">
      <div class="flex h-full w-full flex-col rounded-xl bg-card p-4">
        <div class="flex items-center justify-between">
          <h2 class="text-[15px] font-bold text-primary">订阅源</h2>
          <span class="text-xs text-faint">{feeds.length}</span>
        </div>

        <form class="mt-3 flex gap-2" onSubmit={handleSubmit}>
          <input
            type="text"
            inputMode="url"
            autoComplete="off"
            value={url}
            placeholder="RSS / Atom 地址"
            class="min-w-0 flex-1 rounded-lg border border-white/10 bg-root px-2.5 py-[7px] text-[12.5px] text-primary outline-none placeholder:text-faint focus:border-accent-dim"
            onInput={(event) => setUrl(event.currentTarget.value)}
            disabled={adding}
          />
          <button
            type="submit"
            disabled={adding}
            class="grid h-[32px] w-[32px] shrink-0 cursor-pointer place-items-center rounded-lg bg-accent text-root transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
            title="添加订阅源"
          >
            {adding ? (
              <span class="h-3.5 w-3.5 animate-spin rounded-full border-2 border-root border-t-transparent" />
            ) : (
              <PlusIcon className="h-4 w-4" />
            )}
          </button>
        </form>
        {addError !== null && <p class="mt-1.5 text-[11px] text-danger">{addError}</p>}
        {error !== null && <p class="mt-2 text-[11.5px] text-danger">{error}</p>}

        <div class="mt-3 flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
          {feeds.length === 0 && !loading && (
            <p class="py-6 text-center text-xs text-faint">暂无订阅源，粘贴地址添加</p>
          )}
          {feeds.map((feed) => {
            const selected = feed.id === selectedFeedId;
            const refreshing = feed.id === refreshingFeedId;
            return (
              <div
                key={feed.id}
                class={`group flex items-center gap-1.5 rounded-lg px-2 py-2 ${
                  selected ? "bg-elevated" : "hover:bg-card-hover"
                }`}
              >
                <button
                  type="button"
                  class="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5 text-left"
                  onClick={() => void dispatch.feed.selectFeed(feed.id)}
                  title={feed.url}
                >
                  <Artwork
                    src={feed.logoUrl}
                    fallbackSrc={null}
                    alt={feed.title}
                    className="h-9 w-9 rounded-lg"
                    placeholderClassName="text-sm"
                  />
                  <span class="min-w-0">
                    <span
                      class={`block truncate text-[13px] ${
                        selected ? "font-bold text-accent" : "font-medium text-primary"
                      }`}
                    >
                      {feed.title}
                    </span>
                    <span class="block text-[11px] text-faint">{feed.episodeCount} 集</span>
                  </span>
                </button>
                <button
                  type="button"
                  disabled={refreshing}
                  onClick={() => void dispatch.feed.refreshSubscription(feed.id)}
                  class="grid h-7 w-7 shrink-0 cursor-pointer place-items-center rounded-md text-faint transition-colors hover:bg-card-hover hover:text-accent disabled:cursor-wait"
                  title="刷新订阅"
                  aria-label={`刷新 ${feed.title}`}
                >
                  {refreshing ? (
                    <span class="h-3.5 w-3.5 animate-spin rounded-full border-2 border-accent border-t-transparent" />
                  ) : (
                    <RefreshIcon className="h-3.5 w-3.5" />
                  )}
                </button>
                <button
                  type="button"
                  onClick={() => handleDelete(feed.id)}
                  class="grid h-7 w-7 shrink-0 cursor-pointer place-items-center rounded-md text-faint transition-colors hover:bg-card-hover hover:text-danger"
                  title="删除订阅"
                  aria-label={`删除 ${feed.title}`}
                >
                  <TrashIcon className="h-3.5 w-3.5" />
                </button>
              </div>
            );
          })}
        </div>

        <div class="mt-3 flex items-center gap-2">
          <button
            type="button"
            disabled={opmlBusy}
            onClick={() => void handleImport()}
            class="flex h-7 flex-1 cursor-pointer items-center justify-center gap-1.5 rounded-lg bg-root text-[11.5px] font-medium text-secondary transition-colors hover:bg-card-hover hover:text-accent disabled:cursor-wait disabled:opacity-60"
            title="从 OPML 文件导入订阅源"
          >
            <ImportIcon className="h-3.5 w-3.5" />
            {opmlBusy ? "处理中…" : "导入 OPML"}
          </button>
          <button
            type="button"
            disabled={opmlBusy}
            onClick={() => void handleExport()}
            class="flex h-7 flex-1 cursor-pointer items-center justify-center gap-1.5 rounded-lg bg-root text-[11.5px] font-medium text-secondary transition-colors hover:bg-card-hover hover:text-accent disabled:cursor-wait disabled:opacity-60"
            title="导出订阅源为 OPML 文件"
          >
            <ExportIcon className="h-3.5 w-3.5" />
            导出 OPML
          </button>
        </div>
        {opmlNotice !== null && (
          <p class="mt-1.5 break-all text-[10.5px] text-faint">{opmlNotice}</p>
        )}

        <div class="mt-2 text-[10.5px] text-faint">订阅和播放进度保存在本地数据库</div>
      </div>
    </aside>
  );
}
