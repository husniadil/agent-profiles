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
    <div className="rounded-xl border border-hairline bg-surface p-1 shadow-card">
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

  return (
    <ul>
      {app.profiles.map((profile) => {
        const row = (
          <ProfileRow
            profile={profile}
            bytes={sizes.byKey[`${profile.app_id}:${profile.id}`]}
            reload={reload}
            onError={onError}
            clearError={clearError}
          />
        );
        const arriving = !still && arrivals.has(profile.id);
        return (
          <li key={profile.id}>
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
