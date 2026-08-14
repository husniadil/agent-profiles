import "./styles.css";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { homeDir } from "@tauri-apps/api/path";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

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
const statusElement = document.querySelector<HTMLDivElement>("#status");
const dataRootElement = document.querySelector<HTMLButtonElement>("#data-root");
const dataRootTextElement = document.querySelector<HTMLElement>("#data-root bdi");
const errorElement = document.querySelector<HTMLDivElement>("#error");
const addForm = document.querySelector<HTMLFormElement>("#add-profile-form");
const labelInput = document.querySelector<HTMLInputElement>("#new-label");
const appSelect = document.querySelector<HTMLSelectElement>("#new-app");
const addSubmit = document.querySelector<HTMLButtonElement>("#add-submit");
const addErrorElement = document.querySelector<HTMLDivElement>("#add-error");
const budgetElement = document.querySelector<HTMLDivElement>("#budget");
const budgetPathElement = document.querySelector<HTMLDivElement>("#budget-path");
const budgetFillElement = document.querySelector<HTMLSpanElement>("#budget-fill");
const budgetNoteElement = document.querySelector<HTMLSpanElement>("#budget-note");
const budgetCountElement = document.querySelector<HTMLSpanElement>("#budget-count");
const budgetHelperElement = document.querySelector<HTMLParagraphElement>("#budget-helper");
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
  !addSubmit ||
  !addErrorElement ||
  !budgetElement ||
  !budgetPathElement ||
  !budgetFillElement ||
  !budgetNoteElement ||
  !budgetCountElement ||
  !budgetHelperElement ||
  !budgetAlertElement
) {
  throw new Error("Agent Profiles management window is missing required elements");
}

const appsContainer = appsElement;
const statusText = statusElement;
const dataRootButton = dataRootElement;
const dataRootText = dataRootTextElement;
const errorBox = errorElement;
const profileForm = addForm;
const profileLabelInput = labelInput;
const profileAppSelect = appSelect;
const profileAddButton = addSubmit;
const addErrorBox = addErrorElement;
const budgetBox = budgetElement;
const budgetPath = budgetPathElement;
const budgetFill = budgetFillElement;
const budgetNote = budgetNoteElement;
const budgetCount = budgetCountElement;
const budgetHelper = budgetHelperElement;
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

function makeTextElement(
  tag: "div" | "p" | "span" | "em" | "b" | "strong",
  className: string,
  text: string,
): HTMLElement {
  const element = document.createElement(tag);
  if (className) element.className = className;
  element.textContent = text;
  return element;
}

// ---------------------------------------------------------------------------
// Paths, written the way a person says them
// ---------------------------------------------------------------------------

// Both are learned once and never change while the app runs. They are only ever
// used to shorten what is drawn — every element also keeps the full path — so
// arriving late, or not at all, costs an abbreviation and nothing else.
let dataRoot = "";
let homePath = "";

/// A path split at its last separator, either kind, so the same code reads a
/// Windows path and a POSIX one.
function splitTail(path: string): [string, string] {
  const cut = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return cut < 0 ? ["", path] : [path.slice(0, cut + 1), path.slice(cut + 1)];
}

/// A profile path as the window shows it: everything above our own data root is
/// scenery, and the home directory is a name the reader already knows.
function shortenPath(path: string): string {
  if (dataRoot && path.startsWith(`${dataRoot}/`)) {
    return `…/${splitTail(dataRoot)[1]}${path.slice(dataRoot.length)}`;
  }
  if (homePath && path.startsWith(`${homePath}/`)) {
    return `~${path.slice(homePath.length)}`;
  }
  return path;
}

/// The data root, shortened to the one segment that names it.
function shortenRoot(root: string): string {
  const tail = splitTail(root)[1];
  if (!homePath || !root.startsWith(`${homePath}/`)) return `…/${tail}`;
  const inside = root.slice(homePath.length + 1);
  return inside === tail ? `~/${tail}` : `~/…/${tail}`;
}

/// Draw a path into a row, with its last segment set apart.
///
/// The full value is kept on the element, both as the tooltip — the line is
/// ellipsised, and this is the only thing telling two similar profiles apart —
/// and as the source to redraw from when the home directory arrives.
function paintPath(element: HTMLElement, path: string): void {
  element.dataset.path = path;
  element.title = path;
  const shown = shortenPath(path);
  const [head, tail] = splitTail(shown);
  element.replaceChildren(head, makeTextElement("em", "", tail));
}

/// Redraw every path on screen. The data root and the home directory both
/// arrive after the first list is already drawn, and each one shortens paths
/// that were written out in full a moment earlier.
function paintPaths(): void {
  if (dataRoot) dataRootText.textContent = shortenRoot(dataRoot);
  for (const element of appsContainer.querySelectorAll<HTMLElement>(".path")) {
    paintPath(element, element.dataset.path ?? "");
  }
}

// The line is drawn from the list that is on screen, so the list lives here
// beside the total rather than being passed in: the sizes arrive long after
// `render` has returned, and a repaint that needed the list handed to it would
// mean keeping a second copy of it somewhere just to say the same thing again.
let statusApps: AppView[] = [];
let sizeTotal: number | null = null;
// What the strip currently says, in the plain-text form the whole line reduces
// to. The strip is markup now — counts in bold, separators of their own — so
// there is no single `textContent` to compare against without rebuilding it.
let statusKey = "";

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
  // The strip is a polite live region: it is repainted once per render, and once
  // more when the sizes have all arrived. Rewriting identical text still counts
  // as a change to a screen reader, so an unchanged line is left alone rather
  // than read out again.
  const line = statusLine(profiles, running, sizeTotal);
  if (statusKey === line) return;
  statusKey = line;

  const parts: Array<[string, string]> = [
    [String(profiles), profiles === 1 ? "profile" : "profiles"],
    [String(running), "running"],
  ];
  // Absent until every row has reported: a total that counts half the profiles
  // is a wrong number stated confidently.
  if (sizeTotal !== null) parts.push([formatBytes(sizeTotal), "on disk"]);

  statusText.replaceChildren();
  for (const [index, [value, word]] of parts.entries()) {
    if (index > 0) statusText.append(makeTextElement("span", "sep", "·"));
    statusText.append(makeTextElement("b", "", value), ` ${word}`);
  }
}

// Asked for again on every window-shown rather than cached. The root cannot
// change while the app runs, so this is not about freshness — it is the retry
// for a first attempt that failed.
async function loadDataRoot(): Promise<void> {
  try {
    const root = await invoke<string>("data_root");
    dataRoot = root;
    dataRootButton.hidden = false;
    // The strip shows the shape of the path, not the path, so the full value has
    // to stay reachable. The tooltip belongs on the button, not the `bdi`.
    dataRootButton.title = root;
    paintPaths();
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

// The home directory is a fact about the machine, not about this window, so it
// is read once at startup and never again.
async function loadHome(): Promise<void> {
  try {
    homePath = (await homeDir()).replace(/[\\/]+$/, "");
    paintPaths();
  } catch {
    // Every path stays correct without it — only longer.
  }
}

const ROW_ICONS = { open: "i-open", rename: "i-rename", delete: "i-delete" } as const;

function iconButton(
  icon: keyof typeof ROW_ICONS,
  label: string,
  onClick: () => void,
): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = icon === "delete" ? "icon-btn is-danger" : "icon-btn";
  button.title = label;
  // The picture is the whole control, so the name has to reach a screen reader
  // some other way. `title` covers the pointer; this covers everything else.
  button.setAttribute("aria-label", label);
  // The geometry lives in the sprite in `index.html`; only its id is written
  // here, so nothing user-supplied is ever parsed as markup.
  button.innerHTML = `<svg aria-hidden="true"><use href="#${ROW_ICONS[icon]}"/></svg>`;
  button.addEventListener("click", onClick);
  return button;
}

function profileRow(profile: ProfileView): HTMLLIElement {
  const item = document.createElement("li");
  item.className = "row";

  const dot = document.createElement("span");
  dot.className = profile.running ? "dot live" : "dot";
  // Colour alone is never the message: the same fact is in the tag beside the
  // name, in this tooltip, and counted in words in the status strip.
  dot.title = profile.running ? "running" : "not running";

  const content = document.createElement("div");
  const nameRow = document.createElement("div");
  nameRow.className = "row-name";
  nameRow.append(makeTextElement("span", "name", profile.label));
  if (profile.running) nameRow.append(makeTextElement("span", "tag live", "Running"));
  if (profile.shares_account) {
    nameRow.append(makeTextElement("span", "tag warn", "Shared sign-in"));
  }
  content.append(nameRow);

  const path = document.createElement("p");
  path.className = "path";
  paintPath(path, profile.path);
  content.append(path);

  // Size and actions share one slot: the size is what the row says at rest, the
  // icons are what it offers when reached for.
  const end = document.createElement("div");
  end.className = "row-end";

  const size = makeTextElement("span", "size", "—");
  size.dataset.appId = profile.app_id;
  size.dataset.profileId = profile.id;
  end.append(size);

  const actions = document.createElement("div");
  actions.className = "row-actions";
  actions.append(
    iconButton("open", `Open ${profile.label}`, () => void openProfile(profile)),
    iconButton("rename", `Rename ${profile.label}`, () => startRename(profile, content)),
  );
  // The Default profile is the app's own existing installation, so its directory
  // is never ours to delete. Its label is still just a label.
  if (!profile.is_default) {
    actions.append(
      iconButton("delete", `Delete ${profile.label}`, () => void startDelete(profile, content)),
    );
  }
  end.append(actions);

  item.append(dot, content, end);
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
    appsContainer.append(emptyState(available.length, apps));
    // Clear the picker on the way out. Returning early used to leave whichever
    // options the last render built, so submitting the form after the only
    // installed app disappeared would create a profile directory for an app
    // that is no longer there to launch it.
    renderAppChoices([]);
    return;
  }

  for (const app of available) {
    const group = document.createElement("section");
    group.className = "group";
    // A heading only earns its space once there is a second app to tell apart.
    if (available.length > 1) {
      const head = document.createElement("div");
      head.className = "group-head";
      head.append(
        makeTextElement("span", "group-name", app.label),
        makeTextElement("span", "group-rule", ""),
      );
      group.append(head);
    }
    const list = document.createElement("ul");
    list.className = "rows";
    for (const profile of app.profiles) list.append(profileRow(profile));
    group.append(list);
    appsContainer.append(group);
  }

  renderAppChoices(available);
  void measureSizes(pass);
}

/// The window with nothing to manage.
///
/// The apps are named from what the backend actually looked for rather than
/// from a fixed list, so this sentence cannot come to describe a different set
/// of apps than the one the app supports.
function emptyState(found: number, apps: AppView[]): HTMLElement {
  const empty = document.createElement("div");
  empty.className = "empty";
  const names = apps.map((app) => app.label).join(", ");
  empty.append(
    makeTextElement("div", "empty-mark", `${found} apps found`),
    makeTextElement("div", "empty-title", "Nothing to open yet"),
    makeTextElement(
      "p",
      "empty-body",
      `Agent Profiles runs the coding agents already installed on this computer — ${names}. Install one, then reopen this window.`,
    ),
  );
  return empty;
}

/// Filling in the size of every row, one row at a time, after the list is drawn.
///
/// Sequentially rather than all at once: each of these is a walk of a whole
/// profile directory, and a dozen of them in flight together turns opening the
/// window into a disk storm. Top to bottom also reads as progress.
async function measureSizes(pass: number): Promise<void> {
  const cells = appsContainer.querySelectorAll<HTMLElement>(".size");
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
  // With nothing to add a profile to, the whole band goes: a label over an empty
  // space reads as something failing to load, and the form beneath it is a
  // control that could only fail.
  const section = profileForm.closest("section") as HTMLElement | null;
  if (section) section.hidden = available.length === 0;
  void loadBudget();
}

/// Hiding the meter, for every reason there is not one to draw.
function hideBudget(): void {
  budgetBox.hidden = true;
  budgetHelper.hidden = true;
  profileAddButton.disabled = false;
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
  const appLabel = profileAppSelect.selectedOptions[0]?.textContent ?? "This app";
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
  // The button would submit into a refusal the backend has already decided on.
  profileAddButton.disabled = over;
  // `profile_dir` carries a placeholder id of the right width, not a directory
  // that exists — see the doc comment on the Rust struct. It is drawn because
  // its *length* is the whole subject, and the shape is what makes that legible:
  // the part of the path we chose is set bright, and the part the machine
  // handed us is dimmed to scenery.
  budgetPath.replaceChildren();
  const inside =
    dataRoot && budget.profile_dir.startsWith(`${dataRoot}/`)
      ? budget.profile_dir.slice(dataRoot.length + 1)
      : "";
  if (inside) {
    budgetPath.append(makeTextElement("span", "dim", `${shortenPath(dataRoot)}/`), inside);
  } else {
    budgetPath.append(budget.profile_dir);
  }
  // Ellipsised to one line like every other path in the window, so the value has
  // to stay reachable somewhere.
  budgetPath.title = budget.profile_dir;

  budgetFill.style.width = `${Math.min(100, (budget.used_bytes / limit) * 100)}%`;
  budgetFill.classList.toggle("over", over);
  budgetNote.textContent = over
    ? `${budget.used_bytes - limit} bytes over the limit`
    : `socket path budget · this system stops at ${limit}`;
  budgetNote.classList.toggle("over", over);
  budgetCount.replaceChildren(
    makeTextElement("b", "", String(budget.used_bytes)),
    ` / ${limit} bytes`,
  );
  budgetCount.classList.toggle("over", over);

  budgetHelper.hidden = !over;
  budgetHelper.textContent = over
    ? `${appLabel} would not be able to create its socket here. Move the data root somewhere shorter to make room.`
    : "";

  const alert = over
    ? `This folder is too deep for ${budget.used_bytes - limit} bytes of the socket path a profile needs. No profile can be added here.`
    : "";
  // `loadBudget` runs on every render, and this is an assertive live region:
  // reassigning the same sentence would interrupt to say a thing already said.
  if (budgetAlert.textContent !== alert) budgetAlert.textContent = alert;
}

/// Rename and delete both used to call `window.prompt` / `window.confirm`.
/// Tauri's webview does not implement either one, so both actions silently did
/// nothing. Everything below is drawn in the row instead.
function startRename(profile: ProfileView, content: HTMLElement): void {
  if (content.querySelector(".panel")) return;

  const panel = document.createElement("form");
  panel.className = "panel";

  const input = document.createElement("input");
  input.type = "text";
  input.className = "field";
  input.maxLength = 80;
  input.value = profile.label;
  input.setAttribute("aria-label", `New name for ${profile.label}`);

  const row = document.createElement("div");
  row.className = "panel-row";

  const save = document.createElement("button");
  save.type = "submit";
  save.className = "btn solid";
  save.textContent = "Save name";

  const cancel = document.createElement("button");
  cancel.type = "button";
  cancel.className = "btn outline";
  cancel.textContent = "Cancel";
  cancel.addEventListener("click", () => panel.remove());

  row.append(save, cancel);
  panel.append(input, row);
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
  if (content.querySelector(".panel")) return;

  let size: number;
  try {
    size = await invoke<number>("profile_size_bytes", { appId: profile.app_id, id: profile.id });
  } catch (error) {
    showError(error);
    return;
  }

  const panel = document.createElement("div");
  panel.className = "panel is-danger";

  // The size and the path are set in the mono face here, as they are on the row
  // two lines above. They are the two facts this sentence turns on, and reading
  // them in the prose face is the one place the window would have set a path
  // like a word rather than like a path.
  const question = makeTextElement("p", "panel-text", "");
  question.append(
    "Delete ",
    makeTextElement("strong", "", profile.label),
    " and the ",
    makeTextElement("strong", "figure", formatBytes(size)),
    " in ",
    makeTextElement("strong", "figure", shortenPath(profile.path)),
    ". This can’t be undone.",
  );
  panel.append(question);

  const row = document.createElement("div");
  row.className = "panel-row";

  const confirm = document.createElement("button");
  confirm.type = "button";
  confirm.className = "btn solid-danger";
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
  cancel.className = "btn outline";
  cancel.textContent = "Keep it";
  cancel.addEventListener("click", () => panel.remove());

  row.append(confirm, cancel);
  panel.append(row);
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
    showAddError("Enter a name for this profile.");
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

// The strip names the folder; this is the only way to actually get to it.
dataRootButton.addEventListener("click", async () => {
  if (!dataRoot) return;
  try {
    await revealItemInDir(dataRoot);
    clearError();
  } catch (error) {
    // Unlike reading the root, this one the user asked for.
    showError(error);
  }
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

void loadHome();
void loadProfiles();
void loadDataRoot();
void loadAutostart();
