import { useCallback, useEffect, useState } from "react";

import * as api from "@/lib/api";
import type { KeepAwakeSettings, KeepAwakeStatus } from "@/lib/api";

/// How often the window re-reads what the sweep decided.
///
/// Faster than the backend's own fifteen-second sweep on purpose: the numbers on
/// screen — how long ago an agent wrote, how long the hold has run — move
/// continuously, and a reader watching them should not have to wonder whether
/// the window has frozen. It costs a mutex read; the sweep does the work.
const POLL_MS = 3000;

export type KeepAwake = {
  status: KeepAwakeStatus | null;
  busy: boolean;
  save: (settings: KeepAwakeSettings) => Promise<void>;
  authorize: () => Promise<void>;
  restore: () => Promise<void>;
};

export function useKeepAwake(visit: number, fail: (error: unknown) => void): KeepAwake {
  const [status, setStatus] = useState<KeepAwakeStatus | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setStatus(await api.keepAwakeStatus());
    } catch (cause) {
      fail(cause);
    }
  }, [fail]);

  // A hidden window is not being read. Mirrors `useAppData`: the poll stops with
  // visibility and picks up again on the next visit.
  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | null = null;
    const start = () => {
      if (timer !== null || document.hidden) return;
      timer = setInterval(() => void refresh(), POLL_MS);
    };
    const stop = () => {
      if (timer === null) return;
      clearInterval(timer);
      timer = null;
    };
    const follow = () => (document.hidden ? stop() : start());

    void refresh();
    follow();
    document.addEventListener("visibilitychange", follow);
    return () => {
      stop();
      document.removeEventListener("visibilitychange", follow);
    };
  }, [refresh, visit]);

  /// Every mutation returns the whole status, so the answer on screen is the one
  /// the backend just computed rather than an optimistic guess followed by a
  /// correction three seconds later.
  const run = useCallback(
    async (action: () => Promise<KeepAwakeStatus>) => {
      setBusy(true);
      try {
        setStatus(await action());
      } catch (cause) {
        fail(cause);
        // The action failed, so whatever is on screen may now be wrong about
        // more than the one field that was being changed.
        void refresh();
      } finally {
        setBusy(false);
      }
    },
    [fail, refresh],
  );

  return {
    status,
    busy,
    save: useCallback(
      (settings: KeepAwakeSettings) => run(() => api.setKeepAwake(settings)),
      [run],
    ),
    authorize: useCallback(() => run(api.authorizeKeepAwake), [run]),
    restore: useCallback(() => run(api.restoreSleep), [run]),
  };
}
