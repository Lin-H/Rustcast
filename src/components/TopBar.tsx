import { BrandIcon, RefreshIcon } from "./icons";
import { useTranslator } from "../hooks/useTranslator";
import { dispatch, useAppSelector } from "../store";
import type { Language, ThemeMode } from "../store/models/settings";

const LANGUAGE_OPTIONS: Array<{ value: Language; label: string }> = [
  { value: "zh", label: "中文" },
  { value: "en", label: "EN" },
];

export function TopBar() {
  const t = useTranslator();
  const theme = useAppSelector((state) => state.settings.theme);
  const language = useAppSelector((state) => state.settings.language);
  const updateStatus = useAppSelector((state) => state.update.status);

  const themeModes: Array<{ mode: ThemeMode; label: string }> = [
    { mode: "system", label: t("themeSystem") },
    { mode: "light", label: t("themeLight") },
    { mode: "dark", label: t("themeDark") },
  ];

  return (
    <header class="flex shrink-0 items-center bg-panel px-[22px] py-3">
      <div class="flex items-center gap-2.5">
        <BrandIcon className="h-[22px] w-[22px] text-accent" />
        <span class="text-[17px] font-bold text-primary">Rustcast</span>
      </div>
      <span class="ml-3 text-xs text-faint">{t("appSubtitle")}</span>

      <div class="ml-auto flex items-center gap-2.5">
        <button
          type="button"
          class={`grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-secondary transition-colors hover:bg-card-hover hover:text-accent disabled:cursor-wait disabled:opacity-50 ${
            updateStatus === "checking" ? "animate-pulse" : ""
          }`}
          onClick={() => void dispatch.update.checkForUpdates(true)}
          disabled={updateStatus === "checking" || updateStatus === "downloading" || updateStatus === "installing"}
          title={t("updateCheck")}
          aria-label={t("updateCheck")}
        >
          <RefreshIcon className="h-3.5 w-3.5" />
        </button>

        <div
          class="flex items-center gap-0.5 rounded-lg bg-root p-0.5"
          role="group"
          aria-label={t("settingsTheme")}
        >
          {themeModes.map(({ mode, label }) => (
            <button
              key={mode}
              type="button"
              class={`h-6 cursor-pointer rounded-md px-2 text-[11px] font-medium transition-colors ${
                theme === mode
                  ? "bg-elevated font-bold text-accent"
                  : "text-faint hover:text-secondary"
              }`}
              onClick={() => dispatch.settings.setTheme(mode)}
              aria-pressed={theme === mode}
              title={`${t("settingsTheme")}: ${label}`}
            >
              {label}
            </button>
          ))}
        </div>

        <div
          class="flex items-center gap-0.5 rounded-lg bg-root p-0.5"
          role="group"
          aria-label={t("settingsLanguage")}
        >
          {LANGUAGE_OPTIONS.map(({ value, label }) => (
            <button
              key={value}
              type="button"
              class={`h-6 cursor-pointer rounded-md px-2 text-[11px] font-medium transition-colors ${
                language === value
                  ? "bg-elevated font-bold text-accent"
                  : "text-faint hover:text-secondary"
              }`}
              onClick={() => dispatch.settings.setLanguage(value)}
              aria-pressed={language === value}
              title={`${t("settingsLanguage")}: ${label}`}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
    </header>
  );
}
