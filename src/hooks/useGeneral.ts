import { useCallback, useEffect, useState } from "react";

import * as api from "@/lib/api";
import type { GeneralSettings } from "@/lib/api";

export type General = {
  /// Null until the first read lands. Every consumer has to handle it, because
  /// rendering English for one frame and then switching would be a visible flash
  /// of the wrong language.
  settings: GeneralSettings | null;
  save: (patch: Partial<GeneralSettings>) => Promise<void>;
};

/// Read once, not polled. Nothing outside this window changes these two values —
/// unlike keep-awake, whose numbers a background sweep keeps moving.
export function useGeneral(fail: (error: unknown) => void): General {
  const [settings, setSettings] = useState<GeneralSettings | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        setSettings(await api.generalSettings());
      } catch (cause) {
        fail(cause);
      }
    })();
  }, [fail]);

  const save = useCallback(
    async (patch: Partial<GeneralSettings>) => {
      if (!settings) return;
      try {
        // The command returns what it stored, so what is on screen is the
        // backend's answer rather than an optimistic guess.
        setSettings(await api.setGeneralSettings({ ...settings, ...patch }));
      } catch (cause) {
        fail(cause);
      }
    },
    [settings, fail],
  );

  return { settings, save };
}
