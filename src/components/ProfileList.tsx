import { useEffect, useRef } from "react";
import { motion, useReducedMotion } from "motion/react";

import { ProfileRow } from "@/components/ProfileRow";
import type { AppView } from "@/lib/api";
import type { Sizes } from "@/hooks/useSizes";
import { useT } from "@/lib/i18n";

// The window is fixed and non-resizable, so its height is ours to get right, and
// the right height is exactly the content's — a tray window is a popover, not a
// panel to be filled. Below this floor the chrome (status line, add form, login
// bar) would start to clip; above the ceiling a long list would run off the
// screen, so past it the list scrolls inside the window instead of growing it.
const MIN_WINDOW_HEIGHT = 280;
const SCREEN_MARGIN = 0.92;

/// Sizes the window to its content, so a list of three profiles does not sit in
/// a window built for nine.
///
/// The whole calculation is one number: how far the list's natural height is
/// from the height the window currently gives it. That gap is the void when the
/// list underfills and the overflow when it spills, and adding it to the current
/// window height lands the window exactly on its content in a single set — the
/// content's height does not depend on the window's, so there is nothing to
/// converge, and re-measuring after the resize yields the same answer.
///
/// `scroller` is the clipped, scrolling box; `content` is its natural-height
/// child. Measuring the child is what sees past a scroller that has been
/// stretched to fill: a stretched scroller reports its own height as its
/// content, and the void would be invisible.
function useFitWindowToContent(
  scroller: React.RefObject<HTMLDivElement | null>,
  content: React.RefObject<HTMLDivElement | null>,
) {
  useEffect(() => {
    // Only inside the app, and imported only there: `@tauri-apps/api/window`
    // wires itself to Tauri's IPC the moment it loads, which throws in the
    // browser preview. A static import would run that on every page; a dynamic
    // one behind this guard runs it only where the IPC exists.
    if (!("__TAURI_INTERNALS__" in window)) return;
    let disposed = false;
    let observer: ResizeObserver | undefined;

    void import("@tauri-apps/api/window").then(({ getCurrentWindow, LogicalSize }) => {
      if (disposed) return;
      const fit = () => {
        const box = scroller.current;
        const inner = content.current;
        if (!box || !inner) return;
        // Before first layout the content measures zero; sizing to that would
        // slam the window down to its floor for a frame. Wait for real rows.
        if (inner.offsetHeight < 40) return;
        const delta = inner.offsetHeight - box.clientHeight;
        const ceiling = Math.round(window.screen.availHeight * SCREEN_MARGIN);
        const target = Math.min(
          ceiling,
          Math.max(MIN_WINDOW_HEIGHT, window.innerHeight + delta),
        );
        if (Math.abs(target - window.innerHeight) <= 1) return;
        void getCurrentWindow().setSize(
          new LogicalSize(window.innerWidth, target),
        );
      };

      // Fires on the first layout and on every content change after it — a
      // profile added or removed, a size arriving and reflowing a row.
      observer = new ResizeObserver(fit);
      if (content.current) observer.observe(content.current);
    });

    return () => {
      disposed = true;
      observer?.disconnect();
    };
  }, [scroller, content]);
}

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
  const scroller = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  useFitWindowToContent(scroller, content);

  // Apps that work first, their reasons after — the order the tray builds its
  // rows in. Interleaving them by registry position would put "Claude Desktop
  // is not installed" above the ChatGPT profiles in one surface and below them
  // in the other, for the same machine.
  const ordered = [
    ...apps.filter((app) => !app.unavailable),
    ...apps.filter((app) => app.unavailable),
  ];
  // The same count the tray heads its menu on: an app that cannot be used
  // brings no rows to tell apart from anyone else's, and its own sentence names
  // its product. Counting it would give someone with one app installed seven
  // headed sections where the tray gives them a flat list.
  const headed = apps.filter((app) => !app.unavailable).length > 1;

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
      <div
        ref={scroller}
        className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto p-1 [scrollbar-gutter:stable]"
      >
        {/* Natural height, never stretched: this is what the window is sized to
            match, so it must report the content's height and not the scroller's. */}
        <div ref={content} data-fit-content>
          {ordered.map((app, index) => (
          <section key={app.id} className={index > 0 ? "mt-1" : undefined}>
            {/* A heading only earns its space once there is a second *usable*
                app to tell apart. With one app installed the list is simply the
                list, and an app that cannot be used names its own product in
                its sentence, so it needs no heading over it either. */}
            {headed ? (
              <div className="flex items-center gap-2 px-2 pt-1.5 pb-0.5">
                {/* Sentence case, no tracking. The typeface was always the
                    system one; the setting was not — small caps on a wide track
                    is a dashboard's idea of a section label. macOS sets a group
                    heading in plain semibold at the subheadline size, the way
                    the Finder sidebar and System Settings do, and this window
                    sits next to both. */}
                <h2 className="text-sub font-semibold text-ink-2">
                  {app.label}
                </h2>
                <span aria-hidden="true" className="h-px flex-1 bg-hairline" />
              </div>
            ) : null}
            {app.unavailable ? (
              <Unavailable reason={app.unavailable} />
            ) : (
              <Rows
                app={app}
                sizes={sizes}
                reload={reload}
                onError={onError}
                clearError={clearError}
              />
            )}
          </section>
          ))}
        </div>
      </div>
    </div>
  );
}

/// An app this tool knows about but cannot use, saying why.
///
/// Greyed rather than hidden: an app that simply disappears is indistinguishable
/// from one this tool never supported, and the profiles under it are not gone —
/// they come back the moment the app is installed.
///
/// The reason stays the backend's words: it differs by platform and by cause —
/// a binary missing from `/Applications`, a command absent from `PATH`, a
/// registry that could not be read — and that is the only side that knows what
/// it looked for and where. What it cannot do is stand alone, because this row
/// occupies the same slot and the same classes as the translated "no profiles
/// yet" line directly below: the same visual row would be Indonesian for an
/// empty app and English for a missing one, which reads as the one sentence the
/// app forgot to translate. So the frame is translated and the reason is
/// quoted inside it, the way `AwakeStatusCard` sets a verbatim backend error
/// inside a translated band.
function Unavailable({ reason }: { reason: string }) {
  const t = useT();
  return (
    // `ink-2`, not the fainter `ink-3`: this row is quiet because it offers
    // nothing to do, not because it is less worth reading — a sentence set below
    // the contrast floor is one nobody can act on.
    <p className="px-2 py-1.5 text-callout text-ink-2">
      {t("profiles.unavailable", { reason })}
    </p>
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
  const t = useT();
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
    return <p className="px-2 py-1.5 text-callout text-ink-2">{t("profiles.empty")}</p>;
  }

  return (
    <ul>
      {app.profiles.map((profile) => {
        const key = `${profile.app_id}:${profile.id}`;
        const row = (
          <ProfileRow
            profile={profile}
            bytes={sizes.byKey[key]}
            sizeApproximate={sizes.approximate.has(key)}
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
