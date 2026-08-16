import { AlertTriangle, Moon, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";

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
const PHASES: Record<Phase, { title: string; detail: string; dot: string; tone: string }> = {
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
  "paused-cap-reached": {
    title: "Paused — time limit reached",
    detail: "This hold ran its full length. It resumes when the agent next starts.",
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
      <Notice icon={<Moon size={14} aria-hidden="true" />} tone="text-ink-2" title="Not available here">
        {status.refusal ??
          "Holding the lid closed needs macOS. On Windows and Linux the lid action belongs to the system power plan, with no way for an app to override it."}
      </Notice>
    );
  }

  if (status.stranded) {
    return (
      <Notice
        icon={<AlertTriangle size={14} aria-hidden="true" />}
        tone="text-warning"
        title="Your Mac may not be able to sleep"
      >
        <p>
          Agent Profiles ended unexpectedly while holding the lid-closed state, and that setting
          survives a restart.
        </p>
        <div className="mt-1.5 flex flex-wrap items-center gap-2.5">
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
          <code className="font-mono text-sub text-ink-3">sudo pmset -a disablesleep 0</code>
        </div>
      </Notice>
    );
  }

  if (!status.authorized) {
    return (
      <Notice
        icon={<ShieldCheck size={14} aria-hidden="true" />}
        tone="text-ink-2"
        title="Not yet authorized"
      >
        <p>
          Needs an administrator password once per run. A helper turns the setting on while an agent
          works, off when it stops, and shuts down with Agent Profiles.
        </p>
        <Button
          size="sm"
          className={`${CONTROL} mt-1.5`}
          disabled={keepAwake.busy}
          onClick={() => void keepAwake.authorize()}
        >
          Authorize…
        </Button>
      </Notice>
    );
  }

  // Nothing to act on, so nothing that looks like it. A bordered, shadowed card
  // here would outrank the trigger below it — and the trigger is the control,
  // while this is only the readout. The box is reserved for the three states
  // above, which do want something from the reader.
  const phase = PHASES[status.phase];
  return (
    <div className="flex flex-col gap-0.5">
      <p className={`flex items-center gap-1.5 text-callout ${phase.tone}`}>
        <span aria-hidden="true" className={`size-1.5 shrink-0 rounded-full ${phase.dot}`} />
        {phase.title}
      </p>
      <p className="text-sub text-ink-2">
        {phase.detail}{" "}
        <span className="text-ink-3">
          {status.battery_percent === null
            ? "No battery"
            : `Battery ${status.battery_percent}%${status.on_external_power ? ", plugged in" : ""}`}
          {status.held_for_secs > 0 ? ` · held ${formatDuration(status.held_for_secs)}` : ""}
        </span>
      </p>
    </div>
  );
}

/// The boxed form, for the states that want something from the reader.
/// Same shell as the compose card on the other tab — same radius, same padding,
/// same hairline, same shadow — so a panel that asks for something looks the
/// same in both places.
function Notice({
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
    <section className="rounded-xl border border-hairline bg-surface p-2.5 shadow-card">
      <p className={`flex items-center gap-1.5 text-callout font-medium ${tone}`}>
        {icon}
        {title}
      </p>
      <div className="mt-1 text-sub text-ink-2">{children}</div>
    </section>
  );
}
