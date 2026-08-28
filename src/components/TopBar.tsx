import { BrandIcon } from "./icons";

export function TopBar() {
  return (
    <header class="flex shrink-0 items-center bg-panel px-[22px] py-3.5">
      <div class="flex items-center gap-2.5">
        <BrandIcon className="h-[22px] w-[22px] text-accent" />
        <span class="text-[17px] font-bold text-primary">Rustcast</span>
      </div>
      <span class="ml-auto text-xs text-faint">RSS 音频播放器 · M1</span>
    </header>
  );
}
