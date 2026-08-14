import { useCallback, useEffect, useMemo, useState } from "react";

import { AutostartRow } from "@/components/AutostartRow";
import { ComposeCard } from "@/components/ComposeCard";
import { EmptyState } from "@/components/EmptyState";
import { ErrorBanner } from "@/components/ErrorBanner";
import { ProfileList } from "@/components/ProfileList";
import { StatusStrip } from "@/components/StatusStrip";
import { useAppData } from "@/hooks/useAppData";
import { useAutostart } from "@/hooks/useAutostart";
import { useSizes } from "@/hooks/useSizes";
import { useSocketBudget } from "@/hooks/useSocketBudget";
import { PathNamesContext } from "@/lib/paths";

export default function App() {
  const data = useAppData();
  const { available, apps, fail, setError, reload, visit, listVersion } = data;

  const sizes = useSizes(available, visit);

  // Which app a new profile would be added to. Kept here rather than in the
  // form because the socket budget is a fact about the selected app, and the
  // meter sits next to the form rather than inside it.
  const [appId, setAppId] = useState("");
  useEffect(() => {
    // Whatever was chosen is kept if it is still installed. Clearing it when the
    // only installed app disappears matters: a stale id would create a profile
    // directory for an app that is no longer there to launch it.
    setAppId((chosen) =>
      available.some((app) => app.id === chosen) ? chosen : (available[0]?.id ?? ""),
    );
  }, [available]);

  // The meter is re-read when the picker changes and when the list reloads, and
  // by nothing else — never by the name field.
  const budget = useSocketBudget(appId, listVersion);

  const clearError = useCallback(() => setError(null), [setError]);
  const autostart = useAutostart(visit, fail, clearError);

  const counts = useMemo(
    () => ({
      profiles: available.reduce((total, app) => total + app.profiles.length, 0),
      running: available.reduce(
        (total, app) => total + app.profiles.filter((profile) => profile.running).length,
        0,
      ),
    }),
    [available],
  );

  // A desktop app has no business offering "Reload" or "Inspect Element" on
  // right-click. The caret menu stays inside text fields, where it is useful.
  useEffect(() => {
    const suppress = (event: MouseEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input, textarea")) return;
      event.preventDefault();
    };
    document.addEventListener("contextmenu", suppress);
    return () => document.removeEventListener("contextmenu", suppress);
  }, []);

  const paths = useMemo(
    () => ({ dataRoot: data.dataRoot, homePath: data.homePath }),
    [data.dataRoot, data.homePath],
  );

  return (
    <PathNamesContext.Provider value={paths}>
      {/* The window itself, not a card floating on a canvas: the title bar above
          it already carries the name, so this begins at the status strip. */}
      <main className="min-h-screen bg-bg font-sans text-ink">
        <StatusStrip
          profiles={counts.profiles}
          running={counts.running}
          bytes={sizes.total}
          onError={fail}
        />

        <div className="flex flex-col gap-2 p-2">
          <ErrorBanner message={data.error} />

          {available.length === 0 ? (
            <EmptyState apps={apps} />
          ) : (
            <ProfileList
              apps={available}
              sizes={sizes}
              reload={reload}
              onError={fail}
              clearError={clearError}
            />
          )}

          {/* With nothing to add a profile to, the whole band goes: a label over
              an empty space reads as something failing to load, and the form
              beneath it is a control that could only fail. */}
          {available.length > 0 ? (
            <ComposeCard
              apps={available}
              appId={appId}
              onAppId={setAppId}
              budget={budget}
              reload={reload}
              visit={visit}
            />
          ) : null}

          <AutostartRow autostart={autostart} />
        </div>
      </main>
    </PathNamesContext.Provider>
  );
}
