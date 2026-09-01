import { createModel } from "@rematch/core";
import { store, type RootModel } from "../index";

export type ThemeMode = "system" | "light" | "dark";
export type Language = "zh" | "en";

export interface SettingsState {
  theme: ThemeMode;
  /** 系统当前实际生效的主题（system 模式下跟随 prefers-color-scheme）。 */
  effectiveTheme: "light" | "dark";
  language: Language;
}

const THEME_KEY = "rustcast.theme";
const LANGUAGE_KEY = "rustcast.language";

function readTheme(): ThemeMode {
  try {
    const raw = window.localStorage.getItem(THEME_KEY);
    if (raw === "light" || raw === "dark" || raw === "system") {
      return raw;
    }
  } catch {
    // ignore
  }
  return "system";
}

function readLanguage(): Language {
  try {
    const raw = window.localStorage.getItem(LANGUAGE_KEY);
    if (raw === "zh" || raw === "en") {
      return raw;
    }
  } catch {
    // ignore
  }
  return "zh";
}

function systemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

/** 把 theme 模式解析为实际生效主题并同步到 <html> class。 */
function applyTheme(mode: ThemeMode): "light" | "dark" {
  const effective = mode === "system" ? systemTheme() : mode;
  const root = document.documentElement;
  root.classList.toggle("theme-light", effective === "light");
  root.classList.toggle("theme-dark", effective === "dark");
  return effective;
}

function persist(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // ignore
  }
}

export const settingsModel = createModel<RootModel>()({
  state: {
    theme: readTheme(),
    effectiveTheme: applyTheme(readTheme()),
    language: readLanguage(),
  } as SettingsState,

  reducers: {
    themeSet(state, theme: ThemeMode): SettingsState {
      return { ...state, theme, effectiveTheme: applyTheme(theme) };
    },
    effectiveThemeSynced(state, effective: "light" | "dark"): SettingsState {
      return state.effectiveTheme === effective ? state : { ...state, effectiveTheme: effective };
    },
    languageSet(state, language: Language): SettingsState {
      return { ...state, language };
    },
  },

  effects: (dispatch) => ({
    setTheme(theme: ThemeMode): void {
      persist(THEME_KEY, theme);
      dispatch.settings.themeSet(theme);
    },
    setLanguage(language: Language): void {
      persist(LANGUAGE_KEY, language);
      dispatch.settings.languageSet(language);
    },
    /** system 模式下系统主题变化时由 App 层调用。 */
    syncSystemTheme(): void {
      const { settings } = store.getState();
      if (settings.theme === "system") {
        dispatch.settings.effectiveThemeSynced(applyTheme("system"));
      }
    },
  }),
});
