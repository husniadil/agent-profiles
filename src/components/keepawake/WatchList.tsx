import { formatDuration } from "@/components/keepawake/AwakeStatusCard";
import { StateTag } from "@/components/StateTag";
import type { Freshness } from "@/lib/api";

/// The roots being watched, and how long ago each was last written.
///
/// Shown rather than hidden because the trigger is otherwise unfalsifiable: a
/// user whose agent is plainly working, and whose machine slept anyway, has no way to
/// tell whether the detector was wrong or whether they were watching the wrong
/// folder. This turns that into something they can look at.
export function WatchList({
  roots,
  windowMinutes,
}: {
  roots: Freshness[];
  windowMinutes: number;
}) {
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
    // 88px leaves a row half-showing rather than cutting cleanly between two,
    // which is the affordance that says there is more without spending a
    // scrollbar's worth of chrome to say it.
    <ul className="flex max-h-[88px] flex-col gap-1 overflow-y-auto">
      {roots.map((root) => {
        // Three states, not two. "Working" is now a claim the transcript makes
        // — the agent's turn is open — rather than one inferred from a file
        // having been touched, because a transcript is written when a turn ends
        // as well as while it runs. A row that has gone quiet says so.
        const fresh = root.seconds_ago !== null && root.seconds_ago <= window;
        const active = root.mid_turn && fresh;
        // Mid-turn but past the window: something stopped between a tool call
        // and its result. Named rather than folded into "idle", because those
        // are different things and only one of them is a session that finished.
        const stalled = root.mid_turn && root.seconds_ago !== null && !fresh;
        return (
          <li
            key={root.path}
            className="flex items-center justify-between gap-3 text-sub"
          >
            <span className="flex min-w-0 items-center gap-1.5">
              {/* The only thing on this tab that moves on its own, and it earns
                  that by marking the one condition the whole feature turns on:
                  an agent working right now. Idle rows get a flat dot, so the
                  beat is the signal rather than decoration.

                  Stopped, not slowed, under reduced motion: the tag beside it
                  already says "working", so holding the dot still costs the
                  reader nothing. */}
              <span
                aria-hidden="true"
                className={`size-1.5 shrink-0 rounded-full ${
                  active
                    ? "bg-live animate-live-heartbeat motion-reduce:animate-none"
                    : "bg-ink-4"
                }`}
              />
              <span className="truncate text-ink-2">{root.label}</span>
              {/* The same chip a running profile carries on the other tab.
                  "Running" and "Working" are the same kind of claim, and the
                  word is what a screen reader and a colour-blind reader get —
                  the dot and its ring are decoration on top of it. */}
              {active ? (
                <StateTag token="var(--live)" status="success">
                  Working
                </StateTag>
              ) : null}
            </span>
            <span className="shrink-0 font-mono text-ink-3">
              {root.seconds_ago === null
                ? "never"
                : active
                  ? `${formatDuration(root.seconds_ago)} ago`
                  : stalled
                    ? `stalled ${formatDuration(root.seconds_ago)}`
                    : `idle ${formatDuration(root.seconds_ago)}`}
            </span>
          </li>
        );
      })}
    </ul>
  );
}
