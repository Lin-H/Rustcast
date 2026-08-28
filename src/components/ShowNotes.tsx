import type { MouseEvent } from "preact/compat";
import { useMemo } from "preact/hooks";
import { openExternal } from "../services/tauri";
import { sanitizeShowNotes } from "../lib/sanitize";

interface ShowNotesProps {
  html: string;
}

export function ShowNotes({ html }: ShowNotesProps) {
  const safeHtml = useMemo(() => sanitizeShowNotes(html), [html]);

  const handleClick = async (event: MouseEvent<HTMLDivElement>) => {
    const target = event.target;
    const anchor = target instanceof Element ? target.closest("a") : null;
    const href = anchor?.getAttribute("href");

    if (!anchor || !href) {
      return;
    }

    event.preventDefault();
    await openExternal(href);
  };

  return (
    <div
      class="show-notes max-h-[210px] overflow-y-auto rounded-xl border border-white/5 bg-root p-3"
      onClick={handleClick}
      dangerouslySetInnerHTML={{ __html: safeHtml }}
    />
  );
}
