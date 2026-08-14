import { useEffect, useState } from "react";
import { useReducedMotion } from "motion/react";

import { AnimatedListItem } from "@/components/magicui/animated-list";
import { ProfileRow } from "@/components/ProfileRow";
import type { AppView } from "@/lib/api";
import type { Sizes } from "@/hooks/useSizes";

/// How the rows arrive: one after another, a beat apart.
///
/// This is MagicUI's `AnimatedList` behaviour with its one assumption removed.
/// That component is built for a notification stack — it reverses its children
/// so the newest lands on top — and a profile list is not a stack: the Default
/// profile is first because it is first. So the reveal is counted here and each
/// row is still handed to MagicUI's `AnimatedListItem` to make its entrance.
///
/// It only ever counts up. Opening, renaming or deleting a profile reloads the
/// list, and re-staggering the same rows every time would be motion that says
/// nothing.
function useReveal(count: number, still: boolean, step = 45): number {
  const [shown, setShown] = useState(0);

  useEffect(() => {
    if (still) {
      setShown(count);
      return;
    }
    if (shown >= count) return;
    const timer = setTimeout(() => setShown((seen) => seen + 1), shown === 0 ? 0 : step);
    return () => clearTimeout(timer);
  }, [shown, count, step, still]);

  return Math.min(shown, count);
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
    <div className="rounded-xl border border-hairline bg-surface p-1.5 shadow-card">
      {apps.map((app, index) => (
        <section key={app.id} className={index > 0 ? "mt-1" : undefined}>
          {/* A heading only earns its space once there is a second app to tell
              apart. With one app installed the list is simply the list. */}
          {apps.length > 1 ? (
            <div className="flex items-center gap-2.5 px-2 pt-2 pb-1">
              <h2 className="font-wide text-[11px] font-semibold tracking-[0.06em] text-ink-2 uppercase [font-stretch:112%]">
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
  const shown = useReveal(app.profiles.length, still);

  return (
    <ul>
      {app.profiles.slice(0, shown).map((profile) => {
        const row = (
          <ProfileRow
            profile={profile}
            bytes={sizes.byKey[`${profile.app_id}:${profile.id}`]}
            reload={reload}
            onError={onError}
            clearError={clearError}
          />
        );
        return (
          <li key={profile.id}>{still ? row : <AnimatedListItem>{row}</AnimatedListItem>}</li>
        );
      })}
    </ul>
  );
}
