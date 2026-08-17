import { motion, useReducedMotion } from "motion/react";

import { Tooltip } from "@/components/motion/tooltip";

import type { SocketBudget } from "@/lib/api";
import { readable } from "@/lib/color";
import { useT } from "@/lib/i18n";
import { shortenPath, usePathNames } from "@/lib/paths";
import { systemNames } from "@/lib/system";
import { cn } from "@/lib/utils";

/// Bytes of headroom below which the budget is worth a permanent block in a
/// 560×480 window.
///
/// The figure cannot move as someone types — a profile directory is named after
/// a generated id, never after the label — so for one machine this is a fixed
/// verdict rather than a gauge: there is room, or there never will be. What the
/// band is for is the spread between apps, whose ids run 4 to 6 bytes and put
/// their totals 2 apart: a data root that clears `code` can still refuse
/// `claude`. Eight bytes covers that spread with margin, and an ordinary home
/// directory sits about sixteen clear of it.
const TIGHT_BYTES = 8;

/// How much of the socket path a profile under this data root would use.
///
/// Drawn only where there is a limit to draw against: Windows puts its named
/// pipes outside the profile, so `limit_bytes` is null there and a meter would
/// invent a limit that means nothing.
///
/// And only where that limit is close. The refusal itself does not live here —
/// `profile_store::add` turns down a profile that leaves no room whether or not
/// anything was ever drawn — so on the machines that will never reach the limit
/// this block was a number nobody could act on, held permanently in a window
/// with no room to spare. It comes back the moment the margin gets thin.
export function BudgetMeter({ budget, appLabel }: { budget: SocketBudget; appLabel: string }) {
  const t = useT();
  const names = usePathNames();
  const still = useReducedMotion();
  const limit = budget.limit_bytes;
  if (limit === null) return null;

  const over = budget.used_bytes > limit;
  if (!over && limit - budget.used_bytes >= TIGHT_BYTES) return null;
  const danger = readable("var(--danger)");

  // `profile_dir` carries a placeholder id of the right width, not a directory
  // that exists. It is drawn because its *length* is the whole subject, and the
  // shape is what makes that legible: the part of the path we chose is set
  // bright, and the part the machine handed us is dimmed to scenery.
  const inside =
    names.dataRoot && budget.profile_dir.startsWith(`${names.dataRoot}/`)
      ? budget.profile_dir.slice(names.dataRoot.length + 1)
      : "";

  return (
    <>
      <div className="relative mt-2.5 overflow-hidden rounded-lg bg-sunken p-2">
        {/* Said twice, as every other path in the window is: on screen in a
            tooltip that opens on hover and on focus, and to assistive technology
            as the element's own text. beUI's tooltip is `aria-hidden` by design,
            so it can never be the accessible copy. */}
        <Tooltip
          content={budget.profile_dir}
          side="bottom"
          wrapperClassName="block min-w-0 max-w-full"
          className="max-w-[min(420px,calc(100vw-16px))] break-all whitespace-normal font-mono text-sub font-normal"
        >
          <p tabIndex={0} className="truncate rounded-sm font-mono text-sub text-ink">
            <span className="sr-only">{budget.profile_dir}</span>
            <span aria-hidden="true">
              {inside ? (
                <>
                  <span className="text-ink-2">{shortenPath(names.dataRoot, names)}/</span>
                  {inside}
                </>
              ) : (
                budget.profile_dir
              )}
            </span>
          </p>
        </Tooltip>

        <div
          className="mt-1.5 h-1 overflow-hidden rounded-full bg-line"
          role="meter"
          aria-label={t("budget.aria")}
          aria-valuenow={budget.used_bytes}
          aria-valuemin={0}
          aria-valuemax={limit}
        >
          <span
            className="block h-full rounded-full transition-[width] duration-200 ease-out"
            style={{
              width: `${Math.min(100, (budget.used_bytes / limit) * 100)}%`,
              // Neutral until it is a problem. The accent is oxblood, and a bar
              // that reads as damage while the budget is perfectly healthy
              // teaches the reader to ignore the one time it turns red.
              background: over ? "var(--danger)" : "var(--ink-3)",
            }}
          />
        </div>

        <div className="mt-1.5 flex items-baseline justify-between gap-3 text-sub">
          <span
            className={cn("truncate", over ? "font-medium" : "text-ink-2")}
            style={over ? { color: danger } : undefined}
          >
            {over
              ? t("budget.over", { bytes: budget.used_bytes - limit })
              : t("budget.under", { system: systemNames(t).system, limit })}
          </span>
          <span
            className="shrink-0 font-mono tabular-nums"
            style={{ color: over ? danger : "var(--ink-2)" }}
          >
            <b className="font-normal text-ink" style={over ? { color: danger } : undefined}>
              {budget.used_bytes}
            </b>
            {t("budget.ofLimit", { limit })}
          </span>
        </div>

        {/* The one ambient-looking effect in the window, and it is not ambient:
            it exists only while no profile can be created at all, and it goes
            the moment that stops being true. Under the limit nothing is drawn
            here at all — a decoration that runs in the healthy state teaches the
            reader to stop seeing it in the state that matters.

            The breath rests at full strength rather than at its dimmest, so a
            window that never gets a frame — hidden behind the tray, or with
            motion turned off — still shows a meter ringed in the danger colour
            rather than a ghost of one. */}
        {over ? (
          <motion.span
            aria-hidden="true"
            className="pointer-events-none absolute inset-0 rounded-lg ring-1 ring-[var(--danger)]"
            animate={still ? undefined : { opacity: [1, 0.35, 1] }}
            transition={{ duration: 1.8, repeat: Infinity, ease: "easeInOut" }}
          />
        ) : null}
      </div>

      {over ? (
        <p className="mt-1.5 text-sub text-ink-2">
          {t("budget.cannotCreate", { app: appLabel })}
        </p>
      ) : null}

      {/* Over budget means no profile can be created on this machine at all,
          which is too important to leave as a colour change nobody is looking
          at. The readout says it in place; this says it out loud, and only in
          the case that warrants interrupting. */}
      <p className="sr-only" role="alert">
        {over ? t("budget.tooDeep", { bytes: budget.used_bytes - limit }) : ""}
      </p>
    </>
  );
}
