import { formatDuration } from "@/components/keepawake/AwakeStatusCard";
import type { Freshness } from "@/lib/api";

/// The roots being watched, and how long ago each was last written.
///
/// Shown rather than hidden because the trigger is otherwise unfalsifiable: a
/// user whose agent is plainly working, and whose Mac slept anyway, has no way to
/// tell whether the detector was wrong or whether they were watching the wrong
/// folder. This turns that into something they can look at.
export function WatchList({ roots, windowMinutes }: { roots: Freshness[]; windowMinutes: number }) {
  if (roots.length === 0) {
    return (
      <p className="text-sub text-ink-3">
        Nothing to watch yet. Claude Code and Codex are found automatically once they have written a
        session.
      </p>
    );
  }

  const window = windowMinutes * 60;
  return (
    // The one unbounded thing on this tab: two agent CLIs plus a row per Codex
    // profile, so a user with several profiles can run past the panel. It scrolls
    // inside its own box rather than pushing the trigger and the limits out of
    // view — the controls stay put at every window size, and the diagnostic list
    // is the part that gives.
    // 72px leaves a row half-showing rather than cutting cleanly between two,
    // which is the affordance that says there is more without spending a
    // scrollbar's worth of chrome to say it.
    <ul className="flex max-h-[72px] flex-col gap-1 overflow-y-auto">
      {roots.map((root) => {
        const active = root.seconds_ago !== null && root.seconds_ago <= window;
        return (
          <li key={root.path} className="flex items-baseline justify-between gap-3 text-sub">
            <span className="flex min-w-0 items-baseline gap-1.5">
              <span
                aria-hidden="true"
                className={`size-1.5 shrink-0 self-center rounded-full ${
                  active ? "bg-live" : "bg-ink-4"
                }`}
              />
              <span className="truncate text-ink-2">{root.label}</span>
            </span>
            <span className="shrink-0 font-mono text-ink-3">
              {root.seconds_ago === null
                ? "never"
                : active
                  ? `active ${formatDuration(root.seconds_ago)} ago`
                  : `idle ${formatDuration(root.seconds_ago)}`}
            </span>
          </li>
        );
      })}
    </ul>
  );
}
