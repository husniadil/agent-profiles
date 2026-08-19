import type { T } from "@/lib/i18n";

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = -1;
  do {
    value /= 1024;
    unit += 1;
  } while (value >= 1024 && unit < units.length - 1);
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unit]}`;
}

/// The one line that replaces the eyebrow, the h1 and the lede.
///
/// `bytes` is null until every row has reported its size, because a total that
/// counts half the profiles is a wrong number stated confidently — worse than
/// no number at all.
export function summary(
  t: T,
  profiles: number,
  running: number,
  bytes: number | null,
): string {
  const parts = [
    t(profiles === 1 ? "status.summaryProfile" : "status.summaryProfiles", {
      count: profiles,
    }),
    t("status.summaryRunning", { count: running }),
  ];
  if (bytes !== null) {
    parts.push(t("status.summaryOnDisk", { size: formatBytes(bytes) }));
  }
  return parts.join(" · ");
}
