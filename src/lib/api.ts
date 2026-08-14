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

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
