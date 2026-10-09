/* A stand-in for the Rust backend, so the UI can run in a plain browser - and be
   driven from Playwright - with no Tauri process and no access to the file system. It
   is switched on by the query string, `index.html?test`, and is never reached by the
   desktop build, where window.__TAURI__ exists.

   What is shared and what is copied: the rules and their categories are read back out
   of index.html's own checkboxes, so a rule id lives in exactly one place. Only the
   guide presets and the demo project are written out here; RULES and GUIDES in
   src-tauri/src/main.rs stay authoritative.

   Only the *selection* of rules is honoured. The demo reports a fixed set of issues per
   item and filters them by whichever rules are on, so this is a fixture for the UI, not
   a second linter: it cannot decide that a name is wrong, only show what a wrong name
   looks like. */

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

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

interface Issue {
  rule: string;
  message: string;
  category: string;
}

interface Violation extends Issue {
  path: string;
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

interface Node {
  name: string;
  isDir: boolean;
  files: FileEntry[];
  children: Node[];
  assets: number;
  total: number;
  bytes: number;
  issues: Issue[];
  violations: number;
  namingViolations: number;
  structureViolations: number;
}

interface Payload {
  root: string;
  scannedAt: number;
  tree: Node;
  violations: Violation[];
  lintApplied: boolean;
  looksUnreal: boolean;
}

const DEMO_PATH = 'C:\\Dev\\GenericShooter\\Content';

// The expansion of the presets in src-tauri/src/main.rs, where each one is a combination
// of flags, so the demo has a picker at all.
const GUIDES: CatalogGuide[] = [
  {
    id: 'allar',
    label: 'Allar / Gamemakin',
    url: 'https://github.com/Allar/ue5-style-guide',
    rules: ['00.1', '00.3', '1.1', '2.1.2', '2.1.3', '2.2.1', '2.4', '2.6.1', '2.6.2', '2.8', '2.9'],
    custom: false,
  },
  {
    id: 'community',
    label: 'Community',
    url: 'https://dev.epicgames.com/community/learning/tutorials/mX6b/unreal-engine-project-structure-naming-conventions',
    rules: [
      '00.1', '00.2', '00.3', '1.1', '2.1.2', '2.1.3', '2.1.4', '2.2.1', '2.4', '2.6.1',
      '2.6.2', '2.6.3', '2.8', '2.9',
    ],
    custom: false,
  },
  {
    id: 'minimal',
    label: 'Minimal',
    url: 'https://github.com/Allar/ue5-style-guide',
    rules: ['00.1', '00.3', '1.1', '2.1.2', '2.1.3', '2.2.1', '2.4', '2.9'],
    custom: false,
  },
  {
    id: 'unrealdirective',
    label: 'Unreal Directive',
    url: 'https://unrealdirective.com/resources/project-standards/folder-structure/',
    rules: ['00.1', '00.3', '1.1', '2.1.2', '2.1.3', '2.2.1', '2.6.1', '2.9'],
    custom: false,
  },
  {
    id: 'lyra',
    label: 'Lyra',
    url: 'https://dev.epicgames.com/community/learning/paths/Z4/lyra-starter-game',
    rules: ['00.1', '00.3', '1.1', '2.1.2', '2.1.3', '2.4', '2.6.1', '2.6.2', '2.9'],
    custom: false,
  },
  {
    id: 'bytype',
    label: 'By asset type',
    url: 'https://unrealdirective.com/resources/project-standards/folder-structure/',
    rules: ['00.1', '00.3', '1.1', '2.1.2', '2.1.3', '2.2.1', '2.4', '2.6.1', '2.8', '2.9'],
    custom: false,
  },
];

// Just the prefixes the demo uses. The full table is ASSET_KINDS in main.rs.
const KINDS: [string, string, string, string][] = [
  ['BP_', 'blueprint', 'BP', 'Blueprint'],
  ['SK_', 'mesh', 'SK', 'Skeletal Mesh'],
  ['MI_', 'material', 'MI', 'Material Instance'],
  ['M_', 'material', 'M', 'Material'],
  ['T_', 'texture', 'T', 'Texture'],
];

type Pair = [rule: string, message: string];

interface SeedFile {
  name: string;
  issues?: Pair[];
}

interface SeedDir {
  name: string;
  own?: Pair[];
  files?: SeedFile[];
  children?: SeedDir[];
}

/* One demo project that trips a spread of rules, so every part of the UI has something
   to draw: both summary lines, inline markers, type chips and the legend. A couple of
   these - "weapons" against "Weapons", and CON.uasset - cannot exist on a real Windows
   disk, which is exactly why the demo is worth having. */
const DEMO: SeedDir = {
  name: 'Content',
  files: [
    {
      name: 'Orphan.uasset',
      issues: [['2.2.1', '"Orphan.uasset" is a global asset; project assets belong in Content/<Project>.']],
    },
  ],
  children: [
    { name: 'Maps', files: [{ name: 'Arena.umap' }] },
    { name: 'Core', files: [{ name: 'BP_GameMode.uasset' }] },
    {
      name: 'Characters',
      files: [
        { name: 'SK_Hero.uasset' },
        { name: 'MI_Hero.uasset' },
        { name: 'T_Hero_Body_D.uasset' },
      ],
    },
    // correct placement: a base material inside MaterialLibrary, so 2.8 stays quiet
    { name: 'MaterialLibrary', files: [{ name: 'M_Master.uasset' }] },
    {
      name: 'Assets',
      own: [['2.6.1', '"Assets" is a redundant type folder; all assets are assets.']],
      files: [{ name: 'BP_Thing.uasset' }],
      // a folder inside a flagged one, so that "issues only" has to keep a whole branch with it
      children: [{ name: 'Legacy', files: [{ name: 'SK_Old.uasset' }] }],
    },
    {
      name: 'Blueprints',
      own: [['2.6.3', '"Blueprints" is a type folder; asset name prefixes already convey the type.']],
      files: [{ name: 'BP_Pistol.uasset' }],
    },
    {
      name: 'Loose',
      files: [
        {
          name: 'M_Loose.uasset',
          issues: [['2.8', '"Loose/M_Loose.uasset" is a base material outside MaterialLibrary.']],
        },
        { name: 'OldMap.umap', issues: [['2.4', '"Loose/OldMap.umap" is a map outside a Maps folder.']] },
        {
          name: 'BP_lowercase.uasset',
          issues: [['1.1', '"Loose/BP_lowercase.uasset" has "lowercase", which is not PascalCase.']],
        },
        {
          name: 'Bad Name.uasset',
          issues: [
            ['00.1', '"Loose/Bad Name.uasset" contains \' \'; only letters, digits and underscore are allowed.'],
          ],
        },
        {
          name: 'CON.uasset',
          issues: [['00.2', '"Loose/CON.uasset" is a reserved Windows device name, which Windows refuses to create.']],
        },
        { name: 'notes.txt' },
      ],
    },
    {
      name: 'weapons',
      own: [
        ['00.3', '"weapons" is not PascalCase.'],
        ['2.1.4', '"weapons" collides with sibling "Weapons"; folder names must differ by more than case.'],
      ],
      files: [{ name: 'BP_Revolver.uasset' }],
    },
    { name: 'Weapons', own: [['2.9', '"Weapons" is an empty folder.']] },
    // engine-generated: exempt in the real backend, so it is quiet here too
    { name: '__ExternalActors__', files: [{ name: 'Actor.uasset' }] },
  ],
};

let cached: Catalog | null = null;
let cachedCategories: Map<string, string> | null = null;

/* The presets the user made, standing in for the files beside the executable: the demo has no
   file system, so they live here for as long as the page does. */
const userPresets: CatalogGuide[] = [];

function catalogNow(): Catalog {
  const catalog = cached ?? catalogFromMarkup();
  return { categories: catalog.categories, guides: [...GUIDES, ...userPresets] };
}

// The rule ids and categories come from the markup, never from a second list here.
function catalogFromMarkup(): Catalog {
  const categories: CatalogCategory[] = (['naming', 'structure'] as const).map((id) => ({
    id,
    rules: [
      ...document.querySelectorAll<HTMLInputElement>(`#${id}Rules .rules__menu input[type=checkbox]`),
    ].map((box) => ({
      id: box.value,
      label: (box.closest('label')?.textContent ?? '').trim().replace(/^\S+\s*/, ''),
    })),
  }));
  return { categories, guides: GUIDES };
}

function categoryOf(rule: string): string {
  if (!cachedCategories) {
    cachedCategories = new Map();
    for (const category of (cached ?? catalogFromMarkup()).categories) {
      for (const entry of category.rules) cachedCategories.set(entry.id, category.id);
    }
  }
  return cachedCategories.get(rule) ?? 'naming';
}

function issue([rule, message]: Pair): Issue {
  return { rule, message, category: categoryOf(rule) };
}

// `null` means the default guide, which is what an untouched window sends.
function enabled(rules: string[] | null): (rule: string) => boolean {
  if (rules === null) return (rule) => GUIDES[0].rules.includes(rule);
  return (rule) => rules.includes(rule);
}

function kindOf(name: string): [string, string, string] {
  const asset = /\.(uasset|umap)$/i.test(name);
  const base = name.replace(/\.[^.]*$/, '');
  const ext = name.includes('.') ? name.slice(name.lastIndexOf('.') + 1) : '';
  if (asset && ext.toLowerCase() === 'umap') return ['level', 'MAP', 'Level / Map'];
  for (const [prefix, family, label, typeName] of KINDS) {
    if (base.startsWith(prefix)) return [family, label, typeName];
  }
  if (!asset) {
    return ext ? ['file', ext.toUpperCase(), `${ext.toUpperCase()} file`] : ['file', 'FILE', 'File'];
  }
  return ['other', '?', 'Unrecognised prefix'];
}

function buildTree(seed: SeedDir, prefix: string, on: (rule: string) => boolean, out: Violation[]): Node {
  const rel = prefix ? `${prefix}/${seed.name}` : seed.name;
  const files: FileEntry[] = (seed.files ?? []).map((file) => {
    const path = `${rel}/${file.name}`;
    const issues = (file.issues ?? []).filter(([rule]) => on(rule)).map(issue);
    for (const one of issues) out.push({ ...one, path });
    const [typeFamily, typeLabel, typeName] = kindOf(file.name);
    return {
      name: file.name,
      size: 1024 + ((file.name.length * 613) % 400000),
      asset: /\.(uasset|umap)$/i.test(file.name),
      issues,
      typeFamily,
      typeLabel,
      typeName,
    };
  });
  // the scanned root's own name is the user's choice, exactly as in the real checks
  const own = (prefix ? (seed.own ?? []) : []).filter(([rule]) => on(rule)).map(issue);
  for (const one of own) out.push({ ...one, path: rel });

  const children = (seed.children ?? []).map((child) => buildTree(child, rel, on, out));
  const mine = [...own, ...files.flatMap((file) => file.issues)];
  const ownViolations = mine.length;
  const ownNaming = mine.filter((one) => one.category === 'naming').length;
  const nested = children.reduce((sum, child) => sum + child.violations, 0);
  const nestedNaming = children.reduce((sum, child) => sum + child.namingViolations, 0);

  return {
    name: seed.name,
    isDir: true,
    files,
    children,
    assets: files.filter((file) => file.asset).length + children.reduce((sum, c) => sum + c.assets, 0),
    total: files.length + children.reduce((sum, c) => sum + c.total, 0),
    bytes: files.reduce((sum, file) => sum + file.size, 0) + children.reduce((sum, c) => sum + c.bytes, 0),
    issues: own,
    violations: ownViolations + nested,
    namingViolations: ownNaming + nestedNaming,
    structureViolations: ownViolations - ownNaming + (nested - nestedNaming),
  };
}

function demoPayload(path: string, rules: string[] | null): Payload {
  const violations: Violation[] = [];
  const tree = buildTree(DEMO, '', enabled(rules), violations);
  return {
    root: path,
    scannedAt: Date.now(),
    tree,
    violations,
    lintApplied: rules === null || rules.length > 0,
    looksUnreal: true,
  };
}

/// The demo backend when the URL asks for it with `?test`, otherwise `null` so the
/// caller keeps reporting a missing Tauri runtime.
export function testInvoke(): Invoke | null {
  if (!new URLSearchParams(location.search).has('test')) return null;
  cached = catalogFromMarkup();
  document.title = `${document.title} - test mode`;
  console.info('Unreal Directory Scanner: test mode, serving a demo project');

  return async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
    switch (command) {
      case 'rule_catalog':
        return catalogNow() as T;
      case 'save_preset': {
        // the real backend writes a file; the demo only has to keep the row on screen
        const name = String(args?.name ?? '').trim();
        if (!name) throw new Error('a preset needs a name');
        const flags = (args?.flags as string[] | undefined) ?? [];
        const saved: CatalogGuide = { id: name, label: name, url: null, rules: [...flags], custom: true };
        const at = userPresets.findIndex((preset) => preset.id === name);
        if (at === -1) userPresets.push(saved);
        else userPresets[at] = saved;
        return undefined as T;
      }
      case 'delete_preset': {
        const at = userPresets.findIndex((preset) => preset.id === String(args?.name ?? ''));
        if (at !== -1) userPresets.splice(at, 1);
        return undefined as T;
      }
      case 'default_directory':
        return DEMO_PATH as T;
      case 'startup_directory':
        return null as T;
      case 'scan_directory': {
        const path = typeof args?.path === 'string' ? args.path : DEMO_PATH;
        const rules = (args?.rules as string[] | null | undefined) ?? null;
        return demoPayload(path, rules) as T;
      }
      case 'open_docs':
        // nothing to open off a web page: the demo only has to not throw
        return undefined as T;
      default:
        throw new Error(`test mode has no answer for "${command}"`);
    }
  };
}
