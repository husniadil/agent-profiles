import "./styles.css";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { formatBytes, statusLine } from "./format";

type ProfileView = {
  id: string;
  app_id: string;
  label: string;
  path: string;
  is_default: boolean;
  shares_account: boolean;
  running: boolean;
};

type AppView = {
  id: string;
  label: string;
  unavailable: string | null;
  profiles: ProfileView[];
};

type SocketBudget = { profile_dir: string; used_bytes: number; limit_bytes: number | null };

const appsElement = document.querySelector<HTMLDivElement>("#apps");
const statusElement = document.querySelector<HTMLParagraphElement>("#status");
const dataRootElement = document.querySelector<HTMLParagraphElement>("#data-root");
const dataRootTextElement = document.querySelector<HTMLElement>("#data-root bdi");
const errorElement = document.querySelector<HTMLDivElement>("#error");
const addForm = document.querySelector<HTMLFormElement>("#add-profile-form");
const labelInput = document.querySelector<HTMLInputElement>("#new-label");
const appSelect = document.querySelector<HTMLSelectElement>("#new-app");
const addErrorElement = document.querySelector<HTMLDivElement>("#add-error");
const budgetElement = document.querySelector<HTMLDivElement>("#budget");
const budgetPathElement = document.querySelector<HTMLParagraphElement>("#budget-path");
const budgetFillElement = document.querySelector<HTMLSpanElement>("#budget-fill");
const budgetNoteElement = document.querySelector<HTMLSpanElement>("#budget-note");
const budgetCountElement = document.querySelector<HTMLSpanElement>("#budget-count");
const budgetAlertElement = document.querySelector<HTMLParagraphElement>("#budget-alert");

if (
  !appsElement ||
  !statusElement ||
  !dataRootElement ||
  !dataRootTextElement ||
  !errorElement ||
  !addForm ||
  !labelInput ||
  !appSelect ||
  !addErrorElement ||
  !budgetElement ||
  !budgetPathElement ||
  !budgetFillElement ||
  !budgetNoteElement ||
  !budgetCountElement ||
  !budgetAlertElement
) {
  throw new Error("Agent Profiles management window is missing required elements");
}

const appsContainer = appsElement;
const statusText = statusElement;
const dataRootBox = dataRootElement;
const dataRootText = dataRootTextElement;
const errorBox = errorElement;
const profileForm = addForm;
const profileLabelInput = labelInput;
const profileAppSelect = appSelect;
const addErrorBox = addErrorElement;
const budgetBox = budgetElement;
const budgetPath = budgetPathElement;
const budgetFill = budgetFillElement;
const budgetNote = budgetNoteElement;
const budgetCount = budgetCountElement;
const budgetAlert = budgetAlertElement;

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function showError(error: unknown): void {
  errorBox.textContent = errorMessage(error);
  errorBox.hidden = false;
}

function clearError(): void {
  errorBox.textContent = "";
  errorBox.hidden = true;
}

/// Adding a profile reports next to the form rather than in the page banner.
/// The banner sits above the profile list, which on any populated window is far
/// enough above the form to be scrolled out of sight — a refused label then
/// looks like a button that did nothing at all.
function showAddError(error: unknown): void {
  addErrorBox.textContent = errorMessage(error);
  addErrorBox.hidden = false;
}

function clearAddError(): void {
  addErrorBox.textContent = "";
  addErrorBox.hidden = true;
}

// The line is drawn from the list that is on screen, so the list lives here
// beside the total rather than being passed in: the sizes arrive long after
// `render` has returned, and a repaint that needed the list handed to it would
// mean keeping a second copy of it somewhere just to say the same thing again.
let statusApps: AppView[] = [];
let sizeTotal: number | null = null;

// Sizes already measured this visit, keyed `app:profile`. A directory only grows
// while its app is running, and the window is a place you visit for a moment to
// rename or delete something — so measuring once per visit is fresh enough, and
// re-measuring on every list reload is not. Cleared on `window-shown`, which is
// the moment the numbers could have moved without us.
const measured = new Map<string, number>();

function paintStatus(): void {
  const profiles = statusApps.reduce((total, app) => total + app.profiles.length, 0);
  const running = statusApps.reduce(
    (total, app) => total + app.profiles.filter((p) => p.running).length,
    0,
  );
  const line = statusLine(profiles, running, sizeTotal);
  // The line is a polite live region: it is repainted once per render, and once
  // more when the sizes have all arrived. Rewriting identical text still counts
  // as a change to a screen reader, so an unchanged line is left alone rather
  // than read out again.
  if (statusText.textContent !== line) statusText.textContent = line;
}

// Asked for again on every window-shown rather than cached. The root cannot
// change while the app runs, so this is not about freshness — it is the retry
// for a first attempt that failed.
async function loadDataRoot(): Promise<void> {
  try {
    const root = await invoke<string>("data_root");
    dataRootText.textContent = root;
    // The line is ellipsised to one line, so the full value has to stay
    // reachable somewhere. The tooltip belongs on the box, not the `bdi`.
    dataRootBox.title = root;
  } catch {
    // Not worth the error banner: the window works perfectly without knowing
    // where the files are, and the banner is reserved for actions that failed.
    //
    // The last known-good path is left where it is. This value cannot go stale,
    // so a failed re-read is a failure to learn something we may already know —
    // replacing a correct answer with an empty one would be the only way to
    // lose it. Blanking the text alone would also strand the tooltip, leaving a
    // path on hover over nothing.
  }
}

function makeTextElement(tag: "h2" | "h3" | "p" | "span", className: string, text: string): HTMLElement {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}

function iconButton(name: string, label: string, path: string, onClick: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = `row-icon row-icon-${name}`;
  button.title = label;
  // The picture is the whole control, so the name has to reach a screen reader
  // some other way. `title` covers the pointer; this covers everything else.
  button.setAttribute("aria-label", label);
  button.innerHTML = `<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">${path}</svg>`;
  button.addEventListener("click", onClick);
  return button;
}

const ICON_OPEN = `<path d="M9 3h4v4"/><path d="M13 3 7.5 8.5"/><path d="M11 9.5V13H3V5h3.5"/>`;
const ICON_RENAME = `<path d="M10.5 2.5 13.5 5.5 6 13H3v-3z"/><path d="M9 4 12 7"/>`;
const ICON_DELETE = `<path d="M3 4.5h10"/><path d="M6.5 4.5V3h3v1.5"/><path d="M4.5 4.5 5 13.5h6l.5-9"/>`;

function profileCard(profile: ProfileView): HTMLLIElement {
  const item = document.createElement("li");
  item.className = "profile-card";

  const dot = document.createElement("span");
  dot.className = profile.running ? "run-dot run-dot-live" : "run-dot";
  // Colour alone is never the message: the same fact is in the title, and the
  // status line counts it in words.
  dot.title = profile.running ? "running" : "not running";

  const content = document.createElement("div");
  content.className = "profile-content";
  const title = document.createElement("div");
  title.className = "profile-title";
  title.append(makeTextElement("h3", "profile-label", profile.label));
  // The `Default` badge is gone: this profile is recognisable from being the
  // one with no delete action, and a badge that repeats that is decoration.
  if (profile.shares_account) {
    title.append(makeTextElement("span", "status-badge status-warning", "same account"));
  }
  content.append(title);
  // The path is ellipsised to keep rows one line tall, so the full value has to
  // stay reachable — it is the only thing distinguishing two similar profiles.
  const path = makeTextElement("p", "profile-path", profile.path);
  path.title = profile.path;
  content.append(path);

  // Size and actions share one slot: the size is what the row says at rest, the
  // icons are what it offers when reached for.
  const trailing = document.createElement("div");
  trailing.className = "profile-trailing";

  const size = makeTextElement("span", "profile-size", "—");
  size.dataset.appId = profile.app_id;
  size.dataset.profileId = profile.id;
  trailing.append(size);

  const actions = document.createElement("div");
  actions.className = "profile-actions";
  actions.append(
    iconButton("open", `Open ${profile.label}`, ICON_OPEN, () => void openProfile(profile)),
    iconButton("rename", `Rename ${profile.label}`, ICON_RENAME, () => startRename(profile, content)),
  );
  // The Default profile is the app's own existing installation, so its directory
  // is never ours to delete. Its label is still just a label.
  if (!profile.is_default) {
    actions.append(
      iconButton("delete", `Delete ${profile.label}`, ICON_DELETE, () => void startDelete(profile, content)),
    );
  }
  trailing.append(actions);

  item.append(dot, content, trailing);
  return item;
}

async function openProfile(profile: ProfileView): Promise<void> {
  try {
    await invoke("open_profile", { appId: profile.app_id, id: profile.id });
    clearError();
    await loadProfiles();
  } catch (error) {
    showError(error);
  }
}

// Every render retires whatever measuring was still running: a walk started for
// the previous list can still be waiting on the disk when a profile is deleted,
// and its bytes belong to a list that is no longer on screen.
let renderPass = 0;

function render(apps: AppView[]): void {
  appsContainer.replaceChildren();

  const pass = (renderPass += 1);
  // A fresh render restarts the measuring, so the total stops claiming what the
  // previous list added up to.
  sizeTotal = null;

  // The line only ever describes apps that are actually installed, and so does
  // the list below it, so the filter runs once and both read the same answer.
  const available = apps.filter((app) => app.unavailable === null);
  statusApps = available;
  paintStatus();

  // Nothing installed is the only case worth explaining. With one app working,
  // the other's absence is not an error — it is simply not installed.
  if (available.length === 0) {
    for (const app of apps) {
      appsContainer.append(makeTextElement("p", "helper", app.unavailable ?? ""));
    }
    // Clear the picker on the way out. Returning early used to leave whichever
    // options the last render built, so submitting the form after the only
    // installed app disappeared would create a profile directory for an app
    // that is no longer there to launch it.
    renderAppChoices([]);
    return;
  }

  for (const app of available) {
    const group = document.createElement("section");
    group.className = "app-group";
    // A heading only earns its space once there is a second app to tell apart.
    if (available.length > 1) {
      group.append(makeTextElement("h2", "app-heading", app.label));
    }
    const list = document.createElement("ul");
    list.className = "profile-list";
    for (const profile of app.profiles) list.append(profileCard(profile));
    group.append(list);
    appsContainer.append(group);
  }

  renderAppChoices(available);
  void measureSizes(pass);
}

/// Filling in the size of every row, one row at a time, after the list is drawn.
///
/// Sequentially rather than all at once: each of these is a walk of a whole
/// profile directory, and a dozen of them in flight together turns opening the
/// window into a disk storm. Top to bottom also reads as progress.
async function measureSizes(pass: number): Promise<void> {
  const cells = appsContainer.querySelectorAll<HTMLElement>(".profile-size");
  // The running sum belongs to this pass alone. Held in module state it would be
  // shared with whatever render replaced us, and the bytes of a profile that has
  // since been deleted would be added to the new list's total.
  let total = 0;
  let complete = true;

  for (const cell of cells) {
    const appId = cell.dataset.appId ?? "";
    const id = cell.dataset.profileId ?? "";
    const key = `${appId}:${id}`;
    // Renaming a profile cannot change a byte of it, and neither can opening it,
    // but both reload the list — so without this every rename would blank every
    // row and walk every profile directory again, seconds of I/O to arrive back
    // at the same numbers. A profile is measured once per window session.
    const known = measured.get(key);
    if (known !== undefined) {
      cell.textContent = formatBytes(known);
      total += known;
      continue;
    }
    try {
      const bytes = await invoke<number>("profile_size_bytes", { appId, id });
      // The list this cell belongs to has been replaced; the cell is detached and
      // the bytes are about a row nobody is looking at.
      if (pass !== renderPass) return;
      measured.set(key, bytes);
      cell.textContent = formatBytes(bytes);
      total += bytes;
    } catch {
      if (pass !== renderPass) return;
      // No banner: a size that could not be read is not an action that failed,
      // and the row still says everything else it has to say. The remaining rows
      // are still worth filling in, so the walk carries on — but the total is
      // now unknowable, and a total missing a profile is a wrong total.
      cell.textContent = "—";
      complete = false;
    }
  }

  if (pass !== renderPass || !complete) return;
  sizeTotal = total;
  paintStatus();
}

// The picker is only a question when there is more than one answer.
function renderAppChoices(available: AppView[]): void {
  const previous = profileAppSelect.value;
  profileAppSelect.replaceChildren();
  for (const app of available) {
    const option = document.createElement("option");
    option.value = app.id;
    option.textContent = app.label;
    profileAppSelect.append(option);
  }
  if (available.some((app) => app.id === previous)) {
    profileAppSelect.value = previous;
  }
  const picker = profileAppSelect.closest(".app-picker") as HTMLElement | null;
  if (picker) picker.hidden = available.length < 2;
  // With nothing to add a profile to, the whole section goes: a heading over an
  // empty space reads as something failing to load, and the form beneath it is a
  // control that could only fail.
  const section = profileForm.closest("section") as HTMLElement | null;
  if (section) section.hidden = available.length === 0;
  void loadBudget();
}

/// Hiding the meter, for every reason there is not one to draw.
function hideBudget(): void {
  budgetBox.hidden = true;
  // A verdict left in the live region would be read out the next time anything
  // touched it, long after it stopped being true of the selected app.
  budgetAlert.textContent = "";
}

/// The socket-path budget for the app currently selected in the picker.
///
/// Not wired to the label field, and deliberately so: `ProfileStore::add` names
/// a profile directory after a generated id, never after what was typed, so the
/// number cannot move as the user types. It is a property of the data root — on
/// most machines a comfortable constant, and on a long home directory the reason
/// no profile can be created at all.
async function loadBudget(): Promise<void> {
  const appId = profileAppSelect.value;
  if (!appId) {
    hideBudget();
    return;
  }
  let budget: SocketBudget;
  try {
    budget = await invoke<SocketBudget>("socket_budget", { appId });
  } catch {
    // No banner, for the same reason the data root has none: this is a reading
    // the window offers, not an action the user asked for and did not get.
    hideBudget();
    return;
  }
  // The picker moved on while this was in flight. Two apps sit at two depths, so
  // an answer about the one that was selected a moment ago is the wrong number
  // under the app that is selected now.
  if (profileAppSelect.value !== appId) return;
  const limit = budget.limit_bytes;
  // Windows puts its named pipes outside the profile, so there is no budget to
  // keep and a meter there would invent a limit that means nothing.
  if (limit === null) {
    hideBudget();
    return;
  }

  const over = budget.used_bytes > limit;
  budgetBox.hidden = false;
  budgetBox.classList.toggle("budget-over", over);
  // `profile_dir` carries a placeholder id of the right width, not a directory
  // that exists — see the doc comment on the Rust struct. It is drawn because
  // its *length* is the whole subject, and the shape is what makes that legible.
  budgetPath.textContent = budget.profile_dir;
  // Ellipsised to one line like every other path in the window, so the value has
  // to stay reachable somewhere.
  budgetPath.title = budget.profile_dir;
  budgetFill.style.width = `${Math.min(100, (budget.used_bytes / limit) * 100)}%`;
  budgetNote.textContent = over
    ? `no room for the socket apps create inside a profile — ${budget.used_bytes - limit} bytes over`
    : `socket path budget · this system stops at ${limit}`;
  budgetCount.textContent = `${budget.used_bytes} / ${limit} bytes`;
  const alert = over
    ? `This folder is too deep for ${budget.used_bytes - limit} bytes of the socket path a profile needs. No profile can be added here.`
    : "";
  // `loadBudget` runs on every render, and this is an assertive live region:
  // reassigning the same sentence would interrupt to say a thing already said.
  if (budgetAlert.textContent !== alert) budgetAlert.textContent = alert;
}

/// Rename and delete both used to call `window.prompt` / `window.confirm`.
/// Tauri's webview does not implement either one, so both actions silently did
/// nothing. Everything below is drawn in the page instead.
function startRename(profile: ProfileView, content: HTMLElement): void {
  if (content.querySelector(".inline-panel")) return;

  const panel = document.createElement("form");
  panel.className = "inline-panel";

  const input = document.createElement("input");
  input.type = "text";
  input.maxLength = 80;
  input.value = profile.label;
  input.setAttribute("aria-label", `New label for ${profile.label}`);

  const save = document.createElement("button");
  save.type = "submit";
  save.className = "button button-primary";
  save.textContent = "Save";

  const cancel = document.createElement("button");
  cancel.type = "button";
  cancel.className = "button button-quiet";
  cancel.textContent = "Cancel";
  cancel.addEventListener("click", () => panel.remove());

  panel.append(input, save, cancel);
  panel.addEventListener("submit", async (event) => {
    event.preventDefault();
    const label = input.value.trim();
    if (!label || label === profile.label) {
      panel.remove();
      return;
    }
    try {
      await invoke("rename_profile", { appId: profile.app_id, id: profile.id, label });
      clearError();
      await loadProfiles();
    } catch (error) {
      showError(error);
    }
  });

  content.append(panel);
  input.focus();
  input.select();
}

async function startDelete(profile: ProfileView, content: HTMLElement): Promise<void> {
  if (content.querySelector(".inline-panel")) return;

  let size: number;
  try {
    size = await invoke<number>("profile_size_bytes", { appId: profile.app_id, id: profile.id });
  } catch (error) {
    showError(error);
    return;
  }

  const panel = document.createElement("div");
  panel.className = "inline-panel inline-panel-danger";
  // The size and the path are set in the mono face here, as they are on the row
  // two lines above. They are the two facts this sentence turns on, and reading
  // them in the prose face is the one place the window would have set a path
  // like a word rather than like a path.
  const question = makeTextElement("p", "helper", "");
  question.append(
    `Delete “${profile.label}” and all `,
    makeTextElement("span", "figure", formatBytes(size)),
    " in ",
    makeTextElement("span", "figure", profile.path),
    "? This cannot be undone.",
  );
  panel.append(question);

  const confirm = document.createElement("button");
  confirm.type = "button";
  confirm.className = "button button-danger";
  confirm.textContent = "Delete permanently";
  confirm.addEventListener("click", async () => {
    try {
      await invoke("delete_profile", { appId: profile.app_id, id: profile.id });
      clearError();
      await loadProfiles();
    } catch (error) {
      showError(error);
    }
  });

  const cancel = document.createElement("button");
  cancel.type = "button";
  cancel.className = "button button-quiet";
  cancel.textContent = "Keep it";
  cancel.addEventListener("click", () => panel.remove());

  panel.append(confirm, cancel);
  content.append(panel);
  confirm.focus();
}

async function loadProfiles(): Promise<void> {
  try {
    const apps = await invoke<AppView[]>("list_apps");
    render(apps);
    clearError();
  } catch (error) {
    showError(error);
  }
}

async function addProfile(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  const label = profileLabelInput.value.trim();
  if (!label) {
    showAddError("Enter a label for this profile.");
    profileLabelInput.focus();
    return;
  }
  const appId = profileAppSelect.value;
  if (!appId) {
    showAddError("No supported app was found to add a profile to.");
    return;
  }

  try {
    await invoke("add_profile", { appId, label });
    profileLabelInput.value = "";
    clearAddError();
    await loadProfiles();
  } catch (error) {
    showAddError(error);
  }
}

// A refusal is about the label as it was submitted. The moment it is edited the
// verdict is stale, and leaving it on screen invites the reader to believe the
// new label was rejected too.
profileLabelInput.addEventListener("input", clearAddError);
// Switching app switches which data root the meter is about — the two apps sit
// at different depths under the same root, so the number is not the same twice.
profileAppSelect.addEventListener("change", () => {
  clearAddError();
  void loadBudget();
});

// A desktop app has no business offering "Reload" or "Inspect Element" on
// right-click. Keep the caret menu inside text fields, where it is useful.
document.addEventListener("contextmenu", (event) => {
  const target = event.target as HTMLElement | null;
  if (target?.closest("input, textarea")) return;
  event.preventDefault();
});

type AutostartState = { offered: boolean; enabled: boolean };

const autostartSection = document.querySelector<HTMLElement>("#autostart-section");
const autostartToggle = document.querySelector<HTMLInputElement>("#autostart");

// The operating system owns this setting, so the checkbox is refreshed from it
// rather than remembered here — the user may have changed it in System Settings.
async function loadAutostart(): Promise<void> {
  if (!autostartSection || !autostartToggle) return;
  try {
    const state = await invoke<AutostartState>("autostart_state");
    autostartSection.hidden = !state.offered;
    autostartToggle.checked = state.enabled;
  } catch (error) {
    autostartSection.hidden = true;
    showError(error);
  }
}

autostartToggle?.addEventListener("change", async () => {
  const wanted = autostartToggle.checked;
  try {
    await invoke("set_autostart", { enabled: wanted });
    clearError();
  } catch (error) {
    showError(error);
  }
  // Re-read rather than trusting the click: if the OS refused, the checkbox must
  // show what is actually true, not what the user asked for.
  await loadAutostart();
});

profileForm.addEventListener("submit", addProfile);

// Closing the window only hides it, so the page keeps whatever it was last
// showing. An error like "quit this profile's Claude Desktop before deleting it"
// is a verdict about one moment — by the time the window is reopened the user has
// very likely done exactly that. Start every visit from freshly loaded state.
void listen("window-shown", () => {
  clearError();
  clearAddError();
  // A profile grows while its app runs, and the window is closed for most of
  // that. Reopening it is the one moment the sizes are worth walking again.
  measured.clear();
  void loadProfiles();
  void loadDataRoot();
  void loadAutostart();
});

void loadProfiles();
void loadDataRoot();
void loadAutostart();
