import { createModel } from "@rematch/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type { Update } from "@tauri-apps/plugin-updater";
import type { RootModel } from "../index";

export interface UpdateState {
  /** 是否有可用更新。 */
  available: boolean;
  /** 新版本号。 */
  version: string | null;
  /** 更新说明。 */
  notes: string | null;
  /** 状态机：idle / checking / ready / downloading / installing / error / upToDate。 */
  status:
    | "idle"
    | "checking"
    | "ready"
    | "downloading"
    | "installing"
    | "error"
    | "upToDate";
  /** 下载进度 0-1；null 表示总长未知。 */
  downloadProgress: number | null;
  error: string | null;
}

/** 单例待安装更新对象（不可序列化，不进 state）。 */
let pendingUpdate: Update | null = null;

const AUTO_CHECK_DELAY_MS = 4_000;
const RECHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

export const updateModel = createModel<RootModel>()({
  state: {
    available: false,
    version: null,
    notes: null,
    status: "idle",
    downloadProgress: null,
    error: null,
  } as UpdateState,

  reducers: {
    checking(state): UpdateState {
      return { ...state, status: "checking", error: null };
    },
    updateReady(
      state,
      payload: { version: string; notes: string | null },
    ): UpdateState {
      return {
        ...state,
        available: true,
        version: payload.version,
        notes: payload.notes,
        status: "ready",
        error: null,
      };
    },
    upToDate(state): UpdateState {
      return { ...state, available: false, status: "upToDate" };
    },
    checkFailed(state, error: string): UpdateState {
      // 静默自动检查失败不弹错；手动检查时 UI 会展示。
      return { ...state, status: "error", error };
    },
    downloadStarted(state): UpdateState {
      return { ...state, status: "downloading", downloadProgress: 0 };
    },
    downloadAdvanced(state, progress: number): UpdateState {
      return { ...state, downloadProgress: progress };
    },
    installing(state): UpdateState {
      return { ...state, status: "installing", downloadProgress: null };
    },
    dismissed(state): UpdateState {
      return { ...state, status: "idle", error: null };
    },
  },

  effects: (dispatch) => ({
    /** 检查更新（manual=true 时 UI 显示错误）。 */
    async checkForUpdates(manual = false): Promise<void> {
      dispatch.update.checking();
      try {
        const update = await check();
        if (update === null) {
          dispatch.update.upToDate();
          return;
        }
        pendingUpdate = update;
        dispatch.update.updateReady({
          version: update.version,
          notes: update.body ?? null,
        });
      } catch (error) {
        dispatch.update.checkFailed(
          error instanceof Error ? error.message : String(error),
        );
        if (manual) {
          console.warn("检查更新失败", error);
        }
      }
    },

    /** 下载并安装（安装完成后自动重启）。 */
    async downloadAndInstall(): Promise<void> {
      const update = pendingUpdate;
      if (update === null) {
        return;
      }

      dispatch.update.downloadStarted();
      try {
        let downloaded = 0;
        let contentLength = 0;
        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case "Started":
              contentLength = event.data.contentLength ?? 0;
              break;
            case "Progress":
              downloaded += event.data.chunkLength;
              if (contentLength > 0) {
                dispatch.update.downloadAdvanced(
                  Math.min(1, downloaded / contentLength),
                );
              }
              break;
            case "Finished":
              break;
          }
        });

        dispatch.update.installing();
        await relaunch();
      } catch (error) {
        dispatch.update.checkFailed(
          error instanceof Error ? error.message : String(error),
        );
      }
    },

    /** 稍后提醒（关闭提示，周期检查还会再来）。 */
    dismiss(): void {
      dispatch.update.dismissed();
    },

    /** App 启动后自动检查 + 周期复查。 */
    startAutoCheck(): void {
      const delayedCheck = () => void dispatch.update.checkForUpdates(false);
      window.setTimeout(delayedCheck, AUTO_CHECK_DELAY_MS);
      window.setInterval(delayedCheck, RECHECK_INTERVAL_MS);
    },
  }),
});
