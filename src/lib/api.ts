import { invoke } from "@tauri-apps/api/core";

/// Every Tauri command the window knows about, in one place.
///
/// The backend is the only thing this window cannot see the source of from here,
/// so the whole surface is written out once rather than spelled into `invoke`
/// calls scattered through the components. A renamed command is one edit.

export type ProfileView = {
  id: string;
  app_id: string;
  label: string;
  path: string;
  is_default: boolean;
  shares_account: boolean;
  running: boolean;
};

export type AppView = {
  id: string;
  label: string;
  unavailable: string | null;
  profiles: ProfileView[];
};

export type SocketBudget = {
  profile_dir: string;
  used_bytes: number;
  limit_bytes: number | null;
};

export type AutostartState = { offered: boolean; enabled: boolean };

export const listApps = () => invoke<AppView[]>("list_apps");
export const dataRoot = () => invoke<string>("data_root");
/// A directory walk's answer: what it added up, and how many entries it could
/// not reach. `skipped` is non-zero when the total is short by an unknown
/// amount, which is a different thing to say than the total alone.
export type ProfileSize = { bytes: number; skipped: number };

export const profileSizeBytes = (appId: string, id: string) =>
  invoke<ProfileSize>("profile_size_bytes", { appId, id });
export const openProfile = (appId: string, id: string) => invoke("open_profile", { appId, id });
export const renameProfile = (appId: string, id: string, label: string) =>
  invoke("rename_profile", { appId, id, label });
export const deleteProfile = (appId: string, id: string) =>
  invoke("delete_profile", { appId, id });
export const addProfile = (appId: string, label: string) => invoke("add_profile", { appId, label });
export const socketBudget = (appId: string) => invoke<SocketBudget>("socket_budget", { appId });
export const autostartState = () => invoke<AutostartState>("autostart_state");
export const setAutostart = (enabled: boolean) => invoke("set_autostart", { enabled });

export type Trigger = "off" | "agent-active" | "always";

export type Phase = "off" | "idle" | "holding" | "paused-low-battery" | "paused-too-hot";

/// The system's own four-level reading, or Linux's sysfs zones banded to match.
/// "unknown" is Windows, a machine with no readable sensor,
/// and never counts as hot.
export type Thermal = "unknown" | "nominal" | "fair" | "serious" | "critical";

export type KeepAwakeSettings = {
  trigger: Trigger;
  idle_window_minutes: number;
  battery_floor_percent: number;
  /// Whether an overheating machine releases the hold. On by default, and on
  /// for a settings file written before this existed.
  thermal_guard: boolean;
};

/// One watched session root, how long ago anything under it was written, and
/// whether the agent there is part-way through a turn.
///
/// `seconds_ago` is null when nothing ever has — never confused with zero.
/// `mid_turn` is false only when a transcript positively says the turn ended; a
/// root read by freshness alone is always true.
export type Freshness = {
  label: string;
  path: string;
  seconds_ago: number | null;
  mid_turn: boolean;
};

export type KeepAwakeStatus = {
  supported: boolean;
  /// Whether this machine can report how hot it is. False means the thermal
  /// guard is left out of the window rather than shown unable to fire.
  thermal_supported: boolean;
  /// Whether holding costs an administrator password. True only on macOS —
  /// elsewhere the window shows no authorization step at all.
  needs_authorization: boolean;
  authorized: boolean;
  stranded: boolean;
  phase: Phase;
  settings: KeepAwakeSettings;
  roots: Freshness[];
  battery_percent: number | null;
  on_external_power: boolean;
  thermal: Thermal;
  held_for_secs: number;
  refusal: string | null;
  /// Why the last sweep could not make the flag match its decision. Non-null
  /// means the machine is not being held whatever `phase` says.
  hold_error: string | null;
};

export const keepAwakeStatus = () => invoke<KeepAwakeStatus>("keep_awake_status");
export const setKeepAwake = (settings: KeepAwakeSettings) =>
  invoke<KeepAwakeStatus>("set_keep_awake", { settings });
export const authorizeKeepAwake = () => invoke<KeepAwakeStatus>("authorize_keep_awake");
export const restoreSleep = () => invoke<KeepAwakeStatus>("restore_sleep");

/// Hands the OS-level hold back on the way into an update install, leaving the
/// trigger armed for the relaunched app. The updater exits the process itself —
/// on Windows without reaching either exit event — so the window calls this
/// before installing, while our own code still runs. See `useUpdater`.
export const releaseKeepAwakeForUpdate = () => invoke("release_keep_awake_for_update");

/// Puts the sweep back after an install that never happened, so keep-awake is not
/// left switched off for the rest of a run the failed install did not end.
export const resumeKeepAwakeAfterFailedUpdate = () =>
  invoke("resume_keep_awake_after_failed_update");

/// One weekday the schedule is turned on for, with its own launch time.
/// `weekday` is Monday-first (0 = Monday … 6 = Sunday); `time` is a local
/// "HH:MM". The Mac wakes one minute earlier than the time shown.
export type DayWake = {
  weekday: number;
  time: string;
};

export type ScheduleSettings = {
  enabled: boolean;
  /// Only the weekdays the user turned on, each carrying its own time.
  days: DayWake[];
  /// Absolute .app path to open at the scheduled time (e.g. /Applications/Slack.app).
  app_path: string;
};

export type ScheduleStatus = {
  supported: boolean;
  refusal: string | null;
  settings: ScheduleSettings;
  /// Days of already-armed one-off wakes left, or null while disabled,
  /// unsupported, or nothing is installed yet — per-day times cost this
  /// horizon instead of a permanent OS-level slot, so the tab says so.
  coverage_days: number | null;
};

/// One installed application the schedule picker can choose. `icon` is a
/// `data:image/png;base64,…` URI when the backend could read one, else null —
/// the picker renders a fallback glyph in its place.
export type InstalledApp = {
  name: string;
  path: string;
  icon: string | null;
};

export const getSchedule = () => invoke<ScheduleStatus>("get_schedule");
export const setSchedule = (settings: ScheduleSettings) =>
  invoke<ScheduleStatus>("set_schedule", { settings });
export const clearSchedule = () => invoke<ScheduleStatus>("clear_schedule");
export const listApplications = () => invoke<InstalledApp[]>("list_applications");

/// Kept in the same order as `Locale::ALL` in `general.rs`, which is the order
/// the picker offers them in.
export type Locale = "en" | "id" | "ja" | "de" | "es" | "pt";

export type GeneralSettings = {
  autoUpdate: boolean;
  /// `null` is "follow the system". Not the same as `"en"`: someone who never
  /// touched this gets their own language when we add it, and someone who chose
  /// English keeps English.
  locale: Locale | null;
};

export const generalSettings = () => invoke<GeneralSettings>("general_settings");
export const setGeneralSettings = (settings: GeneralSettings) =>
  invoke<GeneralSettings>("set_general_settings", { settings });

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
