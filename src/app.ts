'use strict';

import { testInvoke } from './test-mode.js';

/* Renders the tree that the Rust `scan_directory` command returns. The folder is
   scanned on every call, so nothing here is baked in: "Refresh" re-scans it. */

interface Issue {
  rule: string;
  message: string;
  category: string;
}

interface FileEntry {
  name: string;
  size: number;
  asset: boolean;
  issues: Issue[];
  typeFamily: string;
  typeLabel: string;
  typeName: string;
  // The files this one references, by path. The ?test fixture carries none.
  references?: string[];
}

interface TreeNode {
  name: string;
  isDir: boolean;
  files: FileEntry[];
  children: TreeNode[];
  assets: number;
  total: number;
  bytes: number;
  issues: Issue[];
  violations: number;
  namingViolations: number;
  structureViolations: number;
}

interface Violation {
  rule: string;
  message: string;
  path: string;
  category: string;
}

interface Payload {
  root: string;
  scannedAt: number;
  tree: TreeNode;
  violations: Violation[];
  lintApplied: boolean;
  looksUnreal: boolean;
}

interface CatalogRule {
  id: string;
  label: string;
}

interface CatalogCategory {
  id: string;
  rules: CatalogRule[];
}

interface CatalogGuide {
  id: string;
  label: string;
  url: string | null;
  rules: string[];
  custom: boolean;
}

interface Catalog {
  categories: CatalogCategory[];
  guides: CatalogGuide[];
}

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} is missing from index.html`);
  return el as T;
}

// The Tauri runtime is absent in a plain browser; `?test` swaps in the bundled demo
// backend so the UI can be driven from Playwright without a Rust process or a folder.
const invoke = window.__TAURI__?.core.invoke ?? testInvoke();

const treeEl = byId<HTMLUListElement>('tree');
const rootEl = byId<HTMLParagraphElement>('root');
const statsEl = byId<HTMLParagraphElement>('stats');
const filterEl = byId<HTMLInputElement>('filter');
const filesEl = byId<HTMLInputElement>('showFiles');
const namingEl = byId<HTMLElement>('namingRules');
const structureEl = byId<HTMLElement>('structureRules');
const associationEl = byId<HTMLElement>('associationRules');
const namingAllEl = byId<HTMLInputElement>('namingAll');
const structureAllEl = byId<HTMLInputElement>('structureAll');
const associationAllEl = byId<HTMLInputElement>('associationAll');

// Each category is a single control: the rule picker with the switch that checks
// or clears the whole category sitting inside it.
const categories = [
  [namingEl, namingAllEl],
  [structureEl, structureAllEl],
  [associationEl, associationAllEl],
] as const;
const issuesEl = byId<HTMLInputElement>('issuesOnly');
const summaryEl = byId<HTMLElement>('summary');
const legendEl = byId<HTMLElement>('legend');
const pickEl = byId<HTMLButtonElement>('pick');
const errorEl = byId<HTMLParagraphElement>('error');
const loadingEl = byId<HTMLParagraphElement>('loading');
const darkEl = byId<HTMLInputElement>('darkMode');
const guideMenuEl = byId<HTMLElement>('guideMenu');
const guidePresetsEl = byId<HTMLElement>('guidePresets');
const guideSummaryEl = byId<HTMLElement>('guideSummary');
const guideDocsEl = byId<HTMLAnchorElement>('guideDocs');
const presetNameEl = byId<HTMLInputElement>('presetName');
const presetSaveEl = byId<HTMLButtonElement>('presetSave');

/* Apply the stored theme before anything else renders, so a dark choice does not
   show light first for long. :root is light, so a missing or unreadable
   preference still lands on light. */
const themeKey = 'unrealDirectoryScanner.theme';

function applyTheme(dark: boolean): void {
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  darkEl.checked = dark;
}

applyTheme(localStorage.getItem(themeKey) === 'dark');

const openBelowDepth = 1; // start with the top two levels unfolded
const storageKey = 'unrealDirectoryScanner.path';
const selectionKey = 'unrealDirectoryScanner.selection';

let model: Payload | null = null;
let root = '';
function ruleBoxes(el: HTMLElement): HTMLInputElement[] {
  return [...el.querySelectorAll<HTMLInputElement>('.rules__menu input[type=checkbox]')];
}

function checkedRules(el: HTMLElement): string[] {
  return ruleBoxes(el)
    .filter((box) => box.checked)
    .map((box) => box.value);
}

// Every rule ticked anywhere, in the order the categories are declared. This is the one place the
// category list is spelled out, so a fourth one could not be left half wired.
function everyCheckedRule(): string[] {
  return categories.flatMap(([el]) => checkedRules(el));
}

// The rules the pickers are asking for. It stays null until a rule or a guide is
// touched, so that an untouched UI lets the backend decide: the default guide, but
// only when the folder looks like Unreal content.
let rulesTouched = false;

function pickedRuleSelection(): string[] | null {
  return rulesTouched ? everyCheckedRule() : null;
}

// Nothing picked means the category is off, which the backend is told about.
function categoryOn(el: HTMLElement): boolean {
  return checkedRules(el).length > 0;
}

// The switch itself shows whether the category is on, so no state text is needed.
function renderRuleTriggers(): void {
  for (const [el, master] of categories) {
    const boxes = ruleBoxes(el);
    const on = boxes.filter((box) => box.checked).length;
    master.checked = on > 0;
    master.indeterminate = on > 0 && on < boxes.length;
  }
}

/* A guide is a named preset over the same rule checkboxes: picking one ticks exactly
   its rules and rescans. Which guide is showing is not remembered but worked out from the
   boxes, so a hand-edited selection simply matches none and reads "(edited)" - there is no
   Custom entry to keep in step. Presets of the user's own come from the folder beside the
   executable, which the backend reads, and can be saved and deleted from here. */
let catalog: Catalog | null = null;

function guideInputs(): HTMLInputElement[] {
  return [...guideMenuEl.querySelectorAll<HTMLInputElement>('input[type=radio]')];
}

/// The guide the boxes add up to, if any. A preset with no rules at all is not a match for an
/// empty selection: an empty selection is everything switched off, not a broken file.
function matchingGuide(): CatalogGuide | undefined {
  const picked = new Set(everyCheckedRule());
  return catalog?.guides.find(
    (guide) =>
      guide.rules.length > 0 &&
      guide.rules.length === picked.size &&
      guide.rules.every((rule) => picked.has(rule)),
  );
}

/* Saving over a preset and deleting one both throw away what was there, so neither happens on a
   single click: the first click arms the control and a second one carries it out. Only one is
   armed at a time, and a picker that is closed disarms them, so a stray click cannot fire. */
type PresetAction = 'save' | 'delete';

const PRESET_ACTIONS: Record<
  PresetAction,
  { idle: string; armed: string; title: (file: string, armed: boolean) => string }
> = {
  save: {
    idle: 'save',
    armed: 'overwrite?',
    title: (file, armed) =>
      armed ? `click again to write ${file}` : `write the rules ticked now to ${file}`,
  },
  delete: {
    idle: 'delete',
    armed: 'confirm',
    title: (file, armed) => (armed ? `click again to delete ${file}` : `delete ${file}`),
  },
};

/// How a preset's file reads in a tooltip, which is what an empty name box means.
function presetFile(name: string): string {
  return name ? `${name}.preset` : 'a new preset';
}

let armed: { action: PresetAction; id: string } | null = null;

/// Painted from the one `armed` record, so the buttons on the rows and the box at the bottom of
/// the picker all show the same state.
function paintPresetAction(button: HTMLButtonElement, action: PresetAction, name: string): void {
  const isArmed = armed?.action === action && armed.id === name;
  const file = presetFile(name);
  button.textContent = isArmed ? PRESET_ACTIONS[action].armed : PRESET_ACTIONS[action].idle;
  button.title = PRESET_ACTIONS[action].title(file, isArmed);
  button.setAttribute('aria-label', button.title);
  button.classList.toggle('preset__action--armed', isArmed);
}

/// One click arms, a second carries out. `name` is read afresh each time, so the save box follows
/// whatever has been typed into it.
function onPresetAction(
  button: HTMLButtonElement,
  action: PresetAction,
  name: () => string,
  run: () => void,
): void {
  paintPresetAction(button, action, name());
  button.addEventListener('click', (event) => {
    // a row's button sits inside the row's label, so the click must not also pick the guide
    event.preventDefault();
    event.stopPropagation();
    if (armed?.action !== action || armed.id !== name()) {
      armed = { action, id: name() };
    } else {
      armed = null;
      run();
    }
    repaintGuides();
  });
}

/// Rebuilding the rows is how the armed control repaints, since only one is ever armed.
function repaintGuides(): void {
  if (catalog) renderGuides(catalog.guides);
  renderGuideTrigger();
  paintNewPreset();
}

/// An empty box has nothing to save, so the button beside it has nothing to arm.
function paintNewPreset(): void {
  const name = presetNameEl.value.trim();
  presetSaveEl.disabled = name.length === 0;
  paintPresetAction(presetSaveEl, 'save', name);
}

function presetAction(action: PresetAction, id: string): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = `preset__action preset__action--${action}`;
  onPresetAction(
    button,
    action,
    () => id,
    () => (action === 'save' ? void writePreset(id) : void deletePreset(id)),
  );
  return button;
}

function addGuideOption(guide: CatalogGuide): HTMLLabelElement {
  const item = document.createElement('label');
  item.className = 'rules__item';
  const input = document.createElement('input');
  input.type = 'radio';
  input.name = 'guide';
  input.value = guide.id;
  item.append(input, guide.label);
  // A preset the user wrote can be written over or taken away, which is editing or deleting its
  // file. Both throw something away, so both take two clicks.
  if (guide.custom) {
    const actions = document.createElement('span');
    actions.className = 'preset__actions';
    actions.append(presetAction('save', guide.id), presetAction('delete', guide.id));
    item.append(actions);
  }
  return item;
}

// Built from the backend's catalog, so the guide list has one source of truth.
function renderGuides(guides: CatalogGuide[]): void {
  const frag = document.createDocumentFragment();
  for (const guide of guides) frag.append(addGuideOption(guide));
  guidePresetsEl.replaceChildren(frag);
}

// The picker must not drift from the markup: a rule the backend knows about but
// index.html has no checkbox for would silently never run.
function assertCatalogMatchesMarkup(catalog: Catalog): void {
  const boxed = new Set(categories.flatMap(([el]) => ruleBoxes(el).map((box) => box.value)));
  const missing = catalog.categories
    .flatMap((category) => category.rules)
    .map((rule) => rule.id)
    .filter((id) => !boxed.has(id));
  if (missing.length > 0) {
    throw new Error(`index.html has no checkbox for rule(s): ${missing.join(', ')}`);
  }
}

function renderGuideTrigger(): void {
  const match = matchingGuide();
  for (const input of guideInputs()) input.checked = input.value === match?.id;
  guideSummaryEl.textContent = `guide: ${match ? match.label : '(edited)'}`;
  // The picked guide's own documentation, when its author published any.
  if (match?.url) {
    guideDocsEl.href = match.url;
    guideDocsEl.title = match.url;
    guideDocsEl.hidden = false;
  } else {
    guideDocsEl.removeAttribute('href');
    guideDocsEl.removeAttribute('title');
    guideDocsEl.hidden = true;
  }
}

/// Ticks exactly these rules and nothing else, which is all a preset amounts to.
function tickRules(wanted: Iterable<string>): void {
  const wantedSet = new Set(wanted);
  for (const [el] of categories) {
    for (const box of ruleBoxes(el)) box.checked = wantedSet.has(box.value);
  }
}

function applyGuide(id: string, touched: boolean): void {
  const guide = catalog?.guides.find((entry) => entry.id === id);
  if (!guide) return;
  tickRules(guide.rules);
  if (touched) rulesTouched = true;
  renderGuideTrigger();
  renderRuleTriggers();
}

/* What the window is on, kept alongside the folder it is looking at: the preset the boxes add up
   to, and the flags themselves. Both are held because a shipped preset ought to follow its own
   definition if that ever changes, while a set ticked by hand - or a preset file since deleted -
   is remembered exactly as it was. The flags alone decide the rules; the name is what lets the
   picker say "minimal" rather than "(edited)" on the next launch. */
interface StoredSelection {
  guide: string;
  flags: string[];
}

function rememberSelection(): void {
  const stored: StoredSelection = {
    guide: matchingGuide()?.id ?? '',
    flags: everyCheckedRule(),
  };
  localStorage.setItem(selectionKey, JSON.stringify(stored));
}

/// Puts the window back on the rules it was last on, or reports that there is nothing to put it
/// back on, which leaves the caller to fall back to the default preset.
function restoreSelection(): boolean {
  const raw = localStorage.getItem(selectionKey);
  if (!raw) return false;
  let stored: Partial<StoredSelection>;
  try {
    stored = JSON.parse(raw) as Partial<StoredSelection>;
  } catch {
    return false; // an entry we cannot read is not worth refusing to open the window over
  }
  const guide = catalog?.guides.find((entry) => entry.id === stored.guide);
  if (guide) {
    applyGuide(guide.id, true);
    return true;
  }
  if (!Array.isArray(stored.flags)) return false;
  tickRules(stored.flags);
  rulesTouched = true;
  renderGuideTrigger();
  renderRuleTriggers();
  return true;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

function plural(n: number, word: string): string {
  return `${n.toLocaleString()} ${word}${n === 1 ? '' : 's'}`;
}

function guides(depth: number): DocumentFragment {
  const out = document.createDocumentFragment();
  for (let i = 0; i < depth; i += 1) {
    const span = document.createElement('span');
    span.className = 'guide';
    out.append(span);
  }
  return out;
}

/* One "! n" tag per category plus a tooltip listing that category's rules. `countingBelow` is how
   much of the count comes from further down the tree, which is what a folder roll-up is.
   An asset is the last level, so it is never passed and its tooltip can never mention anything
   below it. */
function issueMarker(issues: Issue[], category: string, countingBelow = 0): HTMLElement {
  const mine = issues.filter((issue) => issue.category === category);
  const marker = document.createElement('span');
  marker.className = `flag flag--${category}`;
  marker.textContent = `! ${mine.length + countingBelow}`;
  const lines = mine.map((issue) => `${issue.rule}  ${issue.message}`);
  if (countingBelow > 0) lines.push(`${countingBelow} more counted, further down`);
  marker.title = lines.join('\n');
  return marker;
}

/* Assets are always listed; the "all files" toggle adds non-asset files. */
function visibleFiles(node: TreeNode): FileEntry[] {
  return filesEl.checked ? node.files : node.files.filter((file) => file.asset);
}

function renderFile(file: FileEntry, depth: number): HTMLLIElement {
  const li = document.createElement('li');
  li.className = 'node';
  li.dataset.name = file.name.toLowerCase();

  const row = document.createElement('div');
  row.className = `row row--leaf t-${file.typeFamily}`;

  const spacer = document.createElement('span');
  spacer.className = 'caret caret--empty';
  const chip = document.createElement('span');
  chip.className = 'type';
  chip.textContent = file.typeLabel;
  chip.title = file.typeName;
  const name = document.createElement('span');
  name.className = 'name name--leaf';
  name.textContent = file.name;
  const size = document.createElement('span');
  size.className = 'size';
  size.textContent = formatBytes(file.size);

  row.append(guides(depth), spacer, name, size);
  // An asset is the last level, so its tags count only its own issues. One tag per category, in
  // the order the toolbar lists them.
  for (const category of ['structure', 'naming', 'association'] as const) {
    const owned = file.issues.filter((issue) => issue.category === category).length;
    if (owned > 0) row.append(issueMarker(file.issues, category));
  }
  // What this asset points at, with the paths in the tooltip. A row with none says nothing, so the
  // tree stays as quiet as it was before the rule existed.
  const refs = file.references ?? [];
  if (refs.length > 0) {
    const references = document.createElement('span');
    references.className = 'refs';
    references.textContent = `${refs.length} ref${refs.length === 1 ? '' : 's'}`;
    references.title = `references ${refs.length === 1 ? '1 file' : `${refs.length} files`}:\n${refs.join('\n')}`;
    row.append(references);
  }
  // appended last so the type lines up in its own right-hand column
  row.append(chip);
  row.title = `${file.name}\n${file.typeName}\n${formatBytes(file.size)}${file.asset ? ' - asset' : ' - not an asset'}`;
  li.append(row);
  return li;
}

function renderDir(node: TreeNode, depth: number, path: string): HTMLLIElement {
  const li = document.createElement('li');
  li.className = 'node';
  li.dataset.name = node.name.toLowerCase();
  li.dataset.path = path;
  if (depth <= openBelowDepth) li.classList.add('is-open');

  const files = visibleFiles(node);
  const expandable = node.children.length > 0 || files.length > 0;
  if (!expandable) li.classList.remove('is-open');

  const row = document.createElement('div');
  row.className = expandable ? 'row row--dir' : 'row';

  const caret = document.createElement('button');
  caret.type = 'button';
  caret.className = expandable ? 'caret' : 'caret caret--empty';
  caret.setAttribute('aria-label', `toggle ${node.name}`);

  const name = document.createElement('span');
  name.className = 'name';
  name.textContent = `${node.name}/`;

  // show assets when there are any, otherwise the plain file count, so a folder
  // like Python/ (scripts, no assets) does not read as empty
  const badge = document.createElement('span');
  badge.className = 'count';
  if (node.assets > 0 || node.total === 0) {
    badge.textContent = node.assets.toLocaleString();
    if (node.assets === 0) badge.classList.add('count--dim');
  } else {
    badge.textContent = node.total.toLocaleString();
    badge.classList.add('count--dim');
  }

  row.append(guides(depth), caret, name, badge);
  // A folder's tag counts its whole subtree, so the part of the count that is not its own is said
  // out loud rather than silently folded in.
  for (const [category, counted] of [
    ['structure', node.structureViolations],
    ['naming', node.namingViolations],
  ] as const) {
    const owned = node.issues.filter((issue) => issue.category === category).length;
    if (counted > 0) row.append(issueMarker(node.issues, category, counted - owned));
  }
  row.title = `${node.name}/\n${plural(node.assets, 'asset')}, ${plural(node.total, 'file')}, ${formatBytes(node.bytes)}`;

  const children = document.createElement('ul');
  children.className = 'children';
  for (const child of node.children) children.append(renderDir(child, depth + 1, `${path}/${child.name}`));
  for (const file of files) children.append(renderFile(file, depth + 1));

  li.append(row, children);
  return li;
}

function render(): void {
  if (!model) return;
  const wasOpen = folderOpenState();
  treeEl.replaceChildren(renderDir(model.tree, 0, model.tree.name));
  restoreOpenState(wasOpen);
  statsEl.textContent = `${plural(model.tree.assets, 'asset')}, ${plural(
    model.tree.total,
    'file',
  )}, ${formatBytes(model.tree.bytes)} - scanned ${new Date(model.scannedAt).toLocaleTimeString()}`;
  renderSummary();
  renderLegend();
  renderRuleTriggers();
  applyFilter();
}

/* The rule-by-rule breakdown answers "what is wrong" without opening the tree.
   Naming and structure get a line each, coloured apart, so one cannot hide the
   other. */
function renderSummary(): void {
  if (!model) return;
  summaryEl.hidden = false;
  summaryEl.replaceChildren();

  if (!model.lintApplied) {
    summaryEl.append(
      summaryLine(
        `Allar checks off - ${
          model.looksUnreal
            ? 'every rule is switched off.'
            : 'no Unreal project detected in this folder.'
        }`,
        'summary--clean',
      ),
    );
    return;
  }

  for (const [category, enabled] of [
    ['naming', categoryOn(namingEl)],
    ['structure', categoryOn(structureEl)],
    ['association', categoryOn(associationEl)],
  ] as const) {
    const byRule = new Map<string, number>();
    for (const violation of model.violations) {
      if (violation.category !== category) continue;
      byRule.set(violation.rule, (byRule.get(violation.rule) ?? 0) + 1);
    }
    const total = [...byRule.values()].reduce((sum, count) => sum + count, 0);
    const parts = [...byRule.entries()]
      .sort((a, b) => (a[0] < b[0] ? -1 : 1))
      .map(([rule, count]) => `${rule} x${count}`);
    const body = !enabled
      ? 'checks off'
      : total === 0
        ? 'no violations'
        : `${plural(total, 'violation')} - ${parts.join(' | ')}`;
    const clean = !enabled || total === 0;
    summaryEl.append(
      summaryLine(`${category} checks: ${body}`, clean ? 'summary--clean' : `summary--${category}`),
    );
  }
}

function summaryLine(text: string, modifier: string): HTMLParagraphElement {
  const line = document.createElement('p');
  line.className = `summary ${modifier}`;
  line.textContent = text;
  return line;
}

/* Type census of the scan, in the same colours as the chips. */
function renderLegend(): void {
  if (!model) return;
  const counts = new Map<string, { family: string; count: number }>();
  const walk = (node: TreeNode): void => {
    for (const file of visibleFiles(node)) {
      const entry = counts.get(file.typeName);
      if (entry) entry.count += 1;
      else counts.set(file.typeName, { family: file.typeFamily, count: 1 });
    }
    for (const child of node.children) walk(child);
  };
  walk(model.tree);

  if (counts.size === 0) {
    legendEl.hidden = true;
    return;
  }

  const rows = [...counts.entries()].sort(
    (a, b) => b[1].count - a[1].count || (a[0] < b[0] ? -1 : 1),
  );
  const frag = document.createDocumentFragment();
  for (const [typeName, { family, count }] of rows) {
    const item = document.createElement('span');
    item.className = `legend__item t-${family}`;
    const swatch = document.createElement('span');
    swatch.className = 'legend__swatch';
    const label = document.createElement('span');
    label.className = 'legend__label';
    label.textContent = typeName;
    label.title = `${typeName} - ${count === 1 ? '1 asset' : `${count.toLocaleString()} assets`}`;
    const tally = document.createElement('span');
    tally.className = 'legend__count';
    tally.textContent = count.toLocaleString();
    item.append(swatch, label, tally);
    frag.append(item);
  }
  legendEl.replaceChildren(frag);
  legendEl.hidden = false;
}

/* Filtering walks the data, then hides DOM rows, so a folder survives when any
   descendant matches and the path down to it stays open. */
function filterNode(li: Element, node: TreeNode, query: string, keepContents: boolean): boolean {
  const only = issuesEl.checked;
  const matches = (name: string): boolean => query.length > 0 && name.toLowerCase().includes(query);
  // A folder carrying an issue of its own is one you have to act on, so what is inside it stays with
  // it: the offending row alone expands to nothing, leaving no way to see what is in there.
  const whole = only && (keepContents || node.issues.length > 0);
  let keep = matches(node.name) || (only && (node.violations > 0 || keepContents));

  const files = visibleFiles(node);
  const childLis = [...(li.querySelector('.children')?.children ?? [])];
  let index = 0;
  for (const child of node.children) {
    const childLi = childLis[index++];
    if (childLi && filterNode(childLi, child, query, whole)) keep = true;
  }
  for (const file of files) {
    const fileLi = childLis[index++];
    if (!fileLi) continue;
    const hit = matches(file.name) || (only && (whole || file.issues.length > 0));
    fileLi.classList.toggle('is-hidden', !hit);
    if (hit) keep = true;
  }

  li.classList.toggle('is-hidden', !keep);
  // A folder the filter kept is one you are meant to look at, so it opens. "Has contents" has to
  // mean subfolders or files, as it does when the tree is built: counting only subfolders left a
  // folder full of hits shut, with what you were looking for still out of sight.
  if (keep && (query.length > 0 || only) && (node.children.length > 0 || files.length > 0)) {
    li.classList.add('is-open');
  }
  return keep;
}

function applyFilter(): void {
  const query = filterEl.value.trim().toLowerCase();
  for (const li of treeEl.querySelectorAll('li.node')) li.classList.remove('is-hidden');
  const first = treeEl.firstElementChild;
  if (model && first && (query.length > 0 || issuesEl.checked)) {
    filterNode(first, model.tree, query, false);
  }
}

/* Which folders are unfolded is the user's doing, so it is put back after the tree
   is rebuilt for a new setting. It is keyed by path, so it also carries across a
   re-scan, and folders that are new to the tree keep the default depth. */
function folderOpenState(): Map<string, boolean> {
  const state = new Map<string, boolean>();
  for (const li of treeEl.querySelectorAll<HTMLElement>('li.node[data-path]')) {
    state.set(li.dataset.path ?? '', li.classList.contains('is-open'));
  }
  return state;
}

function restoreOpenState(state: Map<string, boolean>): void {
  for (const li of treeEl.querySelectorAll<HTMLElement>('li.node[data-path]')) {
    const wasOpen = state.get(li.dataset.path ?? '');
    if (wasOpen === undefined) continue;
    li.classList.toggle('is-open', wasOpen);
  }
}

function setOpen(open: boolean): void {
  for (const li of treeEl.querySelectorAll('li.node')) {
    if (li.querySelector('.children')) li.classList.toggle('is-open', open);
  }
}

function fail(message: string): void {
  errorEl.hidden = false;
  errorEl.textContent = message;
}

async function scan(path: string): Promise<void> {
  if (!invoke) return;
  model = await invoke<Payload>('scan_directory', { path, rules: pickedRuleSelection() });
  root = model.root;
  // Whatever we ended up looking at is what the next launch opens, however we got there: picked,
  // remembered, or named on the command line. Only a scan that worked is worth remembering.
  localStorage.setItem(storageKey, root);
  rootEl.textContent = model.root;
  render();
}

/* The guide picker is built from the backend's catalog so the rule ids and the
   presets have one source of truth. On init the default guide is ticked but the UI
   is left "untouched", so an untouched window still lets the backend apply its
   default; any later choice sends an explicit rule list. */
async function init(): Promise<void> {
  if (invoke) {
    try {
      catalog = await invoke<Catalog>('rule_catalog');
      renderGuides(catalog.guides);
      assertCatalogMatchesMarkup(catalog);
      // the rules the window opens on are the ones it was closed on, or the default preset
      if (!restoreSelection()) applyGuide(catalog.guides[0].id, false);
    } catch (error) {
      fail(`Could not read the rule catalog: ${error}`);
    }
  }
  await load();
}

async function load(): Promise<void> {
  errorEl.hidden = true;
  if (!invoke) {
    loadingEl.hidden = true;
    fail('No backend: the Tauri runtime is missing. Add ?test to the URL for the demo.');
    return;
  }
  try {
    if (!root) {
      const explicit = await invoke<string | null>('startup_directory');
      root =
        explicit ?? localStorage.getItem(storageKey) ?? (await invoke<string>('default_directory'));
    }
    await scan(root);
    loadingEl.hidden = true;
  } catch (error) {
    loadingEl.hidden = true;
    fail(`Could not scan ${root || 'the folder'}: ${error}`);
  }
}

/// The catalog is re-read rather than patched, so the folder stays the one source of truth for
/// which presets exist.
async function refreshCatalog(): Promise<void> {
  if (!invoke) return;
  try {
    catalog = await invoke<Catalog>('rule_catalog');
    repaintGuides();
  } catch (error) {
    fail(`Could not read the rule catalog: ${error}`);
  }
}

/* A preset is written as the rules ticked right now, listed one by one, so the file reads as the
   --rules flag that would pick the same set. Writing over a preset and naming a new one are the
   same call; only the name differs. */
async function writePreset(name: string): Promise<boolean> {
  errorEl.hidden = true;
  if (!invoke) return false;
  const flags = everyCheckedRule();
  try {
    await invoke('save_preset', { name, flags, except: [] });
    await refreshCatalog();
    return true;
  } catch (error) {
    fail(`Could not write the preset ${name}: ${error}`);
    return false;
  }
}

async function saveFromBox(): Promise<void> {
  if (await writePreset(presetNameEl.value.trim())) presetNameEl.value = '';
}

async function deletePreset(name: string): Promise<void> {
  errorEl.hidden = true;
  if (!invoke) return;
  try {
    await invoke('delete_preset', { name });
    await refreshCatalog();
  } catch (error) {
    fail(`Could not delete the preset ${name}: ${error}`);
  }
}

async function pick(): Promise<void> {
  errorEl.hidden = true;
  if (!invoke) return;
  try {
    const selected = await invoke<string | null>('plugin:dialog|open', {
      options: { directory: true, multiple: false, title: 'Choose a folder to scan' },
    });
    if (selected) {
      root = selected;
      loadingEl.hidden = false;
      await load();
    }
  } catch (error) {
    fail(`Folder picker failed: ${error}`);
  }
}

treeEl.addEventListener('click', (event) => {
  const row = (event.target as HTMLElement).closest('.row--dir');
  if (!row) return;
  const li = row.closest('.node');
  if (li?.querySelector('.children')) li.classList.toggle('is-open');
});

filterEl.addEventListener('input', applyFilter);
byId<HTMLButtonElement>('expand').addEventListener('click', () => setOpen(true));
byId<HTMLButtonElement>('collapse').addEventListener('click', () => setOpen(false));
byId<HTMLButtonElement>('refresh').addEventListener('click', load);
pickEl.addEventListener('click', pick);
filesEl.addEventListener('change', () => {
  if (model) render();
});
issuesEl.addEventListener('change', applyFilter);
darkEl.addEventListener('change', () => {
  localStorage.setItem(themeKey, darkEl.checked ? 'dark' : 'light');
  applyTheme(darkEl.checked);
});
// Any change to the rules changes which checks the backend runs, so the folder is
// re-scanned rather than the tree merely re-filtered. Editing a box by hand is enough to stop
// the selection matching a preset, which is what the picker then says.
function applyRulesChange(): void {
  rulesTouched = true;
  rememberSelection();
  renderGuideTrigger();
  renderRuleTriggers();
  void load();
}

for (const [el, master] of categories) {
  master.addEventListener('change', () => {
    for (const box of ruleBoxes(el)) box.checked = master.checked;
    applyRulesChange();
  });
  el.addEventListener('change', applyRulesChange);
}

/* Only one picker is open at a time. They are absolutely positioned and overlap, so two open
   at once means the one underneath cannot be clicked - the guide picker sits under the rule
   pickers exactly this way, which is what hid its presets. */
const pickers = [...document.querySelectorAll<HTMLDetailsElement>('.rules > details')];
for (const picker of pickers) {
  picker.addEventListener('toggle', () => {
    if (!picker.open) return;
    for (const other of pickers) if (other !== picker) other.open = false;
  });
}

/* A control left armed while the picker is shut would go off on a single click when it is opened
   again, so closing the picker puts it back. */
const guidePicker = byId<HTMLElement>('guideRules').querySelector('details');
guidePicker?.addEventListener('toggle', () => {
  if (guidePicker.open) return;
  armed = null;
  repaintGuides();
});

/* The anchor carries the real URL, so it shows on hover and can be copied, but the
   click is handed to the backend: letting the webview follow the link would replace
   the app with the web page and leave no way back to the folder. */
guideDocsEl.addEventListener('click', (event) => {
  event.preventDefault();
  const guide = matchingGuide();
  if (!invoke || !guide?.url) return;
  void invoke('open_docs', { guide: guide.id }).catch((error: unknown) => {
    fail(`Could not open the guide: ${error}`);
  });
});

guideMenuEl.addEventListener('change', (event) => {
  const input = event.target as HTMLInputElement;
  if (input.type !== 'radio') return;
  applyGuide(input.value, true);
  rememberSelection();
  void load();
});

onPresetAction(presetSaveEl, 'save', () => presetNameEl.value.trim(), () => void saveFromBox());
presetNameEl.addEventListener('input', paintNewPreset);
presetNameEl.addEventListener('keydown', (event) => {
  // Enter is a second click when the box is already armed, and arms it otherwise
  if (event.key === 'Enter') presetSaveEl.click();
});
paintNewPreset();

void init();

export {};
