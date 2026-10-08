// Copies the static frontend files into dist/, which is what Tauri embeds.
// tsc writes dist/app.js; the HTML and CSS are not TypeScript, so copy them here
// rather than pointing frontendDist at src/ (which would ship the .ts sources).

import { copyFile, mkdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const staticFiles = ['index.html', 'style.css'];

await mkdir(join(root, 'dist'), { recursive: true });
for (const file of staticFiles) {
  await copyFile(join(root, 'src', file), join(root, 'dist', file));
}
