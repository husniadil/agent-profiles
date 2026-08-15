import { useEffect, useState } from "react";

/// Whether this document has been looked at yet.
///
/// This window spends most of its life hidden behind a tray icon, and a hidden
/// document gets no animation frames. Anything whose *displayed* value is
/// produced by an animation — a ticker that starts at zero and climbs — would
/// therefore sit at zero for as long as the window stays away, and zero is not
/// a slow answer, it is a wrong one.
///
/// So: false while hidden, true the first time the document is shown, and true
/// from the outset when it is already on screen. It never goes back to false;
/// once a number has been counted up it has arrived, and re-hiding the window
/// is not a reason to un-arrive it.
export function useAwake(): boolean {
  const [awake, setAwake] = useState(() => !document.hidden);

  useEffect(() => {
    if (awake) return;
    const wake = () => {
      if (!document.hidden) setAwake(true);
    };
    document.addEventListener("visibilitychange", wake);
    return () => document.removeEventListener("visibilitychange", wake);
  }, [awake]);

  return awake;
}
