import { AlertTriangle, Moon, ShieldCheck, Sun } from "lucide-react";
import type { ReactNode } from "react";

import type { KeepAwake } from "@/hooks/useKeepAwake";
import type { KeepAwakeStatus, Phase } from "@/lib/api";

/// What each phase says, in the second person, with the reason attached.
///
/// The honesty requirement for this feature: a user who trusted it and closed
/// the lid has to be able to tell why their Mac slept anyway. "Paused" on its own
/// would leave them guessing, and "keeping your Mac awake" while a guard has
/// already dropped the hold would be a lie they only discover by losing work.
const PHASES: Record<Phase, { title: string; detail: string; tone: string }> = {
  off: {
    title: "Off",
    detail: "Your Mac sleeps when you close the lid, as usual.",
    tone: "text-ink-2",
  },
  idle: {
    title: "Watching",
    detail: "Nothing is working right now, so nothing is being held.",
    tone: "text-ink-2",
  },
  holding: {
    title: "Keeping your Mac awake",
    detail: "You can close the lid. Sleep is restored when the work stops.",
    // The same green the tray's running dot uses, mapped in `styles.css`.
    // Holding is the same kind of fact as a profile running, so it is
    // deliberately not a second green.
    tone: "text-live",
  },
  "paused-low-battery": {
    title: "Paused — battery low",
    detail: "The hold was dropped to protect the battery. Plug in to resume.",
    tone: "text-warning",
  },
  "paused-cap-reached": {
    title: "Paused — time limit reached",
    detail: "This hold ran its full length. It resumes when the agent next starts.",
    tone: "text-warning",
  },
};

/// Whole units only. A hold measured to the second implies a precision the
/// fifteen-second sweep does not have.
export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

export function AwakeStatusCard({
  status,
  keepAwake,
}: {
  status: KeepAwakeStatus;
  keepAwake: KeepAwake;
}) {
  // Ordered by what blocks what. An unsupported platform makes authorization
  // meaningless, and a stranded machine is a live problem that outranks a
  // feature the user has not turned on yet.
  if (!status.supported) {
    return (
      <Card
        icon={<Moon size={15} aria-hidden="true" />}
        tone="text-ink-2"
        title="Not available here"
      >
        {status.refusal ??
          "Holding the lid-closed state needs macOS. On Windows and Linux the lid action belongs to the system power plan, with no way for an app to override it."}
      </Card>
    );
  }

  if (status.stranded) {
    return (
      <Card
        icon={<AlertTriangle size={15} aria-hidden="true" />}
        tone="text-warning"
        title="Your Mac may not be able to sleep"
      >
        <p>
          Agent Profiles ended unexpectedly while holding the lid-closed state, and that setting
          survives a restart. Restoring it needs your administrator password once.
        </p>
        <div className="mt-2 flex flex-wrap items-center gap-3">
          <button
            type="button"
            disabled={keepAwake.busy}
            onClick={() => void keepAwake.restore()}
            className="rounded-md bg-accent px-2.5 py-1 text-callout text-accent-ink disabled:opacity-50"
          >
            Restore sleep
          </button>
          {/* Said out loud as well as offered as a button: someone who would
              rather not hand this app a password should still leave knowing
              exactly how to fix their machine. */}
          <code className="font-mono text-sub text-ink-3">sudo pmset -a disablesleep 0</code>
        </div>
      </Card>
    );
  }

  if (!status.authorized) {
    return (
      <Card
        icon={<ShieldCheck size={15} aria-hidden="true" />}
        tone="text-ink-2"
        title="Not yet authorized"
      >
        <p>
          Holding the lid closed sets a system power setting that needs an administrator password —
          once per run of this app, never again while it is open. A helper then turns that setting on
          while an agent works and off when it stops, and it shuts down with Agent Profiles.
        </p>
        <button
          type="button"
          disabled={keepAwake.busy}
          onClick={() => void keepAwake.authorize()}
          className="mt-2 rounded-md bg-accent px-2.5 py-1 text-callout text-accent-ink disabled:opacity-50"
        >
          Authorize…
        </button>
      </Card>
    );
  }

  const phase = PHASES[status.phase];
  return (
    <Card icon={<Sun size={15} aria-hidden="true" />} tone={phase.tone} title={phase.title}>
      <p>{phase.detail}</p>
      <p className="mt-1 text-sub text-ink-3">
        {status.battery_percent === null
          ? "No battery on this Mac"
          : `Battery ${status.battery_percent}%${status.on_external_power ? ", plugged in" : ""}`}
        {status.held_for_secs > 0 ? ` · held ${formatDuration(status.held_for_secs)}` : ""}
      </p>
    </Card>
  );
}

function Card({
  icon,
  tone,
  title,
  children,
}: {
  icon: ReactNode;
  tone: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="rounded-lg border border-hairline bg-surface p-3">
      <p className={`flex items-center gap-1.5 text-callout ${tone}`}>
        {icon}
        {title}
      </p>
      <div className="mt-1 text-sub text-ink-2">{children}</div>
    </div>
  );
}
