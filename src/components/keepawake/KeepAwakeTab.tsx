import { AwakeStatusCard } from "@/components/keepawake/AwakeStatusCard";
import { WatchList } from "@/components/keepawake/WatchList";
import { Input, type InputClassNames } from "@/components/motion/input";
import type { KeepAwake } from "@/hooks/useKeepAwake";
import type { KeepAwakeSettings, Trigger } from "@/lib/api";

/// Only the options that are not their own explanation carry a line of prose.
/// "Off" saying "never hold the machine awake" is the label twice.
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
    detail:
      "For agents inside a desktop app, where there is nothing to detect.",
  },
];

type LimitKey = keyof Omit<KeepAwakeSettings, "trigger">;

/// The three guards, side by side rather than stacked. They are one idea — the
/// conditions that end a hold — and the window is short and wide. The bounds are
/// restated here so the field cannot offer a number the backend will clamp
/// underneath it.
const LIMITS: {
  key: LimitKey;
  label: string;
  unit: string;
  min: number;
  max: number;
}[] = [
  {
    key: "idle_window_minutes",
    label: "Idle window",
    unit: "min",
    min: 1,
    max: 60,
  },
  {
    key: "max_hold_minutes",
    label: "Stop after",
    unit: "min",
    min: 5,
    max: 1440,
  },
];

/// The battery floor's range, in whole steps of five.
///
/// A slider rather than a field because this is the guard with a physical
/// consequence: the number means "how much charge I am willing to spend before
/// my Mac is allowed to sleep again", and that is a quantity people set by feel
/// against the charge they have, not by typing a figure. Capped below 100 so the
/// setting cannot be raised to a level that would drop every hold instantly.
const FLOOR = { min: 0, max: 95, step: 5 };

/// Native `input[type=range]`, styled rather than replaced: it arrives with
/// keyboard stepping, page-up/down, home/end and a screen-reader value already
/// working, and a hand-built track would have to earn all of that back.
const SLIDER =
  "h-1.5 w-full cursor-pointer appearance-none rounded-full outline-none " +
  "focus-visible:ring-2 focus-visible:ring-accent/40 " +
  "[&::-webkit-slider-thumb]:size-3.5 [&::-webkit-slider-thumb]:appearance-none " +
  "[&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-accent " +
  "[&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-surface " +
  "[&::-webkit-slider-thumb]:shadow-sm " +
  "disabled:cursor-not-allowed";

/// The compose row's field, at the same height and type size.
///
/// Two things the default gets wrong for a field with a unit after it. beUI
/// reserves `pr-10` for its right slot, so the padding here has to be set per
/// side — `px-2` silently took that reservation back and ran the number under
/// the unit. And a number input's spinner sits in exactly that space: WebKit
/// draws it permanently, so in the shipping app it landed on top of the unit
/// even though Chromium only reveals it on hover.
const FIELD: InputClassNames = {
  root: "gap-1",
  label: "text-sub font-normal text-ink-2",
  field: "h-7 rounded-lg",
  input:
    "pl-2 pr-8 font-mono text-callout [appearance:textfield] " +
    "[&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none",
  rightIcon: "pr-2",
};

/// Section headings, matching the compose card's on the other tab.
const LEGEND = "text-sub font-semibold text-ink-2";

/// The bordered shell the compose card uses, so a container on this tab is the
/// same object as a container on the other one.
const CARD =
  "shrink-0 rounded-xl border border-hairline bg-surface shadow-card";
/// Bands inside one card, divided rather than boxed: a card inside a card is
/// always wrong, and a hairline says "different group" just as clearly.
///
/// The rule goes on the band, never on a `fieldset` directly. A fieldset cuts a
/// notch in its own border to seat the legend, so a `border-t` there draws the
/// line straight through the heading instead of above it.
const BAND = "p-2.5";
const DIVIDED = "border-t border-hairline";

export function KeepAwakeTab({ keepAwake }: { keepAwake: KeepAwake }) {
  const status = keepAwake.status;
  if (!status) {
    // Blank rather than a spinner: the first read lands in a few milliseconds,
    // and a spinner that flashes for one frame is noise, not feedback.
    return (
      <section
        id="panel-keep-awake"
        role="tabpanel"
        className="flex min-h-0 flex-1 flex-col p-2"
      />
    );
  }

  const { settings } = status;
  const change = (patch: Partial<KeepAwakeSettings>) =>
    void keepAwake.save({ ...settings, ...patch });
  const armed = status.supported && settings.trigger !== "off";

  return (
    // `p-2 gap-2` exactly as the profile panel: two tabs, one density, one inset.
    <section
      id="panel-keep-awake"
      role="tabpanel"
      className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2"
    >
      <div className={CARD}>
        <div className={BAND}>
          <AwakeStatusCard status={status} keepAwake={keepAwake} />
        </div>

        <div className={`${BAND} ${DIVIDED}`}>
          <fieldset
            disabled={!status.supported}
            className="disabled:opacity-50"
          >
            <legend className={`${LEGEND} mb-1.5`}>
              Hold the machine awake
            </legend>
            <div className="flex flex-col gap-1.5">
              {TRIGGERS.map((trigger) => (
                <label
                  key={trigger.id}
                  className="flex cursor-pointer items-start gap-2"
                >
                  <input
                    type="radio"
                    name="keep-awake-trigger"
                    // `mt-0.5` sits the control on the label's cap height rather
                    // than its box top, which is where it looked pushed up.
                    className="mt-0.5 size-3.5 shrink-0 accent-accent"
                    checked={settings.trigger === trigger.id}
                    onChange={() => change({ trigger: trigger.id })}
                  />
                  <span className="min-w-0">
                    <span className="text-callout text-ink">
                      {trigger.label}
                    </span>
                    {trigger.detail ? (
                      <span className="block text-sub text-ink-2">
                        {trigger.detail}
                      </span>
                    ) : null}
                  </span>
                </label>
              ))}
            </div>
          </fieldset>
        </div>

        <div className={`${BAND} ${DIVIDED}`}>
          <fieldset disabled={!armed} className="disabled:opacity-50">
            <legend className={`${LEGEND} mb-1.5`}>Limits</legend>

            {/* The battery floor leads the group and gets the width. It is the
              guard people actually reason about, and the only one whose value
              is meaningful against something the app already knows — the charge
              right now, named in the sentence under it. */}
            <div className="mb-2.5">
              <div className="flex items-baseline justify-between gap-3">
                <label
                  htmlFor="battery-floor"
                  className="text-callout text-ink"
                >
                  Pause below
                </label>
                <output
                  htmlFor="battery-floor"
                  className="font-mono text-callout tabular-nums text-ink"
                >
                  {settings.battery_floor_percent}%
                </output>
              </div>
              <input
                id="battery-floor"
                type="range"
                min={FLOOR.min}
                max={FLOOR.max}
                step={FLOOR.step}
                value={settings.battery_floor_percent}
                onChange={(event) =>
                  change({ battery_floor_percent: Number(event.target.value) })
                }
                // The filled half is painted as a gradient stop rather than with a
                // second element: one box, no overlay to keep in sync with the
                // thumb, and it survives the track being restyled.
                style={{
                  background: `linear-gradient(to right, var(--accent) ${
                    (settings.battery_floor_percent / FLOOR.max) * 100
                  }%, var(--sunken) ${(settings.battery_floor_percent / FLOOR.max) * 100}%)`,
                }}
                className={`mt-1.5 ${SLIDER}`}
              />
              {/* The charge right now is deliberately not repeated here — the
                  status band at the top of this card already reports it, and
                  saying it twice in one card is how the panel filled up. */}
              <p className="mt-1.5 text-sub text-ink-2">
                {status.battery_percent === null
                  ? "This Mac has no battery, so this never applies."
                  : "Dropped below this charge, even mid-task. Ignored while plugged in."}
              </p>
            </div>

            {/* The two durations are typed, not dragged: minutes are exact
              quantities with no comfortable feel to them, and "stop after" runs
              to a full day, which no usable track can resolve. */}
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
                      change({
                        [limit.key]: Math.min(
                          Math.max(parsed, limit.min),
                          limit.max,
                        ),
                      });
                    }
                  }}
                  rightIcon={
                    <span className="text-sub text-ink-3">{limit.unit}</span>
                  }
                  className="min-w-[120px] flex-1"
                  classNames={FIELD}
                />
              ))}
            </div>
            <p className="mt-1.5 text-sub text-ink-2">
              With the lid shut nothing can be reported to you, so the time
              limit runs out silently.
            </p>
          </fieldset>
        </div>
      </div>

      {/* Only once the feature can actually act. This list answers "why is it
          holding, or why isn't it" — and before authorization the answer is
          always "because you have not authorized", which the band above already
          says. Live detection state there implies a feature that is working
          when it cannot. */}
      {status.authorized && settings.trigger === "agent-active" ? (
        <div className={`${CARD} ${BAND}`}>
          <p className={`${LEGEND} mb-1.5`}>Watching</p>
          <WatchList
            roots={status.roots}
            windowMinutes={settings.idle_window_minutes}
          />
        </div>
      ) : null}
    </section>
  );
}
