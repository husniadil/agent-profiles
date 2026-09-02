import { useCallback, useEffect, useState } from "react";

import * as api from "@/lib/api";
import type { ScheduleSettings, ScheduleStatus } from "@/lib/api";

export type Schedule = {
  status: ScheduleStatus | null;
  busy: boolean;
  save: (settings: ScheduleSettings) => Promise<void>;
  clear: () => Promise<void>;
};

/// Read once, not polled — unlike keep-awake, a schedule has no background sweep
/// moving numbers. Re-reads on each `visit` (a fresh trip to the window), the
/// same signal the other read-once hooks follow.
export function useSchedule(visit: number, fail: (error: unknown) => void): Schedule {
  const [status, setStatus] = useState<ScheduleStatus | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setStatus(await api.getSchedule());
    } catch (cause) {
      fail(cause);
    }
  }, [fail]);

  useEffect(() => {
    void refresh();
  }, [refresh, visit]);

  /// Every mutation returns the whole status, so what is on screen is what the
  /// backend just stored — including a password prompt the user cancelled, which
  /// leaves the previous settings in place.
  const run = useCallback(
    async (action: () => Promise<ScheduleStatus>) => {
      setBusy(true);
      try {
        setStatus(await action());
      } catch (cause) {
        fail(cause);
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
      (settings: ScheduleSettings) => run(() => api.setSchedule(settings)),
      [run],
    ),
    clear: useCallback(() => run(api.clearSchedule), [run]),
  };
}
