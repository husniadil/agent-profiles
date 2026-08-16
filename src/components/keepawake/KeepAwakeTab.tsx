import { AwakeStatusCard } from "@/components/keepawake/AwakeStatusCard";
import { WatchList } from "@/components/keepawake/WatchList";
import { Input, type InputClassNames } from "@/components/motion/input";
import type { KeepAwake } from "@/hooks/useKeepAwake";
import type { KeepAwakeSettings, Trigger } from "@/lib/api";

/// Only the options that are not their own explanation carry a line of prose.
/// "Off" saying "never hold the machine awake" is the label twice, and six
/// stacked descriptions is what turned this panel into documentation.
const TRIGGERS: { id: Trigger; label: string; detail?: string }[] = [
  { id: "off", label: "Off" },
  {
    id: "agent-active",
    label: "When an agent is working",
    detail: "A Claude Code or Codex session being written to.",
  },
  {
    id: "always",
    label: "Always while Agent Profiles runs",
    detail: "For agents inside a desktop app, where there is nothing to detect.",
  },
];

type LimitKey = keyof Omit<KeepAwakeSettings, "trigger">;

/// The three guards, side by side rather than stacked.
///
/// They are one idea — the conditions that end a hold — and the window is short
/// and wide, so reading them across costs a third of the height that reading
/// them down did. The bounds are restated here so the field cannot offer a
/// number the backend will silently clamp underneath it.
const LIMITS: { key: LimitKey; label: string; unit: string; min: number; max: number }[] = [
  { key: "idle_window_minutes", label: "Idle window", unit: "min", min: 1, max: 60 },
  { key: "battery_floor_percent", label: "Pause below", unit: "%", min: 0, max: 95 },
  { key: "max_hold_minutes", label: "Stop after", unit: "min", min: 5, max: 1440 },
];

/// The compose row's field, at the same height and type size, so a number here
/// is the same object as the name field on the other tab.
const FIELD: InputClassNames = {
  root: "gap-1",
  label: "text-sub font-normal text-ink-2",
  field: "h-7 rounded-lg",
  input: "px-2 font-mono text-callout",
};

/// Section headings, matching the compose card's on the other tab.
const LEGEND = "mb-1.5 text-sub font-semibold text-ink-2";

export function KeepAwakeTab({ keepAwake }: { keepAwake: KeepAwake }) {
  const status = keepAwake.status;
  if (!status) {
    // Blank rather than a spinner: the first read lands in a few milliseconds,
    // and a spinner that flashes for one frame is noise, not feedback.
    return <section id="panel-keep-awake" role="tabpanel" className="flex min-h-0 flex-1 flex-col p-2" />;
  }

  const { settings } = status;
  const change = (patch: Partial<KeepAwakeSettings>) =>
    void keepAwake.save({ ...settings, ...patch });
  const armed = status.supported && settings.trigger !== "off";

  return (
    // `gap-4` between sections against `gap-1.5` within them: the rhythm is what
    // separates three groups from one list of nine things. `p-2` matches the
    // profile panel, so the two tabs sit at one density.
    <section
      id="panel-keep-awake"
      role="tabpanel"
      className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-2"
    >
      <AwakeStatusCard status={status} keepAwake={keepAwake} />

      <fieldset disabled={!status.supported} className="disabled:opacity-50">
        <legend className={LEGEND}>Hold the machine awake</legend>
        <div className="flex flex-col gap-1.5">
          {TRIGGERS.map((trigger) => (
            <label key={trigger.id} className="flex items-start gap-2">
              <input
                type="radio"
                name="keep-awake-trigger"
                className="mt-px size-3.5 shrink-0 accent-accent"
                checked={settings.trigger === trigger.id}
                onChange={() => change({ trigger: trigger.id })}
              />
              <span className="min-w-0">
                <span className="text-callout text-ink">{trigger.label}</span>
                {trigger.detail ? (
                  <span className="block text-sub text-ink-2">{trigger.detail}</span>
                ) : null}
              </span>
            </label>
          ))}
        </div>
      </fieldset>

      <fieldset disabled={!armed} className="disabled:opacity-50">
        <legend className={LEGEND}>Limits</legend>
        {/* Three equal columns rather than three rows: `flex-1` on each keeps
            them even without a grid, and they wrap on a window narrowed to its
            floor instead of squeezing the inputs. */}
        <div className="flex flex-wrap items-end gap-2">
          {LIMITS.map((limit) => (
            <Input
              key={limit.key}
              type="number"
              min={limit.min}
              max={limit.max}
              label={limit.label}
              aria-label={`${limit.label} (${limit.unit})`}
              value={String(settings[limit.key])}
              onChange={(next) => {
                const parsed = Number(next);
                // The backend clamps too. This only stops the field showing a
                // number for the three seconds before it is corrected.
                if (next !== "" && Number.isFinite(parsed)) {
                  change({ [limit.key]: Math.min(Math.max(parsed, limit.min), limit.max) });
                }
              }}
              rightIcon={<span className="text-sub text-ink-3">{limit.unit}</span>}
              className="min-w-[120px] flex-1"
              classNames={FIELD}
            />
          ))}
        </div>
        <p className="mt-1.5 text-sub text-ink-2">
          The battery floor is ignored while plugged in. With the lid shut nothing can be reported
          to you, so the time limit runs out silently.
        </p>
      </fieldset>

      {/* Only once the feature can actually act. This list answers "why is it
          holding, or why isn't it" — and before authorization the answer is
          always "because you have not authorized", which the notice above
          already says. Showing live detection state there implies a feature
          that is working when it cannot. */}
      {status.authorized && settings.trigger === "agent-active" ? (
        <div>
          <p className={LEGEND}>Watching</p>
          <WatchList roots={status.roots} windowMinutes={settings.idle_window_minutes} />
        </div>
      ) : null}
    </section>
  );
}
