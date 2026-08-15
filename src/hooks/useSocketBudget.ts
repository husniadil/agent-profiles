import { useEffect, useState } from "react";

import { socketBudget, type SocketBudget } from "@/lib/api";

/// The socket-path budget for the app currently selected in the picker.
///
/// Not wired to the label field, and deliberately so: `ProfileStore::add` names
/// a profile directory after a generated id, never after what was typed, so the
/// number cannot move as the user types. It is a property of the data root — on
/// most machines a comfortable constant, and on a long home directory the reason
/// no profile can be created at all. It is re-read when the picker changes and
/// when the list reloads, and nothing else moves it.
///
/// `reload` is a plain counter rather than a dependency array so the caller can
/// say "ask again" without this hook having to know why.
export function useSocketBudget(appId: string, reload: number): SocketBudget | null {
  const [budget, setBudget] = useState<SocketBudget | null>(null);

  useEffect(() => {
    if (!appId) {
      setBudget(null);
      return;
    }
    // The picker can move on while this is in flight. Two apps sit at two
    // depths, so an answer about the one that was selected a moment ago is the
    // wrong number under the app that is selected now.
    let current = true;
    void (async () => {
      try {
        const next = await socketBudget(appId);
        if (current) setBudget(next);
      } catch {
        // No banner, for the same reason the data root has none: this is a
        // reading the window offers, not an action the user asked for.
        if (current) setBudget(null);
      }
    })();
    return () => {
      current = false;
    };
  }, [appId, reload]);

  return budget;
}
