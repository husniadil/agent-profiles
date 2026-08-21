import { useCallback, useEffect, useRef, useState } from "react";

import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

import {
  errorMessage,
  releaseKeepAwakeForUpdate,
  resumeKeepAwakeAfterFailedUpdate,
} from "@/lib/api";

export type UpdateState =
  | { kind: "disabled" }
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "downloading"; version: string; percent: number }
  | { kind: "installing"; version: string }
  /// `phase` is which step failed: "check" means we never reached an update —
  /// the manifest was unreachable or unreadable, which is not the same news as
  /// an actual update that failed to install ("install"). The window says very
  /// different things for the two, so a 404 on the endpoint reads as "couldn't
  /// check" rather than the alarming "could not update".
  | { kind: "failed"; phase: "check" | "install"; reason: string };

export type Updater = {
  state: UpdateState;
  version: string;
  lastChecked: number | null;
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
  const [lastChecked, setLastChecked] = useState<number | null>(null);
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
    // Which step we are on when something throws: until an update is in hand the
    // failure is a failed *check*, after that a failed *install*.
    let phase: "check" | "install" = "check";
    // Held outside the `try` so the `catch` can release it. `download()` parks the
    // bundle bytes in the webview's resource table (a `DownloadedBytes` rid); the
    // plugin only frees that rid inside a *successful* `install()`. If anything
    // between download and a clean install throws — `releaseKeepAwakeForUpdate()`
    // or `install()` itself — the rid is orphaned, and since each retry runs a
    // fresh `check()`/`download()` it would leak another whole bundle for the life
    // of the webview. `update.close()` frees both the bytes rid and the update
    // handle; on the success path we never reach the `catch` and `install()` has
    // already nulled the bytes, so there is no double free.
    let update: Update | null = null;
    try {
      update = await check();
      setLastChecked(Date.now());
      if (!update) {
        setState({ kind: "current" });
        return;
      }
      // Narrowed alias so the closures below stay non-null: `update` is a `let`
      // (the `catch` needs it), which TypeScript widens back to `Update | null`
      // inside a callback since it cannot prove no reassignment. `ready` is the
      // same handle, and closing either in the `catch` frees the same resources.
      const ready = update;
      phase = "install";
      let total = 0;
      let received = 0;
      setState({ kind: "downloading", version: ready.version, percent: 0 });
      // Download and install are split, rather than the single
      // `downloadAndInstall`, so the keep-awake hold can be handed back in
      // between — see the release below. A download failure therefore leaves the
      // hold untouched: nothing has been released until the package is in hand.
      //
      // ponytail: no mid-download cancel — flipping the switch off during an
      // in-flight install still completes it. The launch guard already prevents
      // starting one while off; cancelling one already running would need an
      // AbortController the plugin doesn't expose. Add that if it proves
      // annoying.
      await ready.download((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? 0;
            break;
          case "Progress":
            received += event.data.chunkLength;
            setState({
              kind: "downloading",
              version: ready.version,
              // A server that sends no content-length leaves the percentage
              // meaningless rather than wrong: it stays at zero and the label
              // still says what is happening.
              percent: total > 0 ? Math.min(100, Math.round((received / total) * 100)) : 0,
            });
            break;
          case "Finished":
            break;
        }
      });
      setState({ kind: "installing", version: ready.version });
      // Hand the machine back BEFORE installing, not after: the install exits
      // the process itself — on Windows through the NSIS installer and
      // `std::process::exit(0)`, which reaches neither `RunEvent::ExitRequested`
      // nor `RunEvent::Exit` — so the release wired into Rust's `App::run` would
      // never fire and the lid-close action would stay on "do nothing" past the
      // update. This is the last point our own code runs. The trigger is left
      // armed, so the relaunched app holds again if it should.
      await releaseKeepAwakeForUpdate();
      await ready.install();
      // ponytail: relaunch straight away. This app holds no unsaved state, and
      // the profiles it launched are separate processes that outlive it. On
      // Windows `installMode: "quiet"` closes us anyway, so a "restart later"
      // option would be a promise we could only keep on two platforms.
      await relaunch();
    } catch (cause) {
      // The install did not take the process with it, so hand keep-awake back its
      // sweep: `releaseKeepAwakeForUpdate` paused it so nothing could re-arm the
      // hold in the gap before the installer took over, and leaving that pause set
      // would silently switch the feature off for the rest of this run. Safe to
      // call even when the failure came before the release — resuming a sweep that
      // was never paused is a no-op — and guarded so it cannot mask the real cause.
      try {
        await resumeKeepAwakeAfterFailedUpdate();
      } catch {
        // best effort: nothing useful can be done on the error path itself
      }
      // Free the downloaded bundle the plugin would otherwise retain (see above).
      // Guarded so a close failure never masks the real error we are reporting.
      try {
        await update?.close();
      } catch {
        // best effort: the leak is the lesser problem than losing the real cause
      }
      setState({ kind: "failed", phase, reason: errorMessage(cause) });
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
    lastChecked,
    // "Off means no network" lives here, not only on the button's disabled
    // prop: a manual check must never fire while the switch is off (or before
    // settings land, when `autoUpdate` is undefined).
    checkNow: useCallback(async () => {
      if (!autoUpdate) return;
      await run();
    }, [autoUpdate, run]),
  };
}
