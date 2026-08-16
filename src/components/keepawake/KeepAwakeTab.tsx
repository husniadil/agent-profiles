import { useEffect, useRef, useState } from "react";

import { AwakeStatusCard } from "@/components/keepawake/AwakeStatusCard";
import { BatteryGauge } from "@/components/keepawake/BatteryGauge";
import { WatchList } from "@/components/keepawake/WatchList";
import { Input, type InputClassNames } from "@/components/motion/input";
import { RadioGroup, RadioGroupItem } from "@/components/motion/radio";
import { RangeSlider } from "@/components/motion/range-slider";
import { Switch } from "@/components/motion/switch";
import type { KeepAwake } from "@/hooks/useKeepAwake";
import type { KeepAwakeSettings, KeepAwakeStatus, Trigger } from "@/lib/api";

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

/// Only the settings that are actually numbers. Derived rather than listed, so
/// a new numeric setting joins automatically — and a new boolean one, like the
/// thermal guard, cannot silently land in a row built to hold a figure.
type LimitKey = {
  [K in keyof KeepAwakeSettings]: KeepAwakeSettings[K] extends number ? K : never;
}[keyof KeepAwakeSettings];

/// The one typed limit left. "Stop after" used to sit beside it, capping a hold
/// on a clock — but that existed only to stand in for a temperature nobody could
/// read, and the thermal guard measures the real thing now. A Keep Awake feature
/// that stops because time passed is answering a question nobody asked.
///
/// The bounds are restated here so the field cannot offer a number the backend
/// will clamp underneath it.
const LIMITS: {
  key: LimitKey;
  label: string;
  unit: string;
  min: number;
  max: number;
}[] = [
  {
    // Not "idle window": that named the mechanism rather than the decision, and
    // a reader who has to ask what a setting does has been failed by its label.
    // This one says whose state it is, what state, and when.
    key: "idle_window_minutes",
    label: "Agent idle after",
    unit: "min",
    min: 1,
    max: 60,
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

/// beUI's track is a 40px block with a 20px handle, sized for a page. Every
/// control in this window is 28px, so the track takes the field height and the
/// handle is brought back into proportion with it. Nothing else is touched:
/// the ticks are what make a stepped value legible without a scale under it,
/// and the neutral fill is right — accent means "primary action or current
/// selection" here, and a threshold is neither.
const SLIDER = "h-7 [&_[role=slider]]:h-4";

/// Same story as the slider: beUI's dot is a 20px circle with a 2px ring at
/// 14px type, which is a page's radio. This window's is 14px at 12px type with
/// a hairline ring, so the three overrides reach past the label the class lands
/// on. Descendant selectors rather than a fork of the component — one class
/// beats one, two beat one, and the vendored file stays re-syncable.
const RADIO =
  "gap-2 [&>button]:size-3.5 [&>button]:border [&>span]:text-callout";

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

/// A value the control owns while it is being moved, saved once it settles.
///
/// Every setting here writes a file and crosses the IPC boundary, and both a
/// dragged slider and a typed field produce a burst of intermediate values that
/// are not choices — they are the motion between two choices. Saving each one
/// wrote the settings file twenty times per drag and let the backend's clamp
/// rewrite a half-typed number under the caret.
///
/// The draft follows the committed value whenever that changes from elsewhere —
/// the three-second status poll, or the backend clamping what was sent — so this
/// never becomes a second source of truth.
function useCommitted<T>(
  committed: T,
  save: (value: T) => void,
  delayMs = 250,
) {
  const [draft, setDraft] = useState(committed);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const latest = useRef(save);
  latest.current = save;

  useEffect(() => setDraft(committed), [committed]);
  useEffect(
    () => () => (timer.current ? clearTimeout(timer.current) : undefined),
    [],
  );

  return [
    draft,
    (next: T) => {
      setDraft(next);
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => latest.current(next), delayMs);
    },
  ] as const;
}

/// One typed limit. Its own component so the draft lives per field rather than
/// in one shared object, and so blur can commit without the other field's value
/// riding along on a stale copy.
function LimitField({
  limit,
  value,
  onCommit,
}: {
  limit: (typeof LIMITS)[number];
  value: number;
  onCommit: (next: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));
  // Follows the committed value while the field is not being edited, so a
  // number the backend clamped shows up here rather than being contradicted.
  useEffect(() => setDraft(String(value)), [value]);

  // Clamped here, not on every keystroke. Typing "10" into a field whose floor
  // is 5 passes through "1", and clamping that turned the first digit into a 5
  // under the caret — the value fought the hand holding it.
  const commit = () => {
    const parsed = Number(draft);
    const next = Number.isFinite(parsed)
      ? Math.min(Math.max(Math.round(parsed), limit.min), limit.max)
      : value;
    setDraft(String(next));
    if (next !== value) onCommit(next);
  };

  return (
    <Input
      type="number"
      min={limit.min}
      max={limit.max}
      label={limit.label}
      aria-label={`${limit.label} (${limit.unit})`}
      value={draft}
      onChange={setDraft}
      onBlur={commit}
      // Enter commits without needing the field to lose focus first.
      onKeyDown={(event) => {
        if (event.key === "Enter") event.currentTarget.blur();
      }}
      rightIcon={<span className="text-sub text-ink-3">{limit.unit}</span>}
      // Sized to its content, not stretched. It shared a row with "Stop after"
      // and split the width with it; alone, `flex-1` gave a two-digit number a
      // field five hundred pixels wide.
      className="w-40"
      classNames={FIELD}
    />
  );
}

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

/// Splits on the first read so the panel below can hold hooks.
///
/// The draft state the slider and the fields need cannot live behind an early
/// return — hooks have to run in the same order every render — so the "no status
/// yet" case is a separate component rather than a branch inside one.
export function KeepAwakeTab({ keepAwake }: { keepAwake: KeepAwake }) {
  if (!keepAwake.status) {
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
  return <KeepAwakePanel keepAwake={keepAwake} status={keepAwake.status} />;
}

function KeepAwakePanel({
  keepAwake,
  status,
}: {
  keepAwake: KeepAwake;
  status: KeepAwakeStatus;
}) {
  const { settings } = status;
  const change = (patch: Partial<KeepAwakeSettings>) =>
    void keepAwake.save({ ...settings, ...patch });
  const armed = status.supported && settings.trigger !== "off";

  const [floor, setFloor] = useCommitted(
    settings.battery_floor_percent,
    (next) => change({ battery_floor_percent: next }),
  );

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
            <RadioGroup
              value={settings.trigger}
              onValueChange={(next) => change({ trigger: next as Trigger })}
              className="gap-1.5"
            >
              {TRIGGERS.map((trigger) => (
                // The second line is not a label prop — beUI's item takes a
                // string — so it sits beside the item and is indented past the
                // control by hand, to the width of the dot plus its gap.
                <div key={trigger.id} className="flex flex-col">
                  <RadioGroupItem
                    value={trigger.id}
                    label={trigger.label}
                    className={RADIO}
                  />
                  {trigger.detail ? (
                    <span className="pl-[22px] text-sub text-ink-2">
                      {trigger.detail}
                    </span>
                  ) : null}
                </div>
              ))}
            </RadioGroup>
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
              {/* A `<label for>` no longer: beUI's handle is a div carrying
                  `role="slider"`, which nothing can be labelled for. The name
                  is given to the control itself, and this line is the picture
                  of it. */}
              <div className="flex items-baseline justify-between gap-3">
                <span className="flex items-center gap-1.5 text-callout text-ink">
                  {/* The glyph tracks the threshold, not the charge: this row
                      sets a level, and a picture that showed something else
                      while sitting on the control would be answering a question
                      nobody asked here. The current charge is in the band at the
                      top of this card. */}
                  <BatteryGauge percent={floor} className="h-3.5 w-[27px] shrink-0 text-ink-2" />
                  Pause on low battery
                </span>
                {/* "below 30%" rather than "30%": with the threshold no longer
                    named in the label, a bare figure beside it could just as
                    easily be read as the charge right now. Said the same way to
                    a screen reader, through the slider's `aria-valuetext`. */}
                <output className="font-mono text-callout tabular-nums text-ink">
                  below {floor}%
                </output>
              </div>
              <RangeSlider
                aria-label="Pause on low battery"
                formatValueText={(value) => `below ${value}%`}
                min={FLOOR.min}
                max={FLOOR.max}
                step={FLOOR.step}
                value={floor}
                // A drag reports every step it crosses. Committing straight
                // from here wrote the settings file and made an IPC round trip
                // twenty times per sweep of the track; the draft moves the
                // handle at once and `useCommitted` saves when the hand stops.
                onValueChange={setFloor}
                // The enclosing `fieldset[disabled]` cannot reach this one —
                // the handle is a div, and only form controls inherit that.
                disabled={!armed}
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
                <LimitField
                  key={limit.key}
                  limit={limit}
                  value={settings[limit.key]}
                  onCommit={(next) => change({ [limit.key]: next })}
                />
              ))}
            </div>
            {/* One sentence per field, in the order the fields appear. The
                first exists because "idle" is a judgement this app makes on the
                user's behalf, and a setting that decides when your work counts
                as finished has to say so out loud. */}
            <p className="mt-1.5 text-sub text-ink-2">
              An agent counts as finished once its session has gone this long
              without being written to.
            </p>

            {/* Its own row rather than a third number beside the others: this
                one is not a threshold anyone tunes, it is whether a guard is
                armed at all. */}
            <div className="mt-2.5 flex items-center justify-between gap-3">
              <span className="min-w-0">
                <span className="text-callout text-ink">Thermal guard</span>
                <span className="block text-sub text-ink-2">
                  {status.thermal === "unknown"
                    ? "This Mac reports no thermal state, so this never applies."
                    : "Release the hold when macOS reports the machine is overheating."}
                </span>
              </span>
              {/* The same switch the window already uses for "Start at login",
                  named here rather than by a `<label for>`: the sentence to the
                  left runs to two lines and only the first of them is the
                  control's name. */}
              <Switch
                className="shrink-0"
                checked={settings.thermal_guard}
                disabled={!armed}
                onCheckedChange={(next) => change({ thermal_guard: next })}
                ariaLabel="Thermal guard"
              />
            </div>
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
