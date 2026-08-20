import type { T } from "@/lib/i18n";

/// The mark on a size the walk could not fully reach: the figure is real but a
/// floor, short by an unknown amount, so it is shown as a lower bound rather than
/// presented as the answer.
///
/// `≥` and not `~`. A skipped subtree is bytes the walk could not add, so it only
/// ever makes the true size *larger* — never smaller. `~` reads as "roughly,
/// either way", which is the one thing this is not, and it would contradict the
/// spoken form, which already says "at least". A glyph and not a word, because
/// this sits in a column of numbers.
///
/// Exported so the one surface that must render the mark apart from the figure —
/// the ticker in `Counters`, which is handed a number `≥4.2` is not — marks a
/// number the same way. One answer to what an approximate size is called.
export const APPROX_MARK = "≥";

/// `approximate` is the walk having skipped something: the figure is real but a
/// lower bound, so it is marked with `APPROX_MARK` rather than presented as the
/// answer.
export function formatBytes(bytes: number, approximate = false): string {
  return (approximate ? APPROX_MARK : "") + exactBytes(bytes);
}

function exactBytes(bytes: number): string {
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
  approximate = false,
): string {
  const parts = [
    t(profiles === 1 ? "status.summaryProfile" : "status.summaryProfiles", {
      count: profiles,
    }),
    t("status.summaryRunning", { count: running }),
  ];
  if (bytes !== null) {
    parts.push(
      t(approximate ? "status.summaryOnDiskApprox" : "status.summaryOnDisk", {
        size: exactBytes(bytes),
      }),
    );
  }
  return parts.join(" · ");
}
