import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";

import { ProfilesPanel } from "@/components/ProfilesPanel";
import { StatusStrip } from "@/components/StatusStrip";
import { GeneralTab } from "@/components/general/GeneralTab";
import { KeepAwakeTab } from "@/components/keepawake/KeepAwakeTab";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/motion/tabs";
import { useAppData } from "@/hooks/useAppData";
import { useAutostart } from "@/hooks/useAutostart";
import { useGeneral } from "@/hooks/useGeneral";
import { useKeepAwake } from "@/hooks/useKeepAwake";
import { useSizes } from "@/hooks/useSizes";
import { useSocketBudget } from "@/hooks/useSocketBudget";
import type { Locale } from "@/lib/api";
import { I18nProvider, LOCALE_NAMES, useT } from "@/lib/i18n";
import { PathNamesContext } from "@/lib/paths";

type TabId = "profiles" | "keep-awake" | "general";

/// The window's copy of `general::resolve_locale`: chosen wins, else the system
/// language subtag, else English. Split on the same delimiters as the Rust rule
/// (`-`, `_`, `.`) so a POSIX tag like `de.UTF-8` resolves the way Rust resolves
/// it. Two implementations of one rule is a thing to be uneasy about, but the
/// alternative — the backend reporting a resolved locale alongside the stored
/// one — means every save round-trips a value the window then ignores, and the
/// picker showing "Same as system" has to carry both. The rule is small and the
/// Rust side has the tests.
///
/// Called with `general.settings?.locale`, which is `undefined` until the first
/// read lands; that resolves the same as an explicit "follow system", so a user
/// whose stored choice differs from their system language could in principle see
/// one frame of the system language first. Not observable here: the window is
/// created `visible: false` and only shown from the tray, long after this mounts
/// and the read completes.
function resolveLocale(chosen: Locale | null | undefined): Locale {
  if (chosen) return chosen;
  const language = navigator.language.split(/[-_.]/)[0]?.toLowerCase() ?? "";
  return language in LOCALE_NAMES ? (language as Locale) : "en";
}

/// beUI's underline tabs are built for a page, not for a 36px chrome band: they
/// come at `text-sm` with a 44px touch target and an `inline-flex` list. The
/// overrides below put them on this window's scale — `twMerge` inside `cn` lets
/// the later class win — rather than forking the vendored component, which stays
/// verbatim so it can be re-synced from beui.dev.
const TAB_LIST = "flex h-9 shrink-0 items-stretch gap-4 border-hairline bg-surface px-5";
const TAB_TRIGGER = "min-h-0 px-0 py-0 text-callout font-normal";
/// The band that takes whatever the strip leaves. `min-h-0` is what lets it
/// shrink at all: a flex item's default `min-height: auto` refuses to go below
/// its content.
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
  const general = useGeneral(fail);
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
      <I18nProvider locale={resolveLocale(general.settings?.locale)}>
        {/* The window itself, not a card floating on a canvas: the title bar above
            it already carries the name, so this begins at the status strip. */}
        {/* Fixed height, three bands: a strip that does not move, the tab bar,
            and a panel that takes whatever is left. The window is resized by
            the user, not by how many profiles they have. */}
        <main className="flex h-screen flex-col overflow-hidden bg-bg font-sans text-ink">
          <StatusStrip
            profiles={counts.profiles}
            running={counts.running}
            bytes={sizes.total}
            approximate={sizes.totalApproximate}
            onError={fail}
          />

          <TabbedPanels
            tab={tab}
            onTab={setTab}
            profiles={
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
            }
            keepAwake={<KeepAwakeTab keepAwake={keepAwake} />}
            general={<GeneralTab general={general} autostart={autostart} />}
          />
        </main>
      </I18nProvider>
    </PathNamesContext.Provider>
  );
}

/// The tab bar and its three panels, split out from `App` because `App` is the
/// component that renders `I18nProvider` and so cannot call `useT` on its own
/// output — the provider has to be an ancestor of the call, not a sibling.
function TabbedPanels({
  tab,
  onTab,
  profiles,
  keepAwake,
  general,
}: {
  tab: TabId;
  onTab: (next: TabId) => void;
  profiles: ReactNode;
  keepAwake: ReactNode;
  general: ReactNode;
}) {
  const t = useT();
  return (
    <Tabs
      value={tab}
      onValueChange={(next) => onTab(next as TabId)}
      variant="underline"
      className="flex min-h-0 flex-1 flex-col"
    >
      <TabsList className={TAB_LIST}>
        <TabsTrigger value="profiles" className={TAB_TRIGGER}>
          {t("tab.profiles")}
        </TabsTrigger>
        <TabsTrigger value="keep-awake" className={TAB_TRIGGER}>
          {t("tab.keepAwake")}
        </TabsTrigger>
        <TabsTrigger value="general" className={TAB_TRIGGER}>
          {t("tab.general")}
        </TabsTrigger>
      </TabsList>

      {/* All three panels stay mounted — beUI hides the inactive ones, and
          Tailwind's preflight makes `[hidden]` win over the flex utilities here.
          Keeping them mounted is what preserves a half-typed profile name across
          a trip to another tab. */}
      <TabsContent value="profiles" className={TAB_PANEL}>
        {profiles}
      </TabsContent>
      <TabsContent value="keep-awake" className={TAB_PANEL}>
        {keepAwake}
      </TabsContent>
      <TabsContent value="general" className={TAB_PANEL}>
        {general}
      </TabsContent>
    </Tabs>
  );
}
