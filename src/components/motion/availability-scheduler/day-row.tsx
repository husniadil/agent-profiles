"use client";
// beui.dev/components/blocks/availability-scheduler
//
// LOCALLY MODIFIED — not byte-identical to beui.dev, so re-syncing this block
// means re-applying these changes. A scheduled wake is a single instant, not a
// span of availability, so the row was reduced to one time:
//
//   * the end `TimeSelect`, the `–` separator and the remove-range `X` are gone,
//     along with the add-range `+` (a day wakes once),
//   * the row is one line at every width rather than stacking on narrow ones,
//   * the control sizes follow this window's scale (`SWITCH`, `h-7` fields)
//     rather than the block's page-sized defaults.
//
// `CopyMenu` is kept: copying one day's time onto others is still useful.
//
// The row's text is also translated through the app's 6-locale i18n (`useT`),
// via the local `DAY_LABEL_KEY` map in `./i18n-keys` — the block's own `label`
// prop is still accepted (index.tsx passes it) but no longer rendered.

import { motion } from "motion/react";
import { useRef } from "react";
import { Switch } from "@/components/motion/switch";
import { SWITCH } from "@/lib/controls";
import { SPRING_LAYOUT } from "@/lib/ease";
import { useT } from "@/lib/i18n";
import { CopyMenu } from "./copy-menu";
import { DAY_LABEL_KEY } from "./i18n-keys";
import { TimeSelect } from "./time-select";
import {
  type DayAvailability,
  type DayKey,
  panelKey,
  type TimeOption,
  withCurrentOption,
} from "./types";

export function DayRow({
  day,
  // Accepted for compatibility with index.tsx, which still passes it — no
  // longer used for display, see the LOCALLY MODIFIED note above.
  label: _label,
  state,
  options,
  reduce,
  elevated,
  openPanel,
  onChange,
  onCopy,
  onPanelOpenChange,
}: {
  day: DayKey;
  label: string;
  state: DayAvailability;
  options: TimeOption[];
  reduce: boolean;
  // True while this row holds the dropdown that opened last, which paints it
  // above every other row. A time panel opens downward when there is room and
  // upward when there isn't, so it has to clear the rows on either side of it
  // — no fixed paint order can satisfy both directions. The flag stays on
  // after the panel closes so the collapse animation stays on top too.
  elevated: boolean;
  /** Id of the one time panel the scheduler is holding open, if any. */
  openPanel: string | null;
  onChange: (next: DayAvailability) => void;
  onCopy: (targets: DayKey[]) => void;
  onPanelOpenChange: (panelId: string, open: boolean) => void;
}) {
  const t = useT();
  const dayLabel = t(DAY_LABEL_KEY[day]);

  const idRef = useRef(0);
  const nextId = () => `${day}-n${idRef.current++}`;

  // The one range this row still edits. Anything a stored value carries beyond
  // it is left untouched and simply not shown.
  const range = state.ranges[0];
  const startPanel = range ? panelKey(day, range.id, "start") : "";

  const setEnabled = (enabled: boolean) => {
    if (enabled && state.ranges.length === 0) {
      onChange({
        enabled,
        ranges: [{ id: nextId(), start: "09:00", end: "17:00" }],
      });
    } else {
      onChange({ ...state, enabled });
    }
  };

  // Only the start moves. `end` is carried through untouched so the stored
  // shape stays valid for anything that still reads a range.
  const setStart = (start: string) => {
    onChange({
      ...state,
      ranges: state.ranges.map((r, i) => (i === 0 ? { ...r, start } : r)),
    });
  };

  return (
    <motion.div
      layout={reduce ? false : "position"}
      transition={SPRING_LAYOUT}
      // LOCALLY MODIFIED: `zIndex: 1` upstream. A row with a z-index is its own
      // stacking context, so the open panel inside it (`z-20`) can never rise
      // above a *sibling* row's trigger, which carries `z-10` in the shared
      // parent context — every other day's time field painted over the open
      // dropdown. The elevated row has to outrank that `z-10` as a whole.
      style={{ zIndex: elevated ? 30 : undefined }}
      className="relative flex items-center gap-2 py-1.5"
    >
      <Switch
        checked={state.enabled}
        onCheckedChange={setEnabled}
        ariaLabel={t("schedule.day.toggleAria", { day: dayLabel })}
        className={`shrink-0 ${SWITCH}`}
      />
      <span className="w-20 shrink-0 truncate text-callout font-medium text-ink">
        {dayLabel}
      </span>

      <div className="flex min-w-0 flex-1 justify-end">
        {state.enabled && range ? (
          <div className="w-[112px] shrink-0">
            <TimeSelect
              value={range.start}
              // A time saved on a different grid still has to show itself.
              options={withCurrentOption(options, options, range.start)}
              onChange={setStart}
              open={openPanel === startPanel}
              onOpenChange={(open) => onPanelOpenChange(startPanel, open)}
            />
          </div>
        ) : (
          <span className="text-sub text-ink-3">{t("schedule.day.off")}</span>
        )}
      </div>

      <CopyMenu fromLabel={dayLabel} reduce={reduce} onApply={onCopy} />
    </motion.div>
  );
}
