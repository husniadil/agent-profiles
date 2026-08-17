import { AlertTriangle, Moon, ShieldCheck } from "lucide-react";

import { Button } from "@/components/motion/button/base";
import type { KeepAwake } from "@/hooks/useKeepAwake";
import type { KeepAwakeStatus, Phase } from "@/lib/api";
import { useT, type T } from "@/lib/i18n";
import { systemNames } from "@/lib/system";

/// The same 28px control the compose row uses, so a button in this tab is the
/// same object as a button in the other one.
const CONTROL = "h-7 rounded-lg px-2.5 text-callout";

/// What each phase says, in the second person, with the reason attached.
///
/// The honesty requirement for this feature: a user who trusted it and closed
/// the lid has to be able to tell why their machine slept anyway. "Paused" on
/// its own would leave them guessing, and "keeping this Mac awake" while a guard
/// has already dropped the hold would be a lie they only discover by losing
/// work.
///
/// A function of the machine's own name rather than a constant, because there
/// are three of them now. Saying "your Mac" on a ThinkPad is the small kind of
/// wrong that tells a reader this feature was not built for them — and on the
/// tab whose whole job is to be believed about their hardware.
function phases(
  t: T,
  machine: string,
): Record<Phase, { title: string; detail: string; dot: string; tone: string }> {
  const Machine = machine[0].toUpperCase() + machine.slice(1);
  return {
    off: {
      title: t("awake.off.title"),
      detail: t("awake.off.detail", { machine: Machine }),
      dot: "bg-ink-4",
      tone: "text-ink-2",
    },
    idle: {
      title: t("awake.idle.title"),
      detail: t("awake.idle.detail"),
      dot: "bg-ink-4",
      tone: "text-ink-2",
    },
    holding: {
      title: t("awake.holding.title", { machine }),
      detail: t("awake.holding.detail"),
      // The same green the tray's running dot uses. Holding is the same kind of
      // fact as a profile running, so it is deliberately not a second green.
      dot: "bg-live",
      tone: "text-ink",
    },
    "paused-low-battery": {
      title: t("awake.lowBattery.title"),
      detail: t("awake.lowBattery.detail"),
      dot: "bg-warning",
      tone: "text-ink",
    },
    "paused-too-hot": {
      title: t("awake.tooHot.title", { machine }),
      detail: t("awake.tooHot.detail"),
      dot: "bg-warning",
      tone: "text-ink",
    },
  };
}

/// The seconds are kept inside the first hour, and this is why.
///
/// Whole minutes read as a broken clock. Past sixty seconds the old version
/// showed "1m" and then did not change again for a whole minute — and the one
/// question this figure exists to answer is whether the app is still watching.
/// A counter that sits still for sixty seconds answers "no", which is how a
/// working detector got reported as a dead one.
///
/// Past an hour the minute is fine: nobody watches a three-hour figure for
/// proof of life, and `3h 36m 04s` is three facts where one was asked for.
export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
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
  const t = useT();
  const { system, machine } = systemNames(t);

  // Ordered by what blocks what. An unsupported platform makes authorization
  // meaningless, and a stranded machine is a live problem that outranks a
  // feature the user has not turned on yet.
  if (!status.supported) {
    return (
      <Band
        icon={<Moon size={13} aria-hidden="true" className="mt-0.5 shrink-0" />}
        tone="text-ink-2"
        title={t("awake.band.unavailable")}
      >
        {/* No longer "this needs a Mac": all three platforms hold now, so the
            only way to land here is a machine that genuinely cannot. On Linux
            that has one overwhelmingly likely cause and it is worth naming —
            "unsupported" with no reason is what sends someone to an issue
            tracker to ask. */}
        {status.refusal ??
          (system === "Linux"
            ? t("awake.unsupported.linux")
            : t("awake.unsupported.generic", { system, machine }))}
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
        title={t("awake.band.stranded")}
      >
        {t("awake.stranded")}
        <span className="mt-1.5 flex flex-wrap items-center gap-2.5">
          <Button
            size="sm"
            className={CONTROL}
            disabled={keepAwake.busy}
            onClick={() => void keepAwake.restore()}
          >
            {t("awake.restoreSleep")}
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

  // Only where a password is genuinely coming. Linux takes a logind inhibitor
  // as the signed-in user and Windows writes a power scheme that user already
  // owns; a button offering to authorize either would be asking for permission
  // that was never withheld, and teaching the user this app needs admin rights
  // it does not need.
  //
  // This leads the card and outranks the phase: authorization is the first gate,
  // so until it is granted the trigger and its limits below stay locked and this
  // Authorize button is the only thing to do here.
  if (status.needs_authorization && !status.authorized) {
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
        title={t("awake.band.notAuthorized")}
      >
        {t("awake.needsPassword")}
        <span className="mt-1.5 flex">
          <Button
            size="sm"
            className={CONTROL}
            disabled={keepAwake.busy}
            onClick={() => void keepAwake.authorize()}
          >
            {t("awake.authorize")}
          </Button>
        </span>
      </Band>
    );
  }

  // A failed hold outranks the phase, because it contradicts it: the decision
  // was to hold and the platform could not carry it out. Saying "keeping this
  // machine awake" here would be the single lie this feature cannot afford.
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
        title={t("awake.band.holdFailed")}
      >
        {/* The reason is the backend's, verbatim, because it differs by
            platform and by cause: a folder that cannot be written, a logind
            lock that was refused, a power scheme held by policy. A single
            rewritten sentence here would have to be vague enough to cover all
            three, which is the same as saying nothing. */}
        {t("awake.band.holdFailedDetail", {
          machine: machine[0].toUpperCase() + machine.slice(1),
          error: status.hold_error,
        })}
      </Band>
    );
  }

  const phase = phases(t, machine)[status.phase];
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
          ? t("awake.status.noBattery")
          : t("awake.status.battery", { percent: status.battery_percent }) +
            (status.on_external_power ? t("awake.status.pluggedIn") : "")}
        {status.held_for_secs > 0
          ? t("awake.status.held", {
              duration: formatDuration(status.held_for_secs),
            })
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
