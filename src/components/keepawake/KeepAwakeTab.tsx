import { AwakeStatusCard } from "@/components/keepawake/AwakeStatusCard";
import { WatchList } from "@/components/keepawake/WatchList";
import type { KeepAwake } from "@/hooks/useKeepAwake";
import type { KeepAwakeSettings, Trigger } from "@/lib/api";

const TRIGGERS: { id: Trigger; label: string; detail: string }[] = [
  { id: "off", label: "Off", detail: "Never hold the machine awake." },
  {
    id: "agent-active",
    label: "When an agent is working",
    detail: "Held while a Claude Code or Codex session is being written to.",
  },
  {
    id: "always",
    label: "Always while Agent Profiles runs",
    detail: "For agents inside a desktop app, where there is nothing to detect.",
  },
];

type LimitKey = keyof Omit<KeepAwakeSettings, "trigger">;

/// The limits, and the range each is clamped to on the way in. The bounds are
/// restated here so the field cannot offer a number the backend will silently
/// change underneath it.
const LIMITS: {
  key: LimitKey;
  label: string;
  unit: string;
  min: number;
  max: number;
  detail: string;
}[] = [
  {
    key: "idle_window_minutes",
    label: "Agent idle window",
    unit: "minutes",
    min: 1,
    max: 60,
    detail: "How long a session may go quiet before its agent counts as finished.",
  },
  {
    key: "battery_floor_percent",
    label: "Pause below",
    unit: "% battery",
    min: 0,
    max: 95,
    detail: "Ignored while plugged in — a charging Mac cannot run flat.",
  },
  {
    key: "max_hold_minutes",
    label: "Stop after",
    unit: "minutes",
    min: 5,
    max: 1440,
    detail: "With the lid shut nothing can be reported, so this runs out silently.",
  },
];

export function KeepAwakeTab({ keepAwake }: { keepAwake: KeepAwake }) {
  const status = keepAwake.status;
  if (!status) {
    // Blank rather than a spinner: the first read lands in a few milliseconds,
    // and a spinner that flashes for one frame is noise, not feedback.
    return (
      <section id="panel-keep-awake" role="tabpanel" className="flex min-h-0 flex-1 flex-col p-3" />
    );
  }

  const { settings } = status;
  const change = (patch: Partial<KeepAwakeSettings>) =>
    void keepAwake.save({ ...settings, ...patch });

  return (
    <section
      id="panel-keep-awake"
      role="tabpanel"
      className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3"
    >
      <AwakeStatusCard status={status} keepAwake={keepAwake} />

      <fieldset disabled={!status.supported} className="flex flex-col gap-1.5 disabled:opacity-50">
        <legend className="text-callout text-ink">Hold the machine awake</legend>
        {TRIGGERS.map((trigger) => (
          <label key={trigger.id} className="flex items-start gap-2">
            <input
              type="radio"
              name="keep-awake-trigger"
              className="mt-0.5 accent-accent"
              checked={settings.trigger === trigger.id}
              onChange={() => change({ trigger: trigger.id })}
            />
            <span>
              <span className="text-callout text-ink">{trigger.label}</span>
              <span className="block text-sub text-ink-2">{trigger.detail}</span>
            </span>
          </label>
        ))}
      </fieldset>

      <fieldset
        disabled={!status.supported || settings.trigger === "off"}
        className="flex flex-col gap-2 disabled:opacity-50"
      >
        <legend className="text-callout text-ink">Limits</legend>
        {LIMITS.map((limit) => (
          <label key={limit.key} className="flex items-baseline justify-between gap-3">
            <span className="min-w-0">
              <span className="text-callout text-ink">{limit.label}</span>
              <span className="block text-sub text-ink-2">{limit.detail}</span>
            </span>
            <span className="flex shrink-0 items-baseline gap-1.5">
              <input
                type="number"
                min={limit.min}
                max={limit.max}
                value={settings[limit.key]}
                onChange={(event) => {
                  const next = Number(event.target.value);
                  // The backend clamps too. This only stops the field showing a
                  // number for the three seconds before it is corrected.
                  if (Number.isFinite(next)) {
                    change({
                      [limit.key]: Math.min(Math.max(next, limit.min), limit.max),
                    });
                  }
                }}
                className="w-16 rounded-md border border-line bg-sunken px-1.5 py-0.5 text-right font-mono text-callout text-ink"
              />
              <span className="text-sub text-ink-3">{limit.unit}</span>
            </span>
          </label>
        ))}
      </fieldset>

      {settings.trigger === "agent-active" ? (
        <div className="flex flex-col gap-1.5">
          <p className="text-callout text-ink">Watching</p>
          <WatchList roots={status.roots} windowMinutes={settings.idle_window_minutes} />
        </div>
      ) : null}
    </section>
  );
}
