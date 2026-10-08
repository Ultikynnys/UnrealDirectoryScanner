'use strict';

/* Renders the tree that the Rust `scan_directory` command returns. The folder is
   scanned on every call, so nothing here is baked in: "Refresh" re-scans it. */

const tauri = window.__TAURI__;
const invoke = tauri ? tauri.core.invoke : null;

const treeEl = document.getElementById('tree');
const rootEl = document.getElementById('root');
const statsEl = document.getElementById('stats');
const filterEl = document.getElementById('filter');
const filesEl = document.getElementById('showFiles');
const pickEl = document.getElementById('pick');
const errorEl = document.getElementById('error');
const loadingEl = document.getElementById('loading');

const openBelowDepth = 1; // start with the top two levels unfolded
const storageKey = 'unrealDirectoryScanner.path';

let model = null;
let root = '';

function formatBytes(bytes) {
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

function plural(n, word) {
  return `${n.toLocaleString()} ${word}${n === 1 ? '' : 's'}`;
}

function guides(depth) {
  const out = document.createDocumentFragment();
  for (let i = 0; i < depth; i += 1) {
    const span = document.createElement('span');
    span.className = 'guide';
    out.append(span);
  }
  return out;
}

function renderFile(file) {
  const li = document.createElement('li');
  li.className = 'node';
  li.dataset.name = file.name.toLowerCase();

  const row = document.createElement('div');
  row.className = 'row row--leaf';

  const spacer = document.createElement('span');
  spacer.className = 'caret caret--empty';
  const name = document.createElement('span');
  name.className = 'name name--leaf';
  name.textContent = file.name;
  const size = document.createElement('span');
  size.className = 'size';
  size.textContent = formatBytes(file.size);

  row.append(spacer, name, size);
  row.title = `${file.name}\n${formatBytes(file.size)}${file.asset ? ' - asset' : ' - not an asset'}`;
  li.append(row);
  return li;
}

function renderDir(node, depth) {
  const li = document.createElement('li');
  li.className = 'node';
  li.dataset.name = node.name.toLowerCase();
  if (depth <= openBelowDepth) li.classList.add('is-open');

  const expandable = node.children.length > 0 || node.files.length > 0;
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
  row.title = `${node.name}/\n${plural(node.assets, 'asset')}, ${plural(node.total, 'file')}, ${formatBytes(node.bytes)}`;

  const children = document.createElement('ul');
  children.className = 'children';
  for (const child of node.children) children.append(renderDir(child, depth + 1));
  if (filesEl.checked) for (const file of node.files) children.append(renderFile(file));

  li.append(row, children);
  return li;
}

function render() {
  treeEl.replaceChildren(renderDir(model.tree, 0));
  statsEl.textContent = `${plural(model.tree.assets, 'asset')}, ${plural(
    model.tree.total,
    'file',
  )}, ${formatBytes(model.tree.bytes)} - scanned ${new Date(model.scannedAt).toLocaleTimeString()}`;
  applyFilter();
}

/* Filtering walks the data, then hides DOM rows, so a folder survives when any
   descendant matches and the path down to it stays open. */
function filterNode(li, node, query) {
  const self = node.name.toLowerCase().includes(query);
  let keep = self;

  const childLis = [...li.querySelector('.children')?.children ?? []];
  let index = 0;
  for (const child of node.children) {
    const childLi = childLis[index++];
    if (filterNode(childLi, child, query)) keep = true;
  }
  if (filesEl.checked) {
    for (const file of node.files) {
      const fileLi = childLis[index++];
      const hit = file.name.toLowerCase().includes(query);
      fileLi.classList.toggle('is-hidden', !hit);
      if (hit) keep = true;
    }
  }

  li.classList.toggle('is-hidden', !keep);
  if (keep && query && node.children.length) li.classList.add('is-open');
  return keep;
}

function applyFilter() {
  const query = filterEl.value.trim().toLowerCase();
  for (const li of treeEl.querySelectorAll('li.node')) li.classList.remove('is-hidden');
  if (query) filterNode(treeEl.firstElementChild, model.tree, query);
}

function setOpen(open) {
  for (const li of treeEl.querySelectorAll('li.node')) {
    if (li.querySelector('.children')) li.classList.toggle('is-open', open);
  }
}

function fail(message) {
  errorEl.hidden = false;
  errorEl.textContent = message;
}

async function scan(path) {
  model = await invoke('scan_directory', { path });
  root = model.root;
  rootEl.textContent = model.root;
  render();
}

async function load() {
  errorEl.hidden = true;
  if (!invoke) {
    loadingEl.hidden = true;
    fail('The Tauri runtime is not available (window.__TAURI__ is missing).');
    return;
  }
  try {
    if (!root) {
      const explicit = await invoke('startup_directory');
      root = explicit || localStorage.getItem(storageKey) || (await invoke('default_directory'));
    }
    await scan(root);
    loadingEl.hidden = true;
  } catch (error) {
    loadingEl.hidden = true;
    fail(`Could not scan ${root || 'the folder'}: ${error}`);
  }
}

async function pick() {
  errorEl.hidden = true;
  try {
    const selected = await invoke('plugin:dialog|open', {
      options: { directory: true, multiple: false, title: 'Choose a folder to scan' },
    });
    if (typeof selected === 'string' && selected.length > 0) {
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
  const row = event.target.closest('.row--dir');
  if (!row) return;
  const li = row.closest('.node');
  if (li.querySelector('.children')) li.classList.toggle('is-open');
});

filterEl.addEventListener('input', applyFilter);
document.getElementById('expand').addEventListener('click', () => setOpen(true));
document.getElementById('collapse').addEventListener('click', () => setOpen(false));
document.getElementById('refresh').addEventListener('click', load);
pickEl.addEventListener('click', pick);
filesEl.addEventListener('change', () => model && render());

load();
