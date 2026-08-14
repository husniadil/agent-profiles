import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { homeDir } from "@tauri-apps/api/path";

import * as api from "@/lib/api";
import type { AppView } from "@/lib/api";

export type AppData = {
  apps: AppView[];
  /// The apps that are actually installed. The status line only ever describes
  /// these and so does the list below it, so the filter runs once here and
  /// everything reads the same answer.
  available: AppView[];
  dataRoot: string;
  homePath: string;
  error: string | null;
  setError: (error: string | null) => void;
  fail: (error: unknown) => void;
  reload: () => Promise<void>;
  /// Bumped every time the window is shown. Anything cached for the length of a
  /// visit — the measured sizes, the autostart reading — watches this.
  visit: number;
  /// Bumped every time a list actually arrives. The socket budget is re-read on
  /// this and on the app picker, and on nothing else.
  listVersion: number;
};

export function useAppData(): AppData {
  const [apps, setApps] = useState<AppView[]>([]);
  const [dataRoot, setDataRoot] = useState("");
  const [homePath, setHomePath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [visit, setVisit] = useState(0);
  const [listVersion, setListVersion] = useState(0);

  const fail = useCallback((cause: unknown) => setError(api.errorMessage(cause)), []);

  const reload = useCallback(async () => {
    try {
      setApps(await api.listApps());
      setListVersion((version) => version + 1);
      setError(null);
    } catch (cause) {
      fail(cause);
    }
  }, [fail]);

  // Asked for again on every visit rather than cached. The root cannot change
  // while the app runs, so this is not about freshness — it is the retry for a
  // first attempt that failed.
  //
  // A failed re-read leaves the last known-good path where it is. Replacing a
  // correct answer with an empty one would be the only way to lose it, and it
  // would strand the tooltip too, leaving a path on hover over nothing. There is
  // no banner either: the window works perfectly without knowing where the files
  // are, and the banner is reserved for actions that failed.
  const loadDataRoot = useCallback(async () => {
    try {
      setDataRoot(await api.dataRoot());
    } catch {
      /* keep whatever we already knew */
    }
  }, []);

  // The home directory is a fact about the machine, not about this window, so it
  // is read once at startup and never again. Every path stays correct without
  // it — only longer.
  useEffect(() => {
    void homeDir()
      .then((home) => setHomePath(home.replace(/[\\/]+$/, "")))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    void reload();
    void loadDataRoot();
  }, [reload, loadDataRoot]);

  // Closing the window only hides it, so the page keeps whatever it was last
  // showing. An error like "quit this profile's Claude Desktop before deleting
  // it" is a verdict about one moment — by the time the window is reopened the
  // user has very likely done exactly that. Start every visit freshly loaded.
  useEffect(() => {
    const stop = listen("window-shown", () => {
      setError(null);
      setVisit((count) => count + 1);
      void reload();
      void loadDataRoot();
    });
    return () => {
      void stop.then((off) => off());
    };
  }, [reload, loadDataRoot]);

  // Memoised, and not merely for speed: this array is the identity the size
  // measurement keys its generations on. Rebuilt on every render it would retire
  // its own pass the moment that pass published its first row.
  const available = useMemo(() => apps.filter((app) => app.unavailable === null), [apps]);

  return {
    apps,
    available,
    dataRoot,
    homePath,
    error,
    setError,
    fail,
    reload,
    visit,
    listVersion,
  };
}
