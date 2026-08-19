import { useEffect, useMemo, useRef, useState } from "react";

import { AwakeStatusCard } from "@/components/keepawake/AwakeStatusCard";
import { WatchList } from "@/components/keepawake/WatchList";
import { Input, type InputClassNames } from "@/components/motion/input";
import { RadioGroup, RadioGroupItem } from "@/components/motion/radio";
import { RangeSlider } from "@/components/motion/range-slider";
import { Switch } from "@/components/motion/switch";
import type { KeepAwake } from "@/hooks/useKeepAwake";
import type { KeepAwakeSettings, KeepAwakeStatus, Trigger } from "@/lib/api";
import { SWITCH } from "@/lib/controls";
import { useT, type T } from "@/lib/i18n";
import { systemNames } from "@/lib/system";

/// Only the options that are not their own explanation carry a line of prose.
/// "Off" saying "never hold the machine awake" is the label twice.
///
/// Built from `t` rather than declared at module scope: the labels are display
/// strings, and a module-level constant is evaluated once, before any locale is
/// known. The shape is unchanged — only the moment it is built.
function triggers(t: T): { id: Trigger; label: string; detail?: string }[] {
  return [
    { id: "off", label: t("awake.trigger.off") },
    {
      id: "agent-active",
      label: t("awake.trigger.agentActive"),
      detail: t("awake.trigger.agentActiveDetail"),
    },
    {
      id: "always",
      label: t("awake.trigger.always"),
      detail: t("awake.trigger.alwaysDetail"),
    },
  ];
}

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
/// This one used to be that question too: it decided when an agent counted as
/// idle, which meant every finished session held the machine for its whole
/// length. The transcript answers that exactly now, so all this bounds is a
/// session that stopped part-way through a turn and will never write again.
///
/// The bounds are restated here so the field cannot offer a number the backend
/// will clamp underneath it.
function limits(t: T): {
  key: LimitKey;
  label: string;
  unit: string;
  min: number;
  max: number;
}[] {
  return [
    {
      // Not "idle window", and no longer "agent idle after" either: neither named
      // what this decides. A reader who has to ask what a setting does has been
      // failed by its label, and this one now buys exactly one thing — the moment
      // we stop waiting on an agent that went quiet mid-task.
      key: "idle_window_minutes",
      label: t("awake.limit.idleWindow"),
      unit: t("awake.limit.minutes"),
      min: 1,
      max: 60,
    },
  ];
}

/// The battery floor's range, in whole steps of five.
///
/// A slider rather than a field because this is the guard with a physical
/// consequence: the number means "how much charge I am willing to spend before
/// my machine is allowed to sleep again", and that is a quantity people set by feel
/// against the charge they have, not by typing a figure. Capped below 100 so the
/// setting cannot be raised to a level that would drop every hold instantly.
/// Steps of ten, and a ceiling that is one of them. At a step of five the
/// ninety-five on the end was not a stop anyone aimed at — it existed only
/// because the range did not divide — and twenty ticks on a half-width track
/// are a texture rather than a scale.
const FLOOR = { min: 0, max: 90, step: 10 };

/// beUI's track is a 40px block with a 20px handle, sized for a page. Every
/// control in this window is 28px, so the track takes the field height and the
/// handle is brought back into proportion with it. Nothing else is touched:
/// the ticks are what make a stepped value legible without a scale under it,
/// and the neutral fill is right — accent means "primary action or current
/// selection" here, and a threshold is neither.
/// Half the row and hard against its right edge, so this setting is the same
/// object as the two under it: the sentence on the left, the control answering
/// on the right, on one line. On its own line below the name it was the only one
/// of the three shaped differently, and at half width it left a hole beside
/// itself.
///
/// 16px, which is the switch's height exactly and one pixel under the line box
/// of the text it sits in. At 24px the track set the row's height and this one
/// setting stood a third taller than its neighbours; now nothing in the right
/// column is taller than a line of type except the field, which has to be.
///
/// It keeps the default `shrink`, so a narrow window takes it out of the track
/// rather than out of the sentence beside it.
const SLIDER = "h-4 w-1/2 [&_[role=slider]]:h-3";

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
///
/// The number is right-aligned so it lands against the unit and the two read as
/// one quantity — "5 min", not a 5 at one end of the box and a "min" at the
/// other. No `label` key any more: the name is a sibling of the field now, not
/// a thing the field draws above itself.
const FIELD: InputClassNames = {
  field: "h-7 rounded-lg",
  input:
    "pl-2 pr-8 text-right font-mono text-sub [appearance:textfield] " +
    "[&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none",
  rightIcon: "pr-2",
};

/// Section headings, matching the compose card's on the other tab.
const LEGEND = "text-sub font-semibold text-ink-2";

/// One shape for every setting under Limits: the name on the left, the thing
/// that sets it on the right, and the sentence explaining it underneath at full
/// width. Three settings used to have three different geometries — a name after
/// a 27px glyph, a name floating above its own field, a name with the prose
/// tucked into a narrow column beside the control — so nothing down the left
/// edge lined up and no two rows were read the same way twice.
///
/// The space does the grouping: 4px from a name to what it explains, 12px from
/// one setting to the next. It was 6 and 10 before, and a ratio that close
/// reads as one undifferentiated stack rather than three settings.
const SETTING = "flex flex-col gap-1";
const SETTING_ROW = "flex items-center justify-between gap-3";
/// Every setting's name at one size and one weight — including the middle one,
/// which was set in the helper text's own 11px grey and so came second to the
/// sentence underneath it.
const NAME = "text-callout text-ink";
const HINT = "text-sub text-ink-2";

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
  limit: ReturnType<typeof limits>[number];
  value: number;
  onCommit: (next: number) => void;
}) {
  const t = useT();
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
      id={limit.key}
      type="number"
      min={limit.min}
      max={limit.max}
      // The visible name is the row's, not the field's. This still carries the
      // unit, which the row cannot say without repeating what is inside the box.
      aria-label={t("awake.limit.aria", { label: limit.label, unit: limit.unit })}
      value={draft}
      onChange={setDraft}
      onBlur={commit}
      // Enter commits without needing the field to lose focus first.
      onKeyDown={(event) => {
        if (event.key === "Enter") event.currentTarget.blur();
      }}
      rightIcon={<span className="text-sub text-ink-3">{limit.unit}</span>}
      // Sized to what it holds: two digits and a unit. At 160px it was a
      // one-character answer in a box four times too big for it, with the
      // dead space landing exactly where the rows below right-align.
      className="w-20 shrink-0"
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
  // Authorize is the first gate. Until the machine can actually hold — a
  // platform that needs a password and has not been given one — the trigger and
  // its limits stay locked, so the only thing to act on is the Authorize button
  // in the band above. Where no password is needed (Linux, Windows) this is
  // always false and nothing is locked.
  const pendingAuth = status.needs_authorization && !status.authorized;
  const armed = status.supported && !pendingAuth && settings.trigger !== "off";
  // "this Mac" / "this PC" / "this computer". The tab is about the reader's own
  // hardware, and this feature now runs on all three.
  const t = useT();
  const { machine } = systemNames(t);
  const Machine = machine[0].toUpperCase() + machine.slice(1);
  const triggerList = useMemo(() => triggers(t), [t]);
  const limitList = useMemo(() => limits(t), [t]);

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
            disabled={!status.supported || pendingAuth}
            className="disabled:opacity-50"
          >
            <legend className={`${LEGEND} mb-1.5`}>
              {t("awake.section.hold")}
            </legend>
            <RadioGroup
              value={settings.trigger}
              onValueChange={(next) => change({ trigger: next as Trigger })}
              className="gap-1.5"
            >
              {triggerList.map((trigger) => (
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
            <legend className={`${LEGEND} mb-2`}>{t("awake.section.limits")}</legend>

            <div className="flex flex-col gap-3">
              {/* The battery floor leads: it is the guard people actually
                  reason about, and the only one whose value is meaningful
                  against something the app already knows. */}
              <div className={SETTING}>
                {/* The figure sits against the name, not against the track, and
                    `ml-auto` on the track is what holds the right edge. The two
                    together are one sentence — "Pause on low battery below 50%"
                    — so a 6px gap binds them and the control stands apart, which
                    is the shape the other two settings already have.

                    A `<label for>` no longer: beUI's handle is a div carrying
                    `role="slider"`, which nothing can be labelled for. The name
                    is given to the control itself, and this line is the picture
                    of it. */}
                <div className="flex items-center gap-1.5">
                  {/* `whitespace-nowrap` only here, and only because this row
                      holds three things where the others hold two. Narrowed to
                      the smallest window this can be dragged to, the sentence
                      broke across two lines while the track kept its half; the
                      track is the part with nothing to lose, so it gives. Not
                      on `NAME` itself — a label long enough to need wrapping
                      should wrap rather than run out of its row. */}
                  <span className={`${NAME} whitespace-nowrap`}>
                    {t("awake.battery.name")}
                  </span>
                  {/* The battery glyph used to ride here. It said the threshold
                      a third time — the track already draws it and the figure
                      already states it — and in the right-hand column it was a
                      14px rounded shell filled from the left, three rows above
                      a 16px switch with its thumb to the left. Two unrelated
                      controls with one silhouette in one column: the glyph read
                      as a toggle that was off.

                      "below 30%" rather than "30%": with the threshold no
                      longer named in the label, a bare figure could just as
                      easily be read as the charge right now. Said the same way
                      to a screen reader, through `aria-valuetext`. */}
                  {/* Sans, at the name's own size. The window sets figures in
                      mono where they stand in a column of figures — the watch
                      list's ages, the byte counts, the paths — and there the
                      face is doing work, holding a rail of numbers in line.
                      Here the figure is not in a column; it is the second half
                      of a sentence, and a monospaced word inside a sentence is
                      just a different typeface mid-phrase.

                      `tabular-nums` survives the change and is the only part
                      that has to: the figure is dragged, and proportional
                      digits would shift the whole line under the hand. */}
                  <output className="shrink-0 text-callout tabular-nums text-ink">
                    {t("awake.battery.below", { percent: floor })}
                  </output>
                  <RangeSlider
                    aria-label={t("awake.battery.aria")}
                    formatValueText={(value) =>
                      t("awake.battery.below", { percent: value })
                    }
                    min={FLOOR.min}
                    max={FLOOR.max}
                    step={FLOOR.step}
                    value={floor}
                    // A drag reports every step it crosses. Committing straight
                    // from here wrote the settings file and made an IPC round
                    // trip twenty times per sweep of the track; the draft moves
                    // the handle at once and `useCommitted` saves when the hand
                    // stops.
                    onValueChange={setFloor}
                    // The enclosing `fieldset[disabled]` cannot reach this one
                    // — the handle is a div, and only form controls inherit it.
                    disabled={!armed}
                    className={`ml-auto ${SLIDER}`}
                  />
                </div>
                {/* The charge right now is deliberately not repeated here — the
                    status band at the top of this card already reports it, and
                    saying it twice in one card is how the panel filled up. */}
                <p className={HINT}>
                  {status.battery_percent === null
                    ? t("awake.hint.noBattery", { machine: Machine })
                    : t("awake.hint.lowBattery")}
                </p>
              </div>

              {/* Typed, not dragged: minutes are exact quantities with no
                  comfortable feel to them, and a track that resolved a range
                  this wide would be answering by accident. */}
              {limitList.map((limit) => (
                <div key={limit.key} className={SETTING}>
                  <div className={SETTING_ROW}>
                    <label htmlFor={limit.key} className={NAME}>
                      {limit.label}
                    </label>
                    <LimitField
                      limit={limit}
                      value={settings[limit.key]}
                      onCommit={(next) => change({ [limit.key]: next })}
                    />
                  </div>
                  {/* Says what the number buys, because it is no longer the
                      obvious thing. It used to define idleness; a reader who
                      still assumes that would set it low to stop the machine
                      being held after their agent finished — a problem it no
                      longer has, at the price of the one failure it exists to
                      prevent. */}
                  <p className={HINT}>
                    {t("awake.hint.idleWindow", { machine })}
                  </p>
                </div>
              ))}

              {/* Left out entirely where the machine has no temperature to
                  report — Windows, and a Linux box with no thermal zones. Not
                  disabled: a greyed switch reads as "turn something on first",
                  and a switch that says "this never applies" is still a switch
                  someone can leave on and believe they are protected. The one
                  honest shape for a guard that cannot fire is absence.

                  Not a third number beside the others either: this is not a
                  threshold anyone tunes, it is whether a guard is armed. */}
              {status.thermal_supported ? (
                <div className={SETTING}>
                  <div className={SETTING_ROW}>
                    <span className={NAME}>{t("awake.thermal.name")}</span>
                    {/* The same switch the window already uses for "Start at
                        login", named from here rather than by a `<label for>`:
                        the sentence belongs to the setting, not the control. */}
                    <Switch
                      className={`shrink-0 ${SWITCH}`}
                      checked={settings.thermal_guard}
                      disabled={!armed}
                      onCheckedChange={(next) => change({ thermal_guard: next })}
                      ariaLabel={t("awake.thermal.aria")}
                    />
                  </div>
                  {/* Full width like the other two, rather than wrapped into a
                      narrow column beside the switch. */}
                  <p className={HINT}>{t("awake.hint.thermal")}</p>
                </div>
              ) : null}
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
          <p className={`${LEGEND} mb-1.5`}>{t("awake.section.watching")}</p>
          <WatchList
            roots={status.roots}
            windowMinutes={settings.idle_window_minutes}
          />
        </div>
      ) : null}
    </section>
  );
}
