import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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

/// Everything about a list that the window actually draws.
///
/// Used to decide whether an arriving list is news. Polling for liveness means
/// most answers are identical to the last one, and setting state anyway would
/// hand `available` a new identity every few seconds — which is the array the
/// size measurement keys its generations on, so it would retire and restart a
/// directory walk on a timer for no reason.
function signature(apps: AppView[]): string {
  return apps
    .map(
      (app) =>
        `${app.id}|${app.unavailable ?? ""}|` +
        app.profiles
          .map((p) => `${p.id},${p.label},${p.running ? 1 : 0},${p.shares_account ? 1 : 0}`)
          .join(";"),
    )
    .join("//");
}

/// How often the window re-asks who is running, while someone is looking at it.
///
/// `running` is a snapshot taken when the list is read. Without this, quitting
/// an agent while this window is open leaves its row claiming RUNNING until the
/// window is closed and reopened — the tray rescans on every hover and the
/// window would be the one surface telling a stale story.
const LIVENESS_POLL_MS = 2500;

export function useAppData(): AppData {
  const [apps, setApps] = useState<AppView[]>([]);
  const [dataRoot, setDataRoot] = useState("");
  const [homePath, setHomePath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [visit, setVisit] = useState(0);
  const [listVersion, setListVersion] = useState(0);

  const fail = useCallback((cause: unknown) => setError(api.errorMessage(cause)), []);

  const drawn = useRef("");

  const reload = useCallback(async () => {
    try {
      const next = await api.listApps();
      const mark = signature(next);
      if (mark !== drawn.current) {
        drawn.current = mark;
        setApps(next);
        setListVersion((version) => version + 1);
      }
      setError(null);
    } catch (cause) {
      fail(cause);
    }
  }, [fail]);

  /// A quiet re-read of who is running, only while the window is on screen.
  ///
  /// A hidden window is not being read, and the scan costs a process sweep, so
  /// there is nothing to keep fresh for a reader who is not there. It picks up
  /// again on the next visibility change, and `window-shown` reloads anyway.
  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | null = null;

    const start = () => {
      if (timer !== null || document.hidden) return;
      timer = setInterval(() => void reload(), LIVENESS_POLL_MS);
    };
    const stop = () => {
      if (timer === null) return;
      clearInterval(timer);
      timer = null;
    };
    const follow = () => (document.hidden ? stop() : start());

    follow();
    document.addEventListener("visibilitychange", follow);
    return () => {
      stop();
      document.removeEventListener("visibilitychange", follow);
    };
  }, [reload]);

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
