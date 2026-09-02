import { useTranslator } from "../hooks/useTranslator";
import { dispatch, useAppSelector } from "../store";

/** 顶栏下方的更新横幅：发现新版本 → 下载进度 → 安装重启。 */
export function UpdateBanner() {
  const t = useTranslator();
  const status = useAppSelector((state) => state.update.status);
  const version = useAppSelector((state) => state.update.version);
  const notes = useAppSelector((state) => state.update.notes);
  const progress = useAppSelector((state) => state.update.downloadProgress);
  const error = useAppSelector((state) => state.update.error);

  if (status === "ready") {
    return (
      <div class="flex shrink-0 items-center gap-3 border-b border-accent-dim bg-accent/10 px-[22px] py-2">
        <span class="min-w-0 flex-1 truncate text-[12.5px] text-primary">
          {t("updateAvailable")}: <span class="font-bold text-accent">{version}</span>
          {notes !== null && notes !== "" && (
            <span class="ml-1.5 text-secondary">— {notes.slice(0, 120)}</span>
          )}
        </span>
        <button
          type="button"
          class="h-7 shrink-0 cursor-pointer rounded-lg bg-accent px-3 text-[12px] font-bold text-root transition-opacity hover:opacity-90"
          onClick={() => void dispatch.update.downloadAndInstall()}
        >
          {t("updateNow")}
        </button>
        <button
          type="button"
          class="h-7 shrink-0 cursor-pointer rounded-lg px-2.5 text-[12px] text-secondary transition-colors hover:bg-card-hover"
          onClick={() => dispatch.update.dismiss()}
        >
          {t("updateLater")}
        </button>
      </div>
    );
  }

  if (status === "downloading") {
    const pct = progress !== null ? Math.round(progress * 100) : null;
    return (
      <div class="flex shrink-0 items-center gap-3 border-b border-accent-dim bg-accent/10 px-[22px] py-2">
        <span class="text-[12.5px] text-secondary">{t("updateDownloading")}</span>
        <div class="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full bg-root">
          <div
            class="h-full rounded-full bg-accent transition-[width]"
            style={{ width: `${pct ?? 0}%` }}
          />
        </div>
        <span class="w-10 shrink-0 text-right text-[11.5px] text-faint">
          {pct !== null ? `${pct}%` : "…"}
        </span>
      </div>
    );
  }

  if (status === "installing") {
    return (
      <div class="flex shrink-0 items-center gap-2 border-b border-accent-dim bg-accent/10 px-[22px] py-2">
        <span class="h-3.5 w-3.5 animate-spin rounded-full border-2 border-accent border-t-transparent" />
        <span class="text-[12.5px] text-secondary">{t("updateInstalling")}</span>
      </div>
    );
  }

  // 手动检查失败的横幅只在 error 态展示（自动检查失败静默）。
  if (status === "error" && error !== null) {
    return (
      <div class="flex shrink-0 items-center gap-3 border-b border-danger/30 bg-danger/10 px-[22px] py-2">
        <span class="min-w-0 flex-1 truncate text-[12.5px] text-danger">
          {t("updateCheckFailed")}: {error}
        </span>
        <button
          type="button"
          class="h-7 shrink-0 cursor-pointer rounded-lg px-2.5 text-[12px] text-secondary transition-colors hover:bg-card-hover"
          onClick={() => dispatch.update.dismiss()}
        >
          ✕
        </button>
      </div>
    );
  }

  return null;
}
