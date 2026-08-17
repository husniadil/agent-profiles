import { useCallback, useEffect, useMemo, useState } from "react";

import { AutostartRow } from "@/components/AutostartRow";
import { ProfilesPanel } from "@/components/ProfilesPanel";
import { StatusStrip } from "@/components/StatusStrip";
import { KeepAwakeTab } from "@/components/keepawake/KeepAwakeTab";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/motion/tabs";
import { useAppData } from "@/hooks/useAppData";
import { useAutostart } from "@/hooks/useAutostart";
import { useKeepAwake } from "@/hooks/useKeepAwake";
import { useSizes } from "@/hooks/useSizes";
import { useSocketBudget } from "@/hooks/useSocketBudget";
import { PathNamesContext } from "@/lib/paths";

type TabId = "profiles" | "keep-awake";

/// beUI's underline tabs are built for a page, not for a 36px chrome band: they
/// come at `text-sm` with a 44px touch target and an `inline-flex` list. The
/// overrides below put them on this window's scale — `twMerge` inside `cn` lets
/// the later class win — rather than forking the vendored component, which stays
/// verbatim so it can be re-synced from beui.dev.
const TAB_LIST = "flex h-9 shrink-0 items-stretch gap-4 border-hairline bg-surface px-5";
const TAB_TRIGGER = "min-h-0 px-0 py-0 text-callout font-normal";
/// The band that takes whatever the strip and the floor leave. `min-h-0` is what
/// lets it shrink at all: a flex item's default `min-height: auto` refuses to go
/// below its content.
const TAB_PANEL = "mt-0 flex min-h-0 flex-1 flex-col";

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
  const keepAwake = useKeepAwake(visit, fail);

  // Not reset on every visit: someone who left the window on Keep Awake was most
  // likely reading it, and reopening onto the profile list would throw that away
  // for no reason.
  const [tab, setTab] = useState<TabId>("profiles");

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
      {/* Fixed height, four bands: a strip that does not move, the tab bar, a
          panel that takes whatever is left, and a chrome bar on the floor. The
          window is resized by the user, not by how many profiles they have. */}
      <main className="flex h-screen flex-col overflow-hidden bg-bg font-sans text-ink">
        <StatusStrip
          profiles={counts.profiles}
          running={counts.running}
          bytes={sizes.total}
          onError={fail}
        />

        <Tabs
          value={tab}
          onValueChange={(next) => setTab(next as TabId)}
          variant="underline"
          className="flex min-h-0 flex-1 flex-col"
        >
          <TabsList className={TAB_LIST}>
            <TabsTrigger value="profiles" className={TAB_TRIGGER}>
              Agent Profiles
            </TabsTrigger>
            <TabsTrigger value="keep-awake" className={TAB_TRIGGER}>
              Keep Awake
            </TabsTrigger>
          </TabsList>

          {/* Both panels stay mounted — beUI hides the inactive one, and
              Tailwind's preflight makes `[hidden]` win over the flex utilities
              here. Keeping them mounted is what preserves a half-typed profile
              name across a trip to the other tab. */}
          <TabsContent value="profiles" className={TAB_PANEL}>
            <ProfilesPanel
              apps={apps}
              available={available}
              error={data.error}
              sizes={sizes}
              appId={appId}
              onAppId={setAppId}
              budget={budget}
              reload={reload}
              visit={visit}
              fail={fail}
              clearError={clearError}
            />
          </TabsContent>

          <TabsContent value="keep-awake" className={TAB_PANEL}>
            <KeepAwakeTab keepAwake={keepAwake} />
          </TabsContent>
        </Tabs>

        <AutostartRow autostart={autostart} />
      </main>
    </PathNamesContext.Provider>
  );
}
