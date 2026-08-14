import { useEffect, useRef } from "react";
import { motion, useReducedMotion } from "motion/react";

import { ProfileRow } from "@/components/ProfileRow";
import type { AppView } from "@/lib/api";
import type { Sizes } from "@/hooks/useSizes";

/// Which rows are new since the last time this list was drawn.
///
/// Deliberately not a staggered entrance on every load. This window is opened
/// for a couple of seconds to get into a profile, and rows that animate in each
/// time are choreography the reader has to wait out. Worse, gating them on a
/// timer means a throttled window renders an *empty* list rather than a plain
/// one — the reveal has to enhance something already on screen, never replace it.
///
/// A row that was not here a moment ago is different: that is the profile you
/// just added, and saying so is state, which is the only thing motion is for.
function useArrivals(ids: string[]): Set<string> {
  const seen = useRef<Set<string> | null>(null);
  const arrivals = useRef<Set<string>>(new Set());

  // First list wins silently: everything already there is not an arrival.
  if (seen.current === null) {
    seen.current = new Set(ids);
  } else {
    const fresh = ids.filter((id) => !seen.current!.has(id));
    if (fresh.length > 0) arrivals.current = new Set(fresh);
  }

  useEffect(() => {
    for (const id of ids) seen.current!.add(id);
  }, [ids]);

  return arrivals.current;
}

export function ProfileList({
  apps,
  sizes,
  reload,
  onError,
  clearError,
}: {
  apps: AppView[];
  sizes: Sizes;
  reload: () => Promise<void>;
  onError: (error: unknown) => void;
  clearError: () => void;
}) {
  return (
    // The frame takes the height the window has left and clips to its own
    // corners, so a hover fill and an arriving row stay inside the radius.
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border border-hairline bg-surface shadow-card">
      {/* The padding lives on the scroller rather than the frame: a focus ring
          is 2px of outline at 2px offset, and it has to fit inside this `p-1`
          without reaching the edge and summoning a horizontal scrollbar.
          `scrollbar-gutter: stable` reserves the classic scrollbar Windows and
          Linux draw — without it, the first row that overflows would shove the
          trailing column ~15px left and knock the size figures out of line. */}
      <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto p-1 [scrollbar-gutter:stable]">
        {apps.map((app, index) => (
          <section key={app.id} className={index > 0 ? "mt-1" : undefined}>
            {/* A heading only earns its space once there is a second app to tell
                apart. With one app installed the list is simply the list. */}
            {apps.length > 1 ? (
              <div className="flex items-center gap-2 px-2 pt-1.5 pb-0.5">
                <h2 className="font-wide text-[10px] font-semibold tracking-[0.06em] text-ink-2 uppercase [font-stretch:112%]">
                  {app.label}
                </h2>
                <span aria-hidden="true" className="h-px flex-1 bg-hairline" />
              </div>
            ) : null}
            <Rows
              app={app}
              sizes={sizes}
              reload={reload}
              onError={onError}
              clearError={clearError}
            />
          </section>
        ))}
      </div>
    </div>
  );
}

function Rows({
  app,
  sizes,
  reload,
  onError,
  clearError,
}: {
  app: AppView;
  sizes: Sizes;
  reload: () => Promise<void>;
  onError: (error: unknown) => void;
  clearError: () => void;
}) {
  const still = useReducedMotion() ?? false;
  const arrivals = useArrivals(app.profiles.map((profile) => profile.id));
  const arrival = useRef<HTMLLIElement | null>(null);

  // The form is pinned to the floor and the list scrolls, so the profile just
  // added can land below the fold — and an arrival animation nobody sees is
  // not an animation. Keyed on the arrivals set itself, which is replaced only
  // when a row is genuinely new: the 2.5s liveness poll re-renders this list
  // and must never move the page under the reader.
  useEffect(() => {
    arrival.current?.scrollIntoView({
      block: "nearest",
      behavior: still ? "auto" : "smooth",
    });
  }, [arrivals, still]);

  // An installed app with nothing in it says so. An empty list inside a frame
  // that now runs the full height of the window reads as something that failed
  // to load rather than as a fact about this app.
  if (app.profiles.length === 0) {
    return <p className="px-2 py-1.5 text-[12px] text-ink-2">No profiles yet.</p>;
  }

  return (
    <ul>
      {app.profiles.map((profile) => {
        const key = `${profile.app_id}:${profile.id}`;
        const row = (
          <ProfileRow
            profile={profile}
            bytes={sizes.byKey[key]}
            sizeFailed={sizes.failed.has(key)}
            reload={reload}
            onError={onError}
            clearError={clearError}
          />
        );
        const arrived = arrivals.has(profile.id);
        const arriving = !still && arrived;
        return (
          // Scrolled-to rows stop short of the frame's top edge rather than
          // sitting flush against it.
          <li key={profile.id} ref={arrived ? arrival : null} className="scroll-mt-6">
            {/* The row the user just created, saying so once. Scaled up from
                its own top edge so it grows into the gap it made rather than
                shoving the rows below it around. */}
            {arriving ? (
              <motion.div
                className="w-full"
                initial={{ scale: 0, opacity: 0 }}
                animate={{ scale: 1, opacity: 1, originY: 0 }}
                transition={{ type: "spring", stiffness: 350, damping: 40 }}
              >
                {row}
              </motion.div>
            ) : (
              row
            )}
          </li>
        );
      })}
    </ul>
  );
}
