import { Switch } from "@/components/motion/switch";
import type { Autostart } from "@/hooks/useAutostart";

/// The one setting the window carries, and the one control that does not own its
/// own state: the operating system does.
///
/// It sits on the floor of the window as chrome rather than as the last item in
/// the content stack: it belongs to the window, not to the list, and pinning it
/// there is what stops a short list from leaving a hole underneath.
///
/// Where the platform will not take the setting the row stays, greyed, and says
/// why. It used to disappear, which answered the question "can this app start at
/// login?" with silence — a control that is missing teaches nothing, while a
/// control that is present and explains itself teaches the whole rule. The only
/// case today is a development build, where a login item would point at a binary
/// under `target/debug` that moves, is rebuilt, and vanishes on `cargo clean`.
export function AutostartRow({ autostart }: { autostart: Autostart }) {
  const { offered, enabled } = autostart.state;

  return (
    <div className="flex h-10 shrink-0 items-center justify-between gap-4 border-t border-hairline bg-surface px-5">
      <div>
        <p className={offered ? "text-[12px] text-ink" : "text-[12px] text-ink-2"}>
          Start at login
        </p>
        <p className="text-[11px] text-ink-2">
          {offered
            ? "opens the tray only — no profile is launched"
            : "available once Agent Profiles is installed"}
        </p>
      </div>

      {/* A `role="switch"` button rather than a checkbox: it is named here
          rather than by a `<label for>`, because the sentence to its left is
          two lines and only the first of them is the control's name. */}
      <Switch
        className="shrink-0"
        checked={enabled}
        disabled={!offered}
        onCheckedChange={(next) => void autostart.toggle(next)}
        ariaLabel="Start Agent Profiles at login"
      />
    </div>
  );
}
