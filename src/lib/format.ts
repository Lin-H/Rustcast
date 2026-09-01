export function formatTime(totalSeconds: number): string {
  const seconds = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainder = seconds % 60;

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}`;
  }

  return `${minutes}:${String(remainder).padStart(2, "0")}`;
}

export function formatDate(
  timestampSeconds: number,
  language: "zh" | "en" = "zh",
): string {
  const locale = language === "zh" ? "zh-CN" : "en-US";
  const invalid = !Number.isFinite(timestampSeconds) || timestampSeconds <= 0;
  const date = new Date(invalid ? 0 : timestampSeconds * 1000);
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(date);
}
