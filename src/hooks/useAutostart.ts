import { useCallback, useEffect, useState } from "react";

import * as api from "@/lib/api";
import type { AutostartState } from "@/lib/api";

export type Autostart = {
  state: AutostartState;
  toggle: (enabled: boolean) => Promise<void>;
};

/// The operating system owns this setting, so the control is refreshed from it
/// rather than remembered here — the user may have changed it in System
/// Settings, and the OS may refuse what the click asked for. Every toggle is
/// followed by a re-read, so the switch shows what is actually true rather than
/// what was wanted.
export function useAutostart(
  visit: number,
  fail: (error: unknown) => void,
  clearError: () => void,
): Autostart {
  const [state, setState] = useState<AutostartState>({ offered: false, enabled: false });

  const load = useCallback(async () => {
    try {
      setState(await api.autostartState());
    } catch (cause) {
      setState({ offered: false, enabled: false });
      fail(cause);
    }
  }, [fail]);

  useEffect(() => {
    void load();
  }, [load, visit]);

  const toggle = useCallback(
    async (enabled: boolean) => {
      try {
        await api.setAutostart(enabled);
        clearError();
      } catch (cause) {
        fail(cause);
      }
      await load();
    },
    [load, fail, clearError],
  );

  return { state, toggle };
}
