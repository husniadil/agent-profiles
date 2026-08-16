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
export const profileSizeBytes = (appId: string, id: string) =>
  invoke<number>("profile_size_bytes", { appId, id });
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

/// The system's own four-level reading. "unknown" is every platform but macOS,
/// and never counts as hot.
export type Thermal = "unknown" | "nominal" | "fair" | "serious" | "critical";

export type KeepAwakeSettings = {
  trigger: Trigger;
  idle_window_minutes: number;
  battery_floor_percent: number;
};

/// One watched session root and how long ago anything under it was written.
/// `seconds_ago` is null when nothing ever has — never confused with zero.
export type Freshness = { label: string; path: string; seconds_ago: number | null };

export type KeepAwakeStatus = {
  supported: boolean;
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

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
