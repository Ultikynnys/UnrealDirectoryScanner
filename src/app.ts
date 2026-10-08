'use strict';

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

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} is missing from index.html`);
  return el as T;
}

const invoke = window.__TAURI__?.core.invoke ?? null;

const treeEl = byId<HTMLUListElement>('tree');
const rootEl = byId<HTMLParagraphElement>('root');
const statsEl = byId<HTMLParagraphElement>('stats');
const filterEl = byId<HTMLInputElement>('filter');
const filesEl = byId<HTMLInputElement>('showFiles');
const namingEl = byId<HTMLDetailsElement>('namingRules');
const structureEl = byId<HTMLDetailsElement>('structureRules');
const namingAllEl = byId<HTMLInputElement>('namingAll');
const structureAllEl = byId<HTMLInputElement>('structureAll');

// Each category pairs its picker with an on/off switch, so the whole category can
// be checked or cleared in one click without opening the list.
const categories = [
  [namingEl, namingAllEl],
  [structureEl, structureAllEl],
] as const;
const issuesEl = byId<HTMLInputElement>('issuesOnly');
const summaryEl = byId<HTMLElement>('summary');
const legendEl = byId<HTMLElement>('legend');
const pickEl = byId<HTMLButtonElement>('pick');
const errorEl = byId<HTMLParagraphElement>('error');
const loadingEl = byId<HTMLParagraphElement>('loading');
const darkEl = byId<HTMLInputElement>('darkMode');

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

let model: Payload | null = null;
let root = '';
function ruleBoxes(el: HTMLDetailsElement): HTMLInputElement[] {
  return [...el.querySelectorAll<HTMLInputElement>('input[type=checkbox]')];
}

function checkedRules(el: HTMLDetailsElement): string[] {
  return ruleBoxes(el)
    .filter((box) => box.checked)
    .map((box) => box.value);
}

// Nothing picked switches a category off; everything picked leaves the tree
// whole; a partial pick also narrows the tree to the rules that are on.
function categoryOn(el: HTMLDetailsElement): boolean {
  return checkedRules(el).length > 0;
}

function partialRules(): Set<string> | null {
  const picked = [...checkedRules(namingEl), ...checkedRules(structureEl)];
  const total = ruleBoxes(namingEl).length + ruleBoxes(structureEl).length;
  return picked.length > 0 && picked.length < total ? new Set(picked) : null;
}

function renderRuleTriggers(): void {
  for (const [el, master] of categories) {
    const boxes = ruleBoxes(el);
    const on = boxes.filter((box) => box.checked).length;
    const state = el.querySelector('.rules__state');
    if (state) {
      state.textContent = on === 0 ? 'off' : on === boxes.length ? 'all' : `${on}/${boxes.length}`;
    }
    master.checked = on > 0;
    master.indeterminate = on > 0 && on < boxes.length;
  }
}

// True when the node or any descendant reports one of those rules, so a rule
// filter keeps the path down to every match open.
function hasRule(node: TreeNode, rules: Set<string>): boolean {
  return (
    node.issues.some((issue) => rules.has(issue.rule)) ||
    node.children.some((child) => hasRule(child, rules))
  );
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

/* One "! n" marker plus a tooltip listing the Allar rules that fired. */
function issueMarker(issues: Issue[], total: number): HTMLElement {
  const marker = document.createElement('span');
  marker.className = 'flag';
  marker.textContent = `! ${total}`;
  const lines = issues.map((issue) => `${issue.rule}  ${issue.message}`);
  const nested = total - issues.length;
  if (nested > 0) lines.push(`${nested} more in this folder's contents`);
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
  chip.title = `${file.typeName} (${file.typeLabel} prefix)`;
  const name = document.createElement('span');
  name.className = 'name name--leaf';
  name.textContent = file.name;
  const size = document.createElement('span');
  size.className = 'size';
  size.textContent = formatBytes(file.size);

  row.append(guides(depth), spacer, name, size);
  if (file.issues.length > 0) row.append(issueMarker(file.issues, file.issues.length));
  // appended last so the type lines up in its own right-hand column
  row.append(chip);
  row.title = `${file.name}\n${file.typeName}\n${formatBytes(file.size)}${file.asset ? ' - asset' : ' - not an asset'}`;
  li.append(row);
  return li;
}

function renderDir(node: TreeNode, depth: number): HTMLLIElement {
  const li = document.createElement('li');
  li.className = 'node';
  li.dataset.name = node.name.toLowerCase();
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
  if (node.violations > 0) row.append(issueMarker(node.issues, node.violations));
  row.title = `${node.name}/\n${plural(node.assets, 'asset')}, ${plural(node.total, 'file')}, ${formatBytes(node.bytes)}`;

  const children = document.createElement('ul');
  children.className = 'children';
  for (const child of node.children) children.append(renderDir(child, depth + 1));
  for (const file of files) children.append(renderFile(file, depth + 1));

  li.append(row, children);
  return li;
}

function render(): void {
  if (!model) return;
  treeEl.replaceChildren(renderDir(model.tree, 0));
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
   Naming and structure get a line each, so one category cannot hide the other. */
function renderSummary(): void {
  if (!model) return;
  summaryEl.hidden = false;
  summaryEl.replaceChildren();

  if (!model.lintApplied) {
    summaryEl.append(
      summaryLine('Allar checks off - no Unreal project detected in this folder.', true),
    );
    return;
  }

  for (const [category, enabled] of [
    ['naming', categoryOn(namingEl)],
    ['structure', categoryOn(structureEl)],
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
    summaryEl.append(summaryLine(`${category} checks: ${body}`, !enabled || total === 0));
  }
}

function summaryLine(text: string, clean: boolean): HTMLParagraphElement {
  const line = document.createElement('p');
  line.className = clean ? 'summary summary--clean' : 'summary';
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
function filterNode(li: Element, node: TreeNode, query: string): boolean {
  const rules = partialRules();
  const only = issuesEl.checked || rules !== null;
  const matches = (name: string): boolean => query.length > 0 && name.toLowerCase().includes(query);
  let keep =
    matches(node.name) || (only && (rules === null ? node.violations > 0 : hasRule(node, rules)));

  const childLis = [...(li.querySelector('.children')?.children ?? [])];
  let index = 0;
  for (const child of node.children) {
    const childLi = childLis[index++];
    if (childLi && filterNode(childLi, child, query)) keep = true;
  }
  for (const file of visibleFiles(node)) {
    const fileLi = childLis[index++];
    if (!fileLi) continue;
    const hit = matches(file.name) || (only && file.issues.length > 0);
    fileLi.classList.toggle('is-hidden', !hit);
    if (hit) keep = true;
  }

  li.classList.toggle('is-hidden', !keep);
  if (keep && (query.length > 0 || only) && node.children.length) li.classList.add('is-open');
  return keep;
}

function applyFilter(): void {
  const query = filterEl.value.trim().toLowerCase();
  for (const li of treeEl.querySelectorAll('li.node')) li.classList.remove('is-hidden');
  const first = treeEl.firstElementChild;
  if (model && first && (query.length > 0 || issuesEl.checked || partialRules() !== null)) {
    filterNode(first, model.tree, query);
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
  model = await invoke<Payload>('scan_directory', {
    path,
    naming: categoryOn(namingEl),
    structure: categoryOn(structureEl),
  });
  root = model.root;
  rootEl.textContent = model.root;
  render();
}

async function load(): Promise<void> {
  errorEl.hidden = true;
  if (!invoke) {
    loadingEl.hidden = true;
    fail('The Tauri runtime is not available (window.__TAURI__ is missing).');
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

async function pick(): Promise<void> {
  errorEl.hidden = true;
  if (!invoke) return;
  try {
    const selected = await invoke<string | null>('plugin:dialog|open', {
      options: { directory: true, multiple: false, title: 'Choose a folder to scan' },
    });
    if (selected) {
      root = selected;
      localStorage.setItem(storageKey, selected);
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
// Switching a whole category off changes which checks run, so that needs a
// re-scan; narrowing within a live category only re-filters what is shown.
let namingWasOn = true;
let structureWasOn = true;

function applyRulesChange(): void {
  renderRuleTriggers();
  const namingNow = categoryOn(namingEl);
  const structureNow = categoryOn(structureEl);
  if (namingNow === namingWasOn && structureNow === structureWasOn) {
    applyFilter();
    return;
  }
  namingWasOn = namingNow;
  structureWasOn = structureNow;
  void load();
}

for (const [el, master] of categories) {
  master.addEventListener('change', () => {
    for (const box of ruleBoxes(el)) box.checked = master.checked;
    applyRulesChange();
  });
  el.addEventListener('change', applyRulesChange);
  el.addEventListener('toggle', () => {
    if (!el.open) return;
    for (const [other] of categories) {
      if (other !== el) other.open = false;
    }
  });
}

void load();

export {};
