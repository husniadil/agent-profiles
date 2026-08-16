import { AlertTriangle, Moon, ShieldCheck } from "lucide-react";

import { Button } from "@/components/motion/button/base";
import type { KeepAwake } from "@/hooks/useKeepAwake";
import type { KeepAwakeStatus, Phase } from "@/lib/api";

/// The same 28px control the compose row uses, so a button in this tab is the
/// same object as a button in the other one.
const CONTROL = "h-7 rounded-lg px-2.5 text-callout";

/// What each phase says, in the second person, with the reason attached.
///
/// The honesty requirement for this feature: a user who trusted it and closed
/// the lid has to be able to tell why their Mac slept anyway. "Paused" on its own
/// would leave them guessing, and "keeping your Mac awake" while a guard has
/// already dropped the hold would be a lie they only discover by losing work.
const PHASES: Record<
  Phase,
  { title: string; detail: string; dot: string; tone: string }
> = {
  off: {
    title: "Off",
    detail: "Your Mac sleeps when you close the lid, as usual.",
    dot: "bg-ink-4",
    tone: "text-ink-2",
  },
  idle: {
    title: "Watching",
    detail: "Nothing is working right now, so nothing is being held.",
    dot: "bg-ink-4",
    tone: "text-ink-2",
  },
  holding: {
    title: "Keeping your Mac awake",
    detail: "You can close the lid — sleep returns when the work stops.",
    // The same green the tray's running dot uses. Holding is the same kind of
    // fact as a profile running, so it is deliberately not a second green.
    dot: "bg-live",
    tone: "text-ink",
  },
  "paused-low-battery": {
    title: "Paused — battery low",
    detail: "Dropped to protect the battery. Plug in to resume.",
    dot: "bg-warning",
    tone: "text-ink",
  },
  "paused-too-hot": {
    title: "Paused — your Mac is too hot",
    detail: "Holding it awake would make that worse. It resumes once it cools.",
    dot: "bg-warning",
    tone: "text-ink",
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

/// The first band of the settings card: what is true right now, and the one
/// action that changes it when there is one.
///
/// A band rather than a card of its own. The panel already sits inside the same
/// bordered shell the compose card uses on the other tab, and a second box
/// inside it would be a card in a card — which is why this returns bare rows
/// and lets the parent own the border.
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
      <Band
        icon={<Moon size={13} aria-hidden="true" className="mt-0.5 shrink-0" />}
        tone="text-ink-2"
        title="Not available here"
      >
        {status.refusal ??
          "Holding the lid closed needs macOS. On Windows and Linux the lid action belongs to the system power plan, with no way for an app to override it."}
      </Band>
    );
  }

  if (status.stranded) {
    return (
      <Band
        icon={
          <AlertTriangle
            size={13}
            aria-hidden="true"
            className="mt-0.5 shrink-0"
          />
        }
        tone="text-warning"
        title="Your Mac may not be able to sleep"
      >
        Agent Profiles ended unexpectedly while holding the lid-closed state,
        and that setting survives a restart.
        <span className="mt-1.5 flex flex-wrap items-center gap-2.5">
          <Button
            size="sm"
            className={CONTROL}
            disabled={keepAwake.busy}
            onClick={() => void keepAwake.restore()}
          >
            Restore sleep
          </Button>
          {/* Said out loud as well as offered as a button: someone who would
              rather not hand this app a password should still leave knowing
              exactly how to fix their machine. */}
          <code className="font-mono text-sub text-ink-3">
            sudo pmset -a disablesleep 0
          </code>
        </span>
      </Band>
    );
  }

  if (!status.authorized) {
    return (
      <Band
        icon={
          <ShieldCheck
            size={13}
            aria-hidden="true"
            className="mt-0.5 shrink-0"
          />
        }
        tone="text-ink-2"
        title="Not yet authorized"
      >
        Needs an administrator password once per run. A helper turns the setting
        on while an agent works, off when it stops, and shuts down with Agent
        Profiles.
        <span className="mt-1.5 flex">
          <Button
            size="sm"
            className={CONTROL}
            disabled={keepAwake.busy}
            onClick={() => void keepAwake.authorize()}
          >
            Authorize…
          </Button>
        </span>
      </Band>
    );
  }

  // A failed flag write outranks the phase, because it contradicts it: the
  // decision was to hold, and the one channel that could carry that decision to
  // the privileged loop did not take it. Saying "keeping your Mac awake" here
  // would be the single lie this feature cannot afford.
  if (status.hold_error !== null) {
    return (
      <Band
        icon={
          <AlertTriangle
            size={13}
            aria-hidden="true"
            className="mt-0.5 shrink-0"
          />
        }
        tone="text-warning"
        title="Not holding — the flag could not be written"
      >
        Your Mac will sleep as usual. Agent Profiles could not write to its own
        folder: {status.hold_error}
      </Band>
    );
  }

  const phase = PHASES[status.phase];
  return (
    <Band
      icon={
        <span
          aria-hidden="true"
          className={`mt-1.5 size-1.5 shrink-0 rounded-full ${phase.dot}`}
        />
      }
      tone={phase.tone}
      title={phase.title}
    >
      {phase.detail}{" "}
      <span className="text-ink-3">
        {status.battery_percent === null
          ? "No battery"
          : `Battery ${status.battery_percent}%${status.on_external_power ? ", plugged in" : ""}`}
        {status.held_for_secs > 0
          ? ` · held ${formatDuration(status.held_for_secs)}`
          : ""}
      </span>
    </Band>
  );
}

/// Title over detail, with the marker in a fixed 14px gutter so every state
/// lines its text up at the same left edge — a dot and an icon are different
/// widths, and without the gutter the sentence shifts when the state changes.
function Band({
  icon,
  tone,
  title,
  children,
}: {
  icon: React.ReactNode;
  tone: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex gap-1.5">
      <span className={`flex w-3.5 shrink-0 justify-center ${tone}`}>
        {icon}
      </span>
      <div className="min-w-0 flex-1">
        <p className={`text-callout font-medium ${tone}`}>{title}</p>
        <p className="mt-0.5 text-sub text-ink-2">{children}</p>
      </div>
    </div>
  );
}
