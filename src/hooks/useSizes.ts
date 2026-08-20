import { useEffect, useRef, useState } from "react";

import { profileSizeBytes, type AppView, type ProfileSize } from "@/lib/api";

export type Sizes = {
  /// Bytes per profile, keyed `app:profile`. Absent until that row reports.
  byKey: Record<string, number>;
  /// The rows whose walk threw, keyed the same way.
  ///
  /// Read-only, and the reason it exists: a failed walk publishes nothing at
  /// all, so without it a row that could not be measured and a row that has not
  /// been reached yet are the same absence — and they are not the same thing to
  /// say. In neither `byKey` nor here means still to come.
  failed: ReadonlySet<string>;
  /// The rows whose walk could not reach everything, keyed the same way.
  ///
  /// Different from `failed`: these rows have a number, it is simply short by an
  /// unknown amount — a subtree the walk could not descend into. Shown, but
  /// marked, because an unmarked short figure is the one thing worse than no
  /// figure.
  approximate: ReadonlySet<string>;
  /// Absent until every row has reported: a total that counts half the profiles
  /// is a wrong number stated confidently.
  total: number | null;
  /// Whether any row that went into `total` was itself short. One approximate
  /// row makes the sum approximate; there is no partial version of this.
  totalApproximate: boolean;
};

/// Filling in the size of every row, one row at a time, after the list is drawn.
///
/// Sequentially rather than all at once: each of these is a walk of a whole
/// profile directory, and a dozen of them in flight together turns opening the
/// window into a disk storm. Top to bottom also reads as progress.
///
/// Every new list retires whatever measuring was still running. A walk started
/// for the previous list can still be waiting on the disk when a profile is
/// deleted, and its bytes belong to a list that is no longer on screen — so the
/// guard is a generation counter in a ref, read at the moment each answer lands.
/// A ref and not state: the pass has to compare against the newest value the
/// instant it resumes, and a state variable captured by the closure would still
/// be holding the number that was current when the pass began.
export function useSizes(apps: AppView[], visit: number): Sizes {
  // The generation of the newest pass. Written before any awaiting starts, so a
  // pass that resumes after a newer one began sees the newer number.
  const generation = useRef(0);
  // Sizes already measured this visit. A directory only grows while its app is
  // running, and the window is a place you visit for a moment to rename or
  // delete something — so measuring once per visit is fresh enough, and
  // re-measuring on every list reload is not. Renaming a profile cannot change a
  // byte of it, and neither can opening it, but both reload the list.
  const cache = useRef(new Map<string, ProfileSize>());
  const lastVisit = useRef(visit);

  const [byKey, setByKey] = useState<Record<string, number>>({});
  const [failed, setFailed] = useState<ReadonlySet<string>>(() => new Set());
  const [approximate, setApproximate] = useState<ReadonlySet<string>>(() => new Set());
  const [total, setTotal] = useState<number | null>(null);
  const [totalApproximate, setTotalApproximate] = useState(false);

  useEffect(() => {
    // A new visit is the one moment the numbers could have moved without us.
    if (lastVisit.current !== visit) {
      lastVisit.current = visit;
      cache.current.clear();
    }

    const pass = (generation.current += 1);
    const targets = apps.flatMap((app) =>
      app.profiles.map((profile) => ({ appId: profile.app_id, id: profile.id })),
    );

    // A fresh list restarts the measuring, so the total stops claiming what the
    // previous list added up to.
    setByKey({});
    setFailed(new Set());
    setApproximate(new Set());
    setTotal(null);
    setTotalApproximate(false);

    void (async () => {
      // The running sum and the published rows both belong to this pass alone.
      // Held in state and merged into, they would be shared with whatever pass
      // replaced us, and the bytes of a profile that has since been deleted
      // would land in the new list's total.
      let sum = 0;
      let complete = true;
      const found: Record<string, number> = {};
      // Pass-local for the same reason `found` is: a row that could not be read
      // belongs to the list it was read for.
      const missed = new Set<string>();
      // Pass-local for the same reason, and separate from `missed`: a row that
      // came back short still has a number to show.
      const short = new Set<string>();

      // One place decides what a walk's answer means, so a cached row and a
      // freshly measured one cannot disagree about whether they are exact.
      const record = (key: string, size: ProfileSize) => {
        sum += size.bytes;
        found[key] = size.bytes;
        setByKey({ ...found });
        if (size.skipped > 0) {
          short.add(key);
          setApproximate(new Set(short));
        }
      };

      for (const target of targets) {
        const key = `${target.appId}:${target.id}`;
        const known = cache.current.get(key);
        if (known !== undefined) {
          record(key, known);
          continue;
        }
        try {
          const size = await profileSizeBytes(target.appId, target.id);
          // The list this row belongs to has been replaced; the bytes are about
          // a row nobody is looking at.
          if (pass !== generation.current) return;
          cache.current.set(key, size);
          record(key, size);
        } catch {
          if (pass !== generation.current) return;
          // No banner: a size that could not be read is not an action that
          // failed, and the row still says everything else it has to say. The
          // remaining rows are still worth filling in, so the walk carries on —
          // but the total is now unknowable, and a total missing a profile is a
          // wrong total.
          complete = false;
          // Said out loud, though, so the row can stop waiting: silence is how
          // "still measuring" looks, and this row has finished failing.
          missed.add(key);
          setFailed(new Set(missed));
        }
      }

      if (pass !== generation.current || !complete) return;
      setTotal(sum);
      setTotalApproximate(short.size > 0);
    })();
  }, [apps, visit]);

  return { byKey, failed, approximate, total, totalApproximate };
}
