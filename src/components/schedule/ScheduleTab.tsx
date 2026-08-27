import { useEffect, useRef, useState } from "react";
import { AppWindow, CalendarClock } from "lucide-react";

import {
  AvailabilityScheduler,
  type DayKey,
  type WeekAvailability,
} from "@/components/motion/availability-scheduler";
import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
  ComboboxTrigger,
} from "@/components/motion/combobox";
import { Switch } from "@/components/motion/switch";
import type { Schedule } from "@/hooks/useSchedule";
import * as api from "@/lib/api";
import type { DayWake, InstalledApp, ScheduleSettings } from "@/lib/api";
import { SWITCH } from "@/lib/controls";
import { useT } from "@/lib/i18n";

/// The scheduler's day keys in our Monday-first order, so the index into this
/// array *is* the backend's `weekday` (0 = Monday … 6 = Sunday).
const DAY_ORDER: DayKey[] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];


/// The time a newly-enabled day wakes at, and the end the scheduler needs.
///
/// The block edits time *ranges*; a wake is a single instant, so only `start` is
/// ever read back (see `fromWeek`). `end` is carried so the component's own
/// range invariant holds, and nothing downstream looks at it.
const DEFAULT_START = "09:00";
const DEFAULT_END = "17:00";

/// Backend days → the block's week shape. Ids are derived from the day key so
/// they stay stable across re-renders and the rows never remount.
function toWeek(days: DayWake[]): WeekAvailability {
  const week = {} as WeekAvailability;
  DAY_ORDER.forEach((key, weekday) => {
    const found = days.find((d) => d.weekday === weekday);
    week[key] = {
      enabled: found !== undefined,
      ranges: [
        {
          id: `${key}-0`,
          start: found?.time ?? DEFAULT_START,
          end: DEFAULT_END,
        },
      ],
    };
  });
  return week;
}

/// The block's week → backend days. An enabled day contributes its first range's
/// start; everything else the block can express (a second range, an end time) has
/// no meaning for a wake and is dropped.
function fromWeek(week: WeekAvailability): DayWake[] {
  const days: DayWake[] = [];
  DAY_ORDER.forEach((key, weekday) => {
    const day = week[key];
    if (day.enabled && day.ranges.length > 0) {
      days.push({ weekday, time: day.ranges[0].start });
    }
  });
  return days;
}

/// The app name a saved path stands for when it is no longer installed.
function appNameFromPath(path: string): string {
  return (path.split("/").pop() ?? path).replace(/\.app$/, "");
}

export function ScheduleTab({ schedule }: { schedule: Schedule }) {
  const t = useT();
  const { status } = schedule;
  const [apps, setApps] = useState<InstalledApp[]>([]);

  // The installed-app list is read once the feature is known to be supported.
  const supported = status?.supported ?? false;
  useEffect(() => {
    if (!supported) return;
    let live = true;
    api
      .listApplications()
      .then((list) => live && setApps(list))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [supported]);

  // A local editing copy, so a change lands instantly instead of waiting for the
  // save to round-trip and snap the control back. Re-seeded from the backend only
  // when it reports a day set we did not just send — every hook sits above the
  // `!status` early return so the hook order never changes between renders.
  const serverDays = status?.settings.days ?? [];
  const serverSig = JSON.stringify(serverDays);
  const [days, setDays] = useState<DayWake[]>(serverDays);
  const sentSig = useRef(serverSig);
  useEffect(() => {
    if (serverSig !== sentSig.current) {
      setDays(JSON.parse(serverSig) as DayWake[]);
      sentSig.current = serverSig;
    }
  }, [serverSig]);

  if (!status) {
    return (
      <section
        id="panel-schedule"
        role="tabpanel"
        className="flex min-h-0 flex-1 flex-col gap-2 p-2"
      />
    );
  }

  const s = status.settings;

  const write = (patch: Partial<ScheduleSettings>) => {
    void schedule.save({ ...s, ...patch });
  };

  const onWeekChange = (week: WeekAvailability) => {
    const next = fromWeek(week);
    setDays(next);
    sentSig.current = JSON.stringify(next);
    write({ days: next });
  };

  const selectedName = s.app_path
    ? (apps.find((a) => a.path === s.app_path)?.name ??
      appNameFromPath(s.app_path))
    : "";
  const selectedIcon = apps.find((a) => a.path === s.app_path)?.icon ?? null;

  return (
    <section
      id="panel-schedule"
      role="tabpanel"
      className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2"
    >
      {!status.supported ? (
        <div className="flex gap-1.5">
          <span className="flex w-3.5 shrink-0 justify-center text-ink-2">
            <CalendarClock size={13} aria-hidden="true" className="mt-0.5 shrink-0" />
          </span>
          <div className="min-w-0 flex-1">
            <p className="text-callout font-medium text-ink-2">
              {t("schedule.band.unavailable")}
            </p>
            <p className="mt-0.5 text-sub text-ink-2">
              {status.refusal ?? t("schedule.unsupported.generic")}
            </p>
          </div>
        </div>
      ) : (
        <>
          <div className="flex items-center justify-between gap-2 rounded-lg border border-hairline p-2.5">
            <span className="text-callout font-medium">{t("schedule.enable.name")}</span>
            <Switch
              className={`shrink-0 ${SWITCH}`}
              checked={s.enabled}
              disabled={schedule.busy}
              onCheckedChange={(next) => write({ enabled: next })}
              ariaLabel={t("schedule.enable.name")}
            />
          </div>

          {/* Editable whether or not the master switch is on: set the days, times
              and app first (this only persists — no wake is installed and no
              password is asked while the switch is off), then flip it on once. */}
          <div className="rounded-lg border border-hairline p-2.5">
            <p className="mb-1 text-callout font-medium">
              {t("schedule.days.legend")}
            </p>
            <AvailabilityScheduler
              value={toWeek(days)}
              onChange={onWeekChange}
              step={10}
              className="max-w-none"
            />
          </div>

          <div className="flex items-center justify-between gap-2 rounded-lg border border-hairline p-2.5">
            <span className="shrink-0 text-callout font-medium">
              {t("schedule.app.name")}
            </span>
            <Combobox
              value={s.app_path || undefined}
              onValueChange={(next) => write({ app_path: next })}
              className="w-56 shrink-0"
            >
              <ComboboxTrigger className="h-8 min-w-0 rounded-lg px-2.5">
                <span className="flex min-w-0 items-center gap-2">
                  {selectedIcon ? (
                    <img
                      src={selectedIcon}
                      alt=""
                      className="size-4 shrink-0 rounded"
                    />
                  ) : s.app_path ? (
                    <AppWindow className="size-4 shrink-0 text-ink-3" aria-hidden />
                  ) : null}
                  <ComboboxInput
                    placeholder={selectedName || t("schedule.app.placeholder")}
                    aria-label={t("schedule.app.name")}
                    className="min-w-0 flex-1 text-callout"
                  />
                </span>
              </ComboboxTrigger>
              <ComboboxContent>
                <ComboboxList>
                  <ComboboxEmpty>{t("schedule.app.empty")}</ComboboxEmpty>
                  {apps.map((app) => (
                    <ComboboxItem
                      key={app.path}
                      value={app.path}
                      textValue={app.name}
                      keywords={[app.name]}
                    >
                      <span className="flex min-w-0 items-center gap-2">
                        {app.icon ? (
                          <img
                            src={app.icon}
                            alt=""
                            className="size-4 shrink-0 rounded"
                          />
                        ) : (
                          <AppWindow
                            className="size-4 shrink-0 text-ink-3"
                            aria-hidden
                          />
                        )}
                        <span className="truncate">{app.name}</span>
                      </span>
                    </ComboboxItem>
                  ))}
                </ComboboxList>
              </ComboboxContent>
            </Combobox>
          </div>

          <p className="mt-1 text-sub text-ink-2">{t("schedule.caveat")}</p>
        </>
      )}
    </section>
  );
}
