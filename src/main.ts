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

const appsElement = document.querySelector<HTMLDivElement>("#apps");
const statusElement = document.querySelector<HTMLParagraphElement>("#status");
const dataRootElement = document.querySelector<HTMLParagraphElement>("#data-root");
const dataRootTextElement = document.querySelector<HTMLElement>("#data-root bdi");
const errorElement = document.querySelector<HTMLDivElement>("#error");
const addForm = document.querySelector<HTMLFormElement>("#add-profile-form");
const labelInput = document.querySelector<HTMLInputElement>("#new-label");
const appSelect = document.querySelector<HTMLSelectElement>("#new-app");
const addErrorElement = document.querySelector<HTMLDivElement>("#add-error");

if (
  !appsElement ||
  !statusElement ||
  !dataRootElement ||
  !dataRootTextElement ||
  !errorElement ||
  !addForm ||
  !labelInput ||
  !appSelect ||
  !addErrorElement
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

// Sizes arrive one row at a time, so the total is held here and the line is
// redrawn as it fills in.
let sizeTotal: number | null = null;

function paintStatus(apps: AppView[]): void {
  const available = apps.filter((app) => app.unavailable === null);
  const profiles = available.reduce((total, app) => total + app.profiles.length, 0);
  const running = available.reduce(
    (total, app) => total + app.profiles.filter((p) => p.running).length,
    0,
  );
  const line = statusLine(profiles, running, sizeTotal);
  // The line is a polite live region and it is repainted on every render — and,
  // once the sizes arrive, once per row. Rewriting identical text still counts as
  // a change to a screen reader, so an unchanged line is left alone rather than
  // read out again.
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

function makeTextElement(tag: "h3" | "h4" | "p" | "span", className: string, text: string): HTMLElement {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}

function profileCard(profile: ProfileView, position: number): HTMLLIElement {
  const item = document.createElement("li");
  item.className = "profile-card";

  const index = makeTextElement("span", "profile-index", String(position).padStart(2, "0"));
  const content = document.createElement("div");
  content.className = "profile-content";
  const title = document.createElement("div");
  title.className = "profile-title";
  title.append(makeTextElement("h3", "profile-label", profile.label));
  if (profile.is_default) {
    title.append(makeTextElement("span", "status-badge status-default", "Default"));
  }
  if (profile.shares_account) {
    title.append(makeTextElement("span", "status-badge status-warning", "same account"));
  }
  content.append(title);
  // The path is ellipsised to keep rows one line tall, so the full value has to
  // stay reachable — it is the only thing distinguishing two similar profiles.
  const path = makeTextElement("p", "profile-path", profile.path);
  path.title = profile.path;
  content.append(path);

  const actions = document.createElement("div");
  actions.className = "profile-actions";

  const renameButton = document.createElement("button");
  renameButton.className = "button button-quiet";
  renameButton.type = "button";
  renameButton.textContent = "Rename";
  renameButton.addEventListener("click", () => startRename(profile, content));
  actions.append(renameButton);

  // The Default profile is the app's own existing installation, so its directory
  // is never ours to delete. Its label is still just a label.
  if (!profile.is_default) {
    const deleteButton = document.createElement("button");
    deleteButton.className = "button button-danger";
    deleteButton.type = "button";
    deleteButton.textContent = "Delete";
    deleteButton.addEventListener("click", () => startDelete(profile, content));
    actions.append(deleteButton);
  }

  item.append(index, content, actions);
  return item;
}

function render(apps: AppView[]): void {
  appsContainer.replaceChildren();

  // A fresh render restarts the measuring, so the total stops claiming what the
  // previous list added up to.
  sizeTotal = null;
  paintStatus(apps);

  const available = apps.filter((app) => app.unavailable === null);

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
      group.append(makeTextElement("h4", "app-heading", app.label));
    }
    const list = document.createElement("ul");
    list.className = "profile-list";
    app.profiles.forEach((profile, index) => list.append(profileCard(profile, index + 1)));
    group.append(list);
    appsContainer.append(group);
  }

  renderAppChoices(available);
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
  panel.append(
    makeTextElement(
      "p",
      "helper",
      `Delete “${profile.label}” and all ${formatBytes(size)} in ${profile.path}? This cannot be undone.`,
    ),
  );

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
profileAppSelect.addEventListener("change", clearAddError);

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
  void loadProfiles();
  void loadDataRoot();
  void loadAutostart();
});

void loadProfiles();
void loadDataRoot();
void loadAutostart();
