import { Switch } from "@/components/motion/switch";
import type { Autostart } from "@/hooks/useAutostart";

/// The one setting the window carries, and the one control that does not own
/// its own state: the operating system does. The control is hidden entirely
/// where the platform does not offer it.
export function AutostartRow({ autostart }: { autostart: Autostart }) {
  if (!autostart.state.offered) return null;

  return (
    <div className="flex items-center justify-between gap-4 px-1 pt-0.5">
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
