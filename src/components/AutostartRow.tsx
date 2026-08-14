import { Switch } from "@/components/motion/switch";
import type { Autostart } from "@/hooks/useAutostart";

/// The one setting the window carries, and the one control that does not own
/// its own state: the operating system does. The control is hidden entirely
/// where the platform does not offer it.
///
/// It sits on the floor of the window as chrome rather than as the last item in
/// the content stack: it belongs to the window, not to the list, and pinning it
/// there is what stops a short list from leaving a hole underneath.
export function AutostartRow({ autostart }: { autostart: Autostart }) {
  if (!autostart.state.offered) return null;

  return (
    <div className="flex h-10 shrink-0 items-center justify-between gap-4 border-t border-hairline bg-surface px-5">
      <div>
        <p className="text-[12px] text-ink">Start at login</p>
        <p className="text-[11px] text-ink-2">opens the tray only — no profile is launched</p>
      </div>

      {/* A `role="switch"` button rather than a checkbox: it is named here
          rather than by a `<label for>`, because the sentence to its left is
          two lines and only the first of them is the control's name. */}
      <Switch
        className="shrink-0"
        checked={autostart.state.enabled}
        onCheckedChange={(next) => void autostart.toggle(next)}
        ariaLabel="Start Agent Profiles at login"
      />
    </div>
  );
}
