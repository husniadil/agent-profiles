/// Maps the block's day keys onto this app's translation keys. A local addition
/// to the vendored block (see the LOCALLY MODIFIED note in `day-row.tsx`), kept
/// out of `types.ts` so that file stays byte-identical to beui.dev.
import type { DayKey } from "./types";

export const DAY_LABEL_KEY = {
  mon: "schedule.day.mon",
  tue: "schedule.day.tue",
  wed: "schedule.day.wed",
  thu: "schedule.day.thu",
  fri: "schedule.day.fri",
  sat: "schedule.day.sat",
  sun: "schedule.day.sun",
} as const satisfies Record<DayKey, string>;
