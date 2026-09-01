import { useMemo } from "preact/hooks";
import { useAppSelector } from "../store";
import { makeTranslator, type Translator } from "../lib/i18n";

/** 组件内取文案：const t = useTranslator(); t("key") */
export function useTranslator(): Translator {
  const language = useAppSelector((state) => state.settings.language);
  return useMemo(() => makeTranslator(language), [language]);
}
