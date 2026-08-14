import { motion, useReducedMotion } from "motion/react";

import type { SocketBudget } from "@/lib/api";
import { readable } from "@/lib/color";
import { shortenPath, usePathNames } from "@/lib/paths";
import { systemNames } from "@/lib/system";
import { cn } from "@/lib/utils";

/// How much of the socket path a profile under this data root would use.
///
/// Drawn only where there is a limit to draw against: Windows puts its named
/// pipes outside the profile, so `limit_bytes` is null there and a meter would
/// invent a limit that means nothing.
export function BudgetMeter({ budget, appLabel }: { budget: SocketBudget; appLabel: string }) {
  const names = usePathNames();
  const still = useReducedMotion();
  const limit = budget.limit_bytes;
  if (limit === null) return null;

  const over = budget.used_bytes > limit;
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
        <p className="truncate font-mono text-[10.5px] text-ink" title={budget.profile_dir}>
          {inside ? (
            <>
              <span className="text-ink-2">{shortenPath(names.dataRoot, names)}/</span>
              {inside}
            </>
          ) : (
            budget.profile_dir
          )}
        </p>

        <div
          className="mt-1.5 h-1 overflow-hidden rounded-full bg-line"
          role="meter"
          aria-label="Socket path budget"
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

        <div className="mt-1.5 flex items-baseline justify-between gap-3 text-[10.5px]">
          <span
            className={cn("truncate", over ? "font-medium" : "text-ink-2")}
            style={over ? { color: danger } : undefined}
          >
            {over
              ? `${budget.used_bytes - limit} bytes over the limit`
              : `socket path budget · ${systemNames().system} stops at ${limit}`}
          </span>
          <span
            className="shrink-0 font-mono tabular-nums"
            style={{ color: over ? danger : "var(--ink-2)" }}
          >
            <b className="font-normal text-ink" style={over ? { color: danger } : undefined}>
              {budget.used_bytes}
            </b>
            {` / ${limit} bytes`}
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
        <p className="mt-1.5 text-[11px] text-ink-2">
          {appLabel} would not be able to create its socket here. Move the data root somewhere
          shorter to make room.
        </p>
      ) : null}

      {/* Over budget means no profile can be created on this machine at all,
          which is too important to leave as a colour change nobody is looking
          at. The readout says it in place; this says it out loud, and only in
          the case that warrants interrupting. */}
      <p className="sr-only" role="alert">
        {over
          ? `This folder is too deep for ${budget.used_bytes - limit} bytes of the socket path a profile needs. No profile can be added here.`
          : ""}
      </p>
    </>
  );
}
