/// The English dictionary, and the shape every other language is checked
/// against: `type Strings = typeof en` in `./index`, so a locale file that
/// forgets a key fails `pnpm build` rather than rendering a blank label.
///
/// Flat and dotted rather than nested. A nested object gives a nicer-looking
/// file and a worse type error — a missing key in one branch reports as a
/// mismatch on the whole branch, and finding it means reading both files side by
/// side.
///
/// Three strings pluralise, and they are two keys each rather than a plural
/// engine. `id` and `ja` have no plural distinction and get the same string
/// twice; a rule table for three call sites earns nothing.
export const en = {
  // Tabs
  "tab.profiles": "Agent Profiles",
  "tab.keepAwake": "Keep Awake",
  "tab.general": "General",

  // Status strip and its tooltip
  "status.profile": "profile",
  "status.profiles": "profiles",
  "status.running": "running",
  "status.onDisk": "on disk",
  "status.sizing": "Sizing",
  "status.summaryProfile": "{{count}} profile",
  "status.summaryProfiles": "{{count}} profiles",
  "status.summaryRunning": "{{count}} running",
  "status.summaryOnDisk": "{{size}} on disk",
  "status.revealFolder": "Show the profiles folder in the file manager: {{path}}",

  // Start-at-login row, in the General tab
  "autostart.label": "Start at login",
  "autostart.offered": "opens the tray only — no profile is launched",
  "autostart.unavailable": "available once Agent Profiles is installed",
  "autostart.aria": "Start Agent Profiles at login",

  // Nothing installed
  "empty.title": "Nothing to open yet",
  "empty.body":
    "Agent Profiles runs the coding agents already installed on {{machine}}{{names}}. Install one, then reopen this window.",
  "empty.appsSupported": "{{count}} apps supported",

  // Add a profile
  "compose.heading": "New profile",
  "compose.namePlaceholder": "Name this profile",
  "compose.nameAria": "Profile name",
  "compose.appAria": "App",
  "compose.add": "Add",
  "compose.adding": "Adding",
  "compose.added": "Added",
  "compose.retry": "Retry",
  "compose.needName": "Enter a name for this profile.",
  "compose.noApp": "No supported app was found to add a profile to.",
  "compose.thisApp": "This app",

  // A profile row
  "profiles.empty": "No profiles yet.",
  "row.running": "Running",
  "row.sharedSignIn": "Shared sign-in",
  "row.open": "Open {{name}}",
  "row.rename": "Rename {{name}}",
  "row.deleteTrigger": "Delete {{name}}",
  "row.delete": "Delete {{name}} permanently. Press and hold to confirm.",
  "row.deleteUnavailable":
    "{{name}} is the app's own installation and cannot be deleted",
  "row.renameNameAria": "New name for {{name}}",
  "row.saveName": "Save name",
  "row.cancel": "Cancel",
  "row.holdToDelete": "Hold to delete",
  "row.holdingLabel": "Keep holding…",
  "row.completeLabel": "Deleting…",
  "row.keepIt": "Keep it",
  "row.deleteBody":
    "Delete {{label}} and the {{bytes}} in its folder. This can’t be undone.",

  // Socket path budget
  "budget.aria": "Socket path budget",
  "budget.over": "{{bytes}} bytes over the limit",
  "budget.under": "socket path budget · {{system}} stops at {{limit}}",
  "budget.ofLimit": " / {{limit}} bytes",
  "budget.tooDeep":
    "This folder is too deep for {{bytes}} bytes of the socket path a profile needs. No profile can be added here.",
  "budget.cannotCreate":
    "{{app}} would not be able to create its socket here. Move the data root somewhere shorter to make room.",

  // Keep Awake — status card
  "awake.off.title": "Off",
  "awake.off.detail": "{{machine}} sleeps when you close the lid, as usual.",
  "awake.idle.title": "Watching",
  "awake.idle.detail": "Nothing is working right now, so nothing is being held.",
  "awake.holding.title": "Keeping {{machine}} awake",
  "awake.holding.detail":
    "You can close the lid — sleep returns when the work stops.",
  "awake.lowBattery.title": "Paused — battery low",
  "awake.lowBattery.detail": "Dropped to protect the battery. Plug in to resume.",
  "awake.tooHot.title": "Paused — {{machine}} is too hot",
  "awake.tooHot.detail":
    "Holding it awake would make that worse. It resumes once it cools.",
  "awake.stranded":
    "Agent Profiles ended unexpectedly while holding the lid-closed state, and that setting survives a restart.",
  "awake.restoreSleep": "Restore sleep",
  "awake.needsPassword":
    "Needs an administrator password once per run. A helper turns the setting on while an agent works, off when it stops, and shuts down with Agent Profiles.",

  // Keep Awake — status card bands (unsupported, stranded, unauthorized, failed hold)
  "awake.band.unavailable": "Not available here",
  "awake.band.stranded": "Your Mac may not be able to sleep",
  "awake.band.notAuthorized": "Not yet authorized",
  "awake.band.holdFailed": "Not holding — the request failed",
  "awake.band.holdFailedDetail": "{{machine}} will sleep as usual: {{error}}",
  "awake.unsupported.linux":
    "systemd-inhibit was not found, so nothing here can take a lid-switch lock. Holding the lid closed needs a desktop running systemd-logind.",
  "awake.unsupported.generic":
    "{{system}} on {{machine}} reports it cannot hold the lid closed.",
  "awake.authorize": "Authorize…",

  // Keep Awake — status card's assembled status line
  "awake.status.noBattery": "No battery",
  "awake.status.battery": "Battery {{percent}}%",
  "awake.status.pluggedIn": ", plugged in",
  "awake.status.held": " · held {{duration}}",

  // Keep Awake — section legends
  "awake.section.hold": "Hold the machine awake",
  "awake.section.limits": "Limits",
  "awake.section.watching": "Watching",

  // Keep Awake — low-battery control
  "awake.battery.name": "Pause on low battery",
  "awake.battery.aria": "Pause on low battery",
  "awake.battery.below": "below {{percent}}%",

  // Keep Awake — thermal guard
  "awake.thermal.name": "Thermal guard",
  "awake.thermal.aria": "Thermal guard",

  // Keep Awake — hint paragraphs under each Limits setting
  "awake.hint.noBattery": "{{machine}} has no battery, so this never applies.",
  "awake.hint.lowBattery":
    "Dropped below this charge, even mid-task. Ignored while plugged in.",
  "awake.hint.idleWindow":
    "An agent that finished its turn releases {{machine}} at once. This only bounds one that stopped part-way: after this long writing nothing, it is treated as gone rather than working.",
  "awake.hint.thermal":
    "Release the hold when the machine reports it is overheating.",

  // Keep Awake — triggers and limits
  "awake.trigger.off": "Off",
  "awake.trigger.agentActive": "When an agent is working",
  "awake.trigger.agentActiveDetail":
    "A Claude Code or Codex session being written to.",
  "awake.trigger.always": "Always while Agent Profiles runs",
  "awake.trigger.alwaysDetail":
    "For agents inside a desktop app, where there is nothing to detect.",
  "awake.limit.idleWindow": "Give up on a silent agent after",
  "awake.limit.minutes": "min",
  "awake.limit.aria": "{{label}} ({{unit}})",

  // Keep Awake — watch list
  "awake.watch.empty":
    "Nothing to watch yet. Claude Code and Codex are found automatically once they have written a session.",
  "awake.watch.working": "Working",
  "awake.watch.never": "never",
  "awake.watch.ago": "{{duration}} ago",
  "awake.watch.stalled": "stalled {{duration}}",
  "awake.watch.idle": "idle {{duration}}",

  // Machine and system names — "this Mac" reads differently in six languages
  "system.macos": "macOS",
  "system.windows": "Windows",
  "system.linux": "Linux",
  "system.unknown": "this system",
  "machine.mac": "this Mac",
  "machine.pc": "this PC",
  "machine.computer": "this computer",

  // General tab — language
  "general.language.label": "Language",
  "general.language.detail": "Applies to this window and the tray menu.",
  "general.language.system": "Same as system",

  // General tab — updates
  "general.update.label": "Update automatically",
  "general.update.detail": "Installs new releases in the background, then restarts.",
  "general.update.aria": "Install updates automatically",
  "general.update.version": "Version {{version}}",
  "general.update.checkNow": "Check now",
  "general.update.idle": "Not checked yet.",
  "general.update.checking": "Checking for updates…",
  "general.update.current": "Up to date.",
  "general.update.found": "Version {{version}} is available.",
  "general.update.downloading": "Downloading… {{percent}}%",
  "general.update.installing": "Installing, then restarting…",
  "general.update.failed": "Could not update: {{reason}}",
  "general.update.disabled": "Turned off — no release is checked for.",
} as const;
