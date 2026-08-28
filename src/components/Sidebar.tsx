import { Artwork } from "./Artwork";
import type { FeedDto } from "../types";

interface SidebarProps {
  feed: FeedDto | null;
  loading: boolean;
  error: string | null;
}

export function Sidebar({ feed, loading, error }: SidebarProps) {
  let body: preact.JSX.Element;

  if (error !== null) {
    body = (
      <div>
        <h2 class="text-[15px] font-bold text-primary">订阅源加载失败</h2>
        <p class="mt-1.5 line-clamp-6 text-xs text-secondary">{error}</p>
      </div>
    );
  } else if (feed !== null) {
    body = (
      <div class="flex min-h-0 flex-1 flex-col gap-1">
        <Artwork
          src={feed.logoUrl}
          fallbackSrc={null}
          alt={feed.title}
          className="h-[76px] w-[76px] rounded-[10px]"
          placeholderClassName="text-2xl"
        />
        <h2 class="mt-2.5 text-lg font-bold text-primary">{feed.title}</h2>
        <p class="text-xs font-medium text-accent">{feed.episodes.length} 集</p>
        {feed.description !== null && (
          <p class="mt-1 line-clamp-6 text-[13px] text-faint">{feed.description}</p>
        )}
      </div>
    );
  } else {
    body = (
      <p class={loading ? "text-[13px] text-secondary" : "text-[13px] text-secondary"}>
        {loading ? "正在加载订阅源…" : "暂无订阅源"}
      </p>
    );
  }

  return (
    <aside class="flex h-full w-[272px] shrink-0 py-3 pl-4 pr-3">
      <div class="flex h-full w-full flex-col rounded-xl bg-card p-4">
        {body}
        <div class="mt-auto flex items-center justify-center gap-1.5 rounded-lg px-2 py-[7px] text-xs text-faint">
          <span class="text-[13px] font-bold">+</span>
          <span>添加订阅源（即将推出）</span>
        </div>
      </div>
    </aside>
  );
}
