import type { Autostart } from "@/hooks/useAutostart";

/// The one setting the window carries, and the one control that does not own
/// its own state: the operating system does. The control is hidden entirely
/// where the platform does not offer it.
export function AutostartRow({ autostart }: { autostart: Autostart }) {
  if (!autostart.state.offered) return null;

  return (
    <div className="flex items-center justify-between gap-4 px-1 pt-1">
      <div>
        <p className="text-[13px] text-ink">Start at login</p>
        <p className="text-[12px] text-ink-2">opens the tray only — no profile is launched</p>
      </div>

      <label className="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center">
        <input
          type="checkbox"
          className="peer sr-only"
          aria-label="Start Agent Profiles at login"
          checked={autostart.state.enabled}
          onChange={(event) => void autostart.toggle(event.target.checked)}
        />
        <span className="pointer-events-none absolute inset-0 rounded-full bg-line transition-colors duration-150 ease-out peer-checked:bg-accent peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-accent" />
        <span className="pointer-events-none absolute left-0.5 size-4 rounded-full bg-surface shadow-card transition-transform duration-150 ease-out peer-checked:translate-x-4" />
      </label>
    </div>
  );
}
