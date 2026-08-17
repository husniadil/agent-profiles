import { useCallback, useEffect, useRef, useState } from "react";

import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

import { errorMessage } from "@/lib/api";

export type UpdateState =
  | { kind: "disabled" }
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "downloading"; version: string; percent: number }
  | { kind: "installing"; version: string }
  | { kind: "failed"; reason: string };

export type Updater = {
  state: UpdateState;
  version: string;
  checkNow: () => Promise<void>;
};

/// Drives the updater from the window.
///
/// ponytail: this assumes the webview is alive whenever the app is, which holds
/// for this app — the window is created at launch with `visible: false` and
/// hidden rather than destroyed on close (see `close_hides_window`). If a future
/// build ever creates the window lazily, the check has to move into Rust's
/// `setup` and this hook becomes a reader of state it no longer owns.
export function useUpdater(autoUpdate: boolean | undefined): Updater {
  const [state, setState] = useState<UpdateState>({ kind: "idle" });
  const [version, setVersion] = useState("");
  /// One check at a time, and one automatic check per launch. Without this, the
  /// effect below would fire again every time `autoUpdate` is toggled off and on,
  /// and a second `downloadAndInstall` over the first would race two writers onto
  /// the same bundle.
  const busy = useRef(false);
  const checkedOnce = useRef(false);

  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  // Both the launch check and "Check now" run this same silent flow — check,
  // download, install, relaunch — with no confirmation step in between. That's
  // by design: the setting this hook serves is "install updates automatically",
  // and "Check now" just runs that flow on demand rather than waiting for it.
  const run = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    setState({ kind: "checking" });
    try {
      const update = await check();
      if (!update) {
        setState({ kind: "current" });
        return;
      }
      let total = 0;
      let received = 0;
      setState({ kind: "downloading", version: update.version, percent: 0 });
      // ponytail: no mid-download cancel — flipping the switch off during an
      // in-flight install still completes it. The launch guard already prevents
      // starting one while off; cancelling one already running would need an
      // AbortController the plugin doesn't expose. Add that if it proves
      // annoying.
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? 0;
            break;
          case "Progress":
            received += event.data.chunkLength;
            setState({
              kind: "downloading",
              version: update.version,
              // A server that sends no content-length leaves the percentage
              // meaningless rather than wrong: it stays at zero and the label
              // still says what is happening.
              percent: total > 0 ? Math.min(100, Math.round((received / total) * 100)) : 0,
            });
            break;
          case "Finished":
            setState({ kind: "installing", version: update.version });
            break;
        }
      });
      // ponytail: relaunch straight away. This app holds no unsaved state, and
      // the profiles it launched are separate processes that outlive it. On
      // Windows `installMode: "quiet"` closes us anyway, so a "restart later"
      // option would be a promise we could only keep on two platforms.
      await relaunch();
    } catch (cause) {
      setState({ kind: "failed", reason: errorMessage(cause) });
    } finally {
      busy.current = false;
    }
  }, []);

  useEffect(() => {
    if (autoUpdate === undefined) return; // settings have not landed yet
    if (!autoUpdate) {
      setState({ kind: "disabled" });
      return;
    }
    // Once per launch, not once per mount of this tab: the tab is mounted for
    // the life of the window, but the setting can be toggled, and re-checking on
    // every toggle would hammer GitHub for someone flipping a switch.
    if (checkedOnce.current) {
      setState((current) => (current.kind === "disabled" ? { kind: "idle" } : current));
      return;
    }
    checkedOnce.current = true;
    void run();
  }, [autoUpdate, run]);

  return {
    state,
    version,
    checkNow: useCallback(() => run(), [run]),
  };
}
